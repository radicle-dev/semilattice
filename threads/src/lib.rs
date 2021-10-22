#![feature(map_first_last)]

use std::collections::BTreeMap;

use semilattice::{GuardedPair, Map, Max, Redactable, SemiLattice, Set};

pub mod detailed;

/// An actor ID. Probably a public key.
//pub type ActorID = minicbor::bytes::ByteArray<32>;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, minicbor::Encode, minicbor::Decode,
)]
#[cbor(index_only)]
pub enum ActorID {
    #[n(0)]
    Alice,
    #[n(1)]
    Bob,
    #[n(2)]
    Carol,
    #[n(3)]
    Dave,
    #[n(4)]
    Eve,
}

impl core::str::FromStr for ActorID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alice" => Ok(ActorID::Alice),
            "bob" => Ok(ActorID::Bob),
            "carol" => Ok(ActorID::Carol),
            "dave" => Ok(ActorID::Dave),
            "eve" => Ok(ActorID::Eve),
            _ => Err(()),
        }
    }
}

/// A Message ID. An actor ID paired with a supposedly unique number. The actor
/// is responsible for choosing a unique number.
pub type MessageID = (ActorID, u64);

pub type Reaction = String;
pub type Tag = String;

#[derive(Clone, Default, Debug, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Owned {
    #[n(0)]
    titles: GuardedPair<Max<u64>, Set<String>>,
    #[n(1)]
    reply_to: Set<MessageID>,
    #[n(2)]
    content: Map<u64, Redactable<String>>,
}

#[derive(Clone, Default, Debug, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Shared {
    #[n(0)]
    tags: Map<Tag, Max<u64>>,
    #[n(1)]
    reactions: Map<Tag, Max<u64>>,
}

#[derive(Clone, Default, Debug, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Slice {
    #[n(0)]
    owned: Map<u64, Owned>,
    #[n(1)]
    shared: Map<MessageID, Shared>,
}

pub type Root = Map<ActorID, Slice>;

#[derive(Debug)]
pub struct Actor<'a> {
    pub id: ActorID,
    pub device_id: u64,
    pub slice: &'a mut Slice,
    counter: u64,
}

impl Actor<'_> {
    pub fn new(slice: &mut Slice, id: ActorID, device_id: u64) -> Actor {
        Actor {
            id,
            device_id,
            counter: slice.owned.len().try_into().unwrap(),
            slice,
        }
    }

    pub fn new_thread(
        &mut self,
        title: String,
        message: String,
        tags: impl IntoIterator<Item = String>,
    ) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

        self.slice.owned.entry(id.1).join_assign(Owned {
            titles: GuardedPair {
                guard: Max(0),
                value: Set::singleton(title),
            },
            reply_to: Set::default(),
            content: Map::singleton(0, Redactable::Data(message)),
        });

        self.slice.shared.entry(id).tags.join_assign(
            tags.into_iter()
                .map(|x| (x, Max(1)))
                .collect::<BTreeMap<_, _>>()
                .into(),
        );

        id
    }

    pub fn reply(&mut self, parent: MessageID, message: String) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

        self.slice.owned.entry(id.1).join_assign(Owned {
            titles: Default::default(),
            reply_to: Set::singleton(parent),
            content: Map::singleton(0, Redactable::Data(message)),
        });

        id
    }

    pub fn edit(&mut self, id: u64, message: String) -> u64 {
        let content = &mut self.slice.owned.entry(id).content;

        // One greater than the latest version we have observed.
        let version: u64 = content
            .last_key_value()
            .map(|x| (x.0 >> 16) + 1)
            .unwrap_or(0);

        content
            .entry((version << 16) + self.device_id)
            .join_assign(Redactable::Data(message));

        version
    }

    /// Fails if you attempt to redact someone else' message.
    pub fn redact(&mut self, id: u64, version: u64) {
        self.slice
            .owned
            .entry(id)
            .content
            .entry(version)
            .join_assign(Redactable::Redacted);
    }

    pub fn react(&mut self, id: MessageID, reaction: Reaction, vote: bool) {
        let stored_vote = self.slice.shared.entry(id).reactions.entry(reaction);

        if stored_vote.0 % 2 != vote as u64 {
            stored_vote.0 += 1;
        }
    }

    pub fn adjust_tags(
        &mut self,
        id: MessageID,
        add: impl IntoIterator<Item = Reaction>,
        remove: impl IntoIterator<Item = Reaction>,
    ) {
        let tags = &mut self.slice.shared.entry(id).tags;

        for tag in add {
            let vote = tags.entry(tag);
            // 0 = neutral, 1 = positive, 2 = negative, 3 = invalid
            match vote.0 % 4 {
                0 => vote.0 += 1,
                1 => (),
                2 => vote.0 += 3,
                _ => vote.0 += 2,
            }
        }

        for tag in remove {
            let vote = tags.entry(tag);
            match vote.0 % 4 {
                0 => vote.0 += 2,
                1 => vote.0 += 1,
                2 => (),
                _ => vote.0 += 3,
            }
        }
    }
}
