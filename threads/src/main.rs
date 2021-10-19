#![feature(map_first_last)]

use core::ops;
use std::collections::BTreeMap;

use semilattice::{GuardedPair, Map, Max, Redactable, SemiLattice, Set};

pub mod detailed;

#[derive(Debug)]
pub enum Error {
    Impersonation,
}

pub type Result<T = (), E = Error> = core::result::Result<T, E>;

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

/// A Message ID. An actor ID paired with a supposedly unique number. The actor
/// is responsible for choosing a unique number.
pub type MessageID = (ActorID, u64);

pub type Reaction = String;
pub type Tag = String;

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
#[cbor(transparent)]
pub struct Vote<const N: usize>(#[n(0)] Map<ActorID, Max<u64>>);

impl<const N: usize> ops::Deref for Vote<N> {
    type Target = Map<ActorID, Max<u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> ops::DerefMut for Vote<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> Vote<N> {
    pub fn aggregate(&self) -> [usize; N] {
        let mut res = [0; N];

        for v in self.values() {
            // modulo for arbitrary N isn't efficient, but if N is always a
            // power of two, this becomes a bit-mask. Any excess values could
            // be reserved or may be considered equivalent to the highest
            // element.
            res[v.0 as usize % N] += 1;
        }

        res
    }
}

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
pub struct Actor {
    id: ActorID,
    device_id: u64,
    counter: u64,
    slice: Slice,
}

impl Actor {
    fn new(id: ActorID, device_id: u64) -> Self {
        Self {
            id,
            device_id,
            counter: 0,
            slice: Default::default(),
        }
    }

    fn new_thread(
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

    fn reply(&mut self, parent: MessageID, message: String) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

        self.slice.owned.entry(id.1).join_assign(Owned {
            titles: Default::default(),
            reply_to: Set::singleton(parent),
            content: Map::singleton(0, Redactable::Data(message)),
        });

        id
    }

    /// Fails if you attempt to edit someone else' message.
    fn edit(&mut self, id: MessageID, message: String) -> Result<u64> {
        if self.id != id.0 {
            return Err(Error::Impersonation);
        }

        let content = &mut self.slice.owned.entry(id.1).content;

        // One greater than the latest version we have observed.
        let version: u64 = content.last_key_value().map(|x| x.0 + 1).unwrap_or(0);

        content
            .entry((version << 16) + self.device_id)
            .join_assign(Redactable::Data(message));

        Ok(version)
    }

    /// Fails if you attempt to redact someone else' message.
    fn redact(&mut self, id: MessageID, version: u64) -> Result {
        if self.id != id.0 {
            return Err(Error::Impersonation);
        }

        self.slice
            .owned
            .entry(id.1)
            .content
            .entry(version)
            .join_assign(Redactable::Redacted);

        Ok(())
    }

    fn react(&mut self, id: MessageID, reaction: Reaction, vote: u64) {
        self.slice
            .shared
            .entry(id)
            .reactions
            .entry(reaction)
            .join_assign(Max(vote));
    }

    fn adjust_tags(
        &mut self,
        id: MessageID,
        add: impl IntoIterator<Item = Reaction>,
        remove: impl IntoIterator<Item = Reaction>,
    ) {
        self.slice.shared.entry(id).tags.join_assign(
            add.into_iter()
                .map(|x| (x, Max(1)))
                .chain(remove.into_iter().map(|x| (x, Max(2))))
                .collect::<BTreeMap<_, _>>()
                .into(),
        );
    }
}

fn main() {
    // Alice has multiple devices
    let mut alice_0 = Actor::new(ActorID::Alice, 0);
    let mut alice_1 = Actor::new(ActorID::Alice, 1);

    // Bob has one
    let mut bob = Actor::new(ActorID::Bob, 0);

    // Alice creates a new issue from her laptop
    let a0 = alice_0.new_thread(
        "Issue with feature X".to_owned(),
        "Hello world. I have this issue [..]".to_owned(),
        ["bug".to_owned(), "incorrect-tag".to_owned()],
    );

    // Bob responds and adjusts the tags for the thread
    let b0 = bob.reply(a0, "Huh. Can you run the tests?".to_owned());
    bob.adjust_tags(a0, ["regression".to_owned()], ["incorrect-tag".to_owned()]);

    // Alice reacts form her phone
    let _a1 = alice_1.react(b0, ":hourglass:".to_owned(), 1);

    // responds from her laptop
    let a2 = alice_0.reply(b0, "Ah! Test #3 failed. [..]".to_owned());
    // edits her response from her phone
    let _a2_edit_version = alice_1.edit(a2, "Ah! Test #4 failed. [..]".to_owned());
    // and redacts her first version to hide her typo.
    alice_1.redact(a2, 0).expect("Impersonation?");

    // CBOR encode each dirty slice

    let mut buffer = Vec::new();
    minicbor::encode(&alice_0.slice, &mut buffer).expect("Failed to CBOR encode Alice#0.slice.");
    println!("Alice#0: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&alice_1.slice, &mut buffer).expect("Failed to CBOR encode Alice#1.slice.");
    println!("Alice#1: {}", minicbor::display(&buffer));

    let alice_combined = alice_0.slice.clone().join(alice_1.slice.clone());

    buffer.clear();
    minicbor::encode(&alice_combined, &mut buffer).expect("Failed to CBOR encode Alice.slice.");
    println!("Alice: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&bob.slice, &mut buffer).expect("Failed to CBOR encode Bob.slice.");
    println!("Bob: {}", minicbor::display(&buffer));

    let mut root = Root::default();
    root.entry(ActorID::Alice).join_assign(alice_0.slice);
    root.entry(ActorID::Alice).join_assign(alice_1.slice);
    root.entry(ActorID::Bob).join_assign(bob.slice);

    buffer.clear();
    minicbor::encode(&root, &mut buffer).expect("Failed to CBOR encode root.");
    println!("Materialized: {}", minicbor::display(&buffer));

    println!();

    detailed::Detailed::default().join(root).display();
}
