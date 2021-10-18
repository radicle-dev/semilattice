#![feature(map_first_last)]

use std::collections::BTreeMap;

use core::{mem, ops};

use semilattice::{GuardedPair, Map, Max, Redactable, SemiLattice, Set};

/// An actor ID. Probably a public key.
//pub type AID = minicbor::bytes::ByteArray<32>;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, minicbor::Encode, minicbor::Decode,
)]
#[cbor(index_only)]
pub enum AID {
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
pub type MessageID = (AID, u64);

pub type Reaction = String;
pub type Tag = String;

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
#[cbor(transparent)]
pub struct Vote<const N: usize>(#[n(0)] Map<AID, Max<u64>>);

impl<const N: usize> ops::Deref for Vote<N> {
    type Target = Map<AID, Max<u64>>;

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
struct Comment {
    #[n(0)]
    reply_to: Set<MessageID>,
    // Redactable versioned content of a comment.
    #[n(1)]
    content: Map<u64, Redactable<String>>,
}

#[derive(Clone, Default, Debug, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct CommentMeta {
    #[n(0)]
    comment: Comment,
    // odd = true, even = false
    #[n(1)]
    reactions: Map<Reaction, Vote<2>>,
    // back references
    #[n(2)]
    responses: Set<MessageID>,
}

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Thread {
    #[n(0)]
    titles: GuardedPair<Max<u64>, Set<String>>,
    // Modulo 4, 0 = neutral, 1 = positive, 2 = negative, 3 = invalid.  The
    // invalid case is interpreted as neutral and exists purely to replace a
    // modulo operation with a bitwise AND.
    #[n(1)]
    tags: Map<Tag, Vote<4>>,
}

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct ThreadSlice {
    #[n(0)]
    titles: GuardedPair<Max<u64>, Set<String>>,
    // Modulo 4, 0 = neutral, 1 = positive, 2 = negative, 3 = invalid.  The
    // invalid case is interpreted as neutral and exists purely to replace a
    // modulo operation with a bitwise AND.
    #[n(1)]
    tags: Map<Tag, Max<u64>>,
}

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct View {
    #[n(0)]
    threads: Map<AID, Map<u64, Thread>>,
    #[n(1)]
    comments: Map<AID, Map<u64, CommentMeta>>,
}

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
// FIXME: merge the maps over the common keys.
pub struct Slice {
    #[n(0)]
    threads: Map<u64, ThreadSlice>,
    #[n(1)]
    comments: Map<u64, Comment>,
    #[n(2)]
    reactions: Map<MessageID, Map<Reaction, Max<u64>>>,
    #[n(3)]
    tags: Map<MessageID, Map<Tag, Max<u64>>>,
}

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
struct NamedSlice(#[n(0)] AID, #[n(1)] Slice);

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
pub struct Actor {
    // The name of this actor
    #[n(0)]
    id: AID,
    // The unique device ID for this actor
    #[n(1)]
    device_id: u64,
    // The number of comments this device has created.
    #[n(2)]
    counter: u64,
    // Any dirty changes since the last slice extraction.
    #[n(3)]
    slice: Slice,
    // The materialized view from the perspective of this actor.
    #[n(4)]
    view: View,
}

impl Actor {
    fn new(id: AID, device_id: u64) -> Self {
        Self {
            id,
            device_id,
            counter: 0,
            slice: Default::default(),
            view: Default::default(),
        }
    }

    fn extract_slice(&mut self) -> NamedSlice {
        let named_slice = NamedSlice(self.id, mem::take(&mut self.slice));

        self.absorb_slice(named_slice.clone());

        named_slice
    }

    // Inefficient implementation; but is correct by construction without
    // validation steps. Assumes only the named actor ID is correct and
    // authored the slice.
    fn absorb_slice(&mut self, NamedSlice(aid, slice): NamedSlice) {
        for (k, v) in slice.comments.inner {
            // record back-references for responses
            for r in v.reply_to.iter() {
                self.view
                    .comments
                    .entry(r.0)
                    .entry(r.1)
                    .responses
                    .insert((aid, k));
            }
            // record any versions of the comment
            self.view
                .comments
                .entry(aid)
                .entry(k)
                .comment
                .join_assign(v);
        }

        // apply all reactions
        for (k, v) in slice.reactions.inner {
            let reactions = &mut self.view.comments.entry(k.0).entry(k.1).reactions;
            for (r, v) in v.inner {
                reactions.entry(r).entry(aid).join_assign(v);
            }
        }

        // apply all titles
        for (id, thread) in slice.threads.inner {
            self.view
                .threads
                .entry(aid)
                .entry(id)
                .titles
                .join_assign(thread.titles);
        }

        // apply all tags
        for (k, v) in slice.tags.inner {
            let tags = &mut self.view.threads.entry(k.0).entry(k.1).tags;
            for (t, v) in v.inner {
                tags.entry(t).entry(aid).join_assign(v);
            }
        }
    }

    fn new_thread(
        &mut self,
        title: String,
        message: String,
        tags: impl IntoIterator<Item = Tag>,
    ) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

        self.slice.threads.entry(id.1).join_assign(ThreadSlice {
            titles: GuardedPair {
                guard: Max(0),
                value: Set::singleton(title),
            },
            tags: tags
                .into_iter()
                .map(|x| (x, Max(1)))
                .collect::<BTreeMap<_, _>>()
                .into(),
        });

        self.slice
            .comments
            .entry(id.1)
            .content
            .entry(0)
            .join_assign(Redactable::Data(message));

        id
    }

    fn reply(&mut self, parent: MessageID, message: String) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

        self.slice.comments.entry(id.1).join_assign(Comment {
            reply_to: Set::singleton(parent),
            content: Map::singleton(0, Redactable::Data(message)),
        });

        id
    }

    fn edit(&mut self, id: MessageID, message: String) -> u64 {
        assert_eq!(self.id, id.0, "You may only edit your own messages.");

        // One greater than the latest version we have observed.
        let version: u64 = self
            .view
            .comments
            .entry(self.id)
            .entry(id.1)
            .comment
            .content
            .last_key_value()
            .map(|x| x.0 + 1)
            .unwrap_or(0);

        self.slice
            .comments
            .entry(id.1)
            .content
            .entry((version << 16) + self.device_id)
            .join_assign(Redactable::Data(message));

        version
    }

    fn redact(&mut self, id: MessageID, version: u64) {
        assert_eq!(self.id, id.0, "You may only redact your own messages.");

        self.slice
            .comments
            .entry(id.1)
            .content
            .entry(version)
            .join_assign(Redactable::Redacted);
    }

    fn react(&mut self, id: MessageID, reaction: Reaction, vote: u64) {
        self.slice
            .reactions
            .entry(id)
            .entry(reaction)
            .join_assign(Max(vote));
    }
}

fn main() {
    // Alice has multiple devices
    let mut alice_0 = Actor::new(AID::Alice, 0);
    let mut alice_1 = Actor::new(AID::Alice, 1);

    // Bob has one
    let mut bob = Actor::new(AID::Bob, 0);

    // Alice creates a new issue from her laptop
    let a0 = alice_0.new_thread(
        "Issue with feature X".to_owned(),
        "Hello world. I have this issue [..]".to_owned(),
        vec![],
    );
    // Bob responds
    let b0 = bob.reply(a0, "Huh. Can you run the tests?".to_owned());

    // Alice reacts form her phone
    let _a1 = alice_1.react(b0, ":hourglass:".to_owned(), 1);

    // responds from her laptop
    let a2 = alice_0.reply(b0, "Ah! Test #3 failed. [..]".to_owned());
    // edits her response from her phone
    let _a2_edit_version = alice_1.edit(a2, "Ah! Test #4 failed. [..]".to_owned());
    // and redacts her first version to hide her typo.
    alice_1.redact(a2, 0);

    // CBOR encode each dirty slice

    let alice_0_slice = alice_0.extract_slice();
    let alice_1_slice = alice_1.extract_slice();
    let bob_slice = bob.extract_slice();

    let mut buffer = Vec::new();
    minicbor::encode(&alice_0_slice, &mut buffer)
        .expect("Failed to encode Alice#0' slice to CBOR.");
    println!("Alice#0: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&alice_1_slice, &mut buffer)
        .expect("Failed to encode Alice#1' slice to CBOR.");
    println!("Alice#1: {}", minicbor::display(&buffer));

    // Combine the slices from each of Alice' devices
    let alice_combined_slice = NamedSlice(
        alice_0_slice.0,
        alice_0_slice.1.clone().join(alice_1_slice.1.clone()),
    );

    buffer.clear();
    minicbor::encode(&alice_combined_slice, &mut buffer)
        .expect("Failed to encode Alice' slice to CBOR.");
    println!("Alice: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&bob_slice, &mut buffer).expect("Failed to encode Bob's slice to CBOR");
    println!("Bob: {}", minicbor::display(&buffer));

    // Each user may absorb the other's slices.
    alice_0.absorb_slice(alice_1_slice.clone());
    alice_1.absorb_slice(alice_0_slice.clone());
    alice_0.absorb_slice(bob_slice.clone());
    alice_1.absorb_slice(bob_slice);
    bob.absorb_slice(alice_0_slice);
    bob.absorb_slice(alice_1_slice);

    // All actors reach convergent views and have clean slices.
    assert_eq!(alice_0.view, alice_1.view);
    assert_eq!(alice_0.view, bob.view);
    assert_eq!(alice_0.slice, Default::default());
    assert_eq!(alice_1.slice, Default::default());
    assert_eq!(bob.slice, Default::default());

    buffer.clear();
    minicbor::encode(&alice_0.view, &mut buffer)
        .expect("Failed to encode the materialized view to CBOR.");
    println!("Materialized: {}", minicbor::display(&buffer));

    println!();

    // Bob may add tags
    bob.view.threads.entry(a0.0).entry(a0.1).tags.join_assign(
        BTreeMap::from([
            ("bug".to_owned(), Vote(Map::singleton(AID::Bob, Max(1)))),
            (
                "regression".to_owned(),
                Vote(Map::singleton(AID::Bob, Max(1))),
            ),
            (
                "resolved".to_owned(),
                Vote(Map::singleton(AID::Bob, Max(2))),
            ),
        ])
        .into(),
    );

    let View { threads, comments } = bob.view;

    // An awful example UI.
    for (aid, thread) in threads.inner {
        for (id, Thread { titles, tags }) in thread.inner {
            println!("Author: {:?} [{}]", aid, id);
            if titles.value.len() != 1 {
                eprintln!("[NOTE]: There is not exactly one title for this thread.");
            }
            for title in titles.value.inner {
                println!("Title: {}", title);
            }

            let mut tag_votes = BTreeMap::new();
            for (tag, votes) in tags.inner {
                let va = votes.aggregate();
                *tag_votes.entry(tag).or_insert(0) += va[1] as i64 - va[2] as i64;
            }

            print!("Tags: ");
            for (tag, score) in tag_votes {
                print!("{} ({}), ", tag, score);
            }
            println!();
            println!();

            let mut stack = vec![(0, (aid, id))];

            while let Some((depth, (aid, id))) = stack.pop() {
                let comment = comments
                    .inner
                    .get(&aid)
                    .expect("Expected aid")
                    .get(&id)
                    .expect("Expected id.");

                stack.extend(
                    comment
                        .responses
                        .inner
                        .clone()
                        .into_iter()
                        .map(|x| (depth + 1, x)),
                );

                println!("Depth: {}", depth);
                println!("Author: {:?} [{}]", aid, id);
                for (_, content) in &comment.comment.content.inner {
                    println!("Body: {:?}", content);
                }
                println!();
            }
        }
    }
}
