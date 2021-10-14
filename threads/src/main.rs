use core::{mem, ops};

use semilattice::{Map, Max, Redactable, SemiLattice, Set};

/// An actor ID. Probably a public key.
pub type AID = minicbor::bytes::ByteArray<32>;

/// A Message ID. An actor ID paired with a supposedly unique number. The actor
/// is responsible for choosing a unique number.
pub type MessageID = (AID, u64);

pub type Reaction = String;

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
pub struct Node {
    #[n(0)]
    comment: Comment,
    #[n(1)]
    reactions: Map<Reaction, Vote<2>>,
    // back references
    #[n(2)]
    responses: Set<MessageID>,
}

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Slice {
    #[n(0)]
    comments: Map<u64, Comment>,
    #[n(1)]
    reactions: Map<MessageID, Map<Reaction, Max<u64>>>,
}

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
struct NamedSlice(#[n(0)] AID, #[n(1)] Slice);

#[derive(Default, Debug, Clone, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
pub struct Thread {
    #[n(0)]
    graph: Map<AID, Map<u64, Node>>,
}

impl Thread {
    // Inefficient implementation; but is correct by construction without
    // validation steps. Assumes only that NamedSlice is well formed.
    fn absorb_slice(&mut self, NamedSlice(aid, slice): NamedSlice) {
        use std::collections::BTreeMap;

        for (k, v) in BTreeMap::from(slice.comments) {
            // record back-references for responses
            for r in v.reply_to.iter() {
                self.graph.entry(r.0).entry(r.1).responses.insert((aid, k));
            }
            // record any versions of the comment
            self.graph.entry(aid).entry(k).comment.join_assign(v);
        }

        // apply all reactions
        for (k, v) in BTreeMap::from(slice.reactions) {
            for (r, v) in BTreeMap::from(v) {
                self.graph.entry(k.0).entry(k.1).reactions.entry(r).entry(aid).join_assign(v);
            }
        }
    }
}

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
pub struct Actor {
    #[n(0)]
    id: AID,
    #[n(1)]
    device_id: u64,
    #[n(2)]
    counter: u64,
    #[n(3)]
    slice: Slice,
}

impl Actor {
    fn new(id: AID, device_id: u64) -> Self {
        Self {
            id,
            device_id,
            counter: 0,
            slice: Default::default(),
        }
    }

    fn extract_slice(&mut self) -> NamedSlice {
        NamedSlice(self.id, mem::take(&mut self.slice))
    }

    fn new_thread(&mut self, message: String) -> MessageID {
        let id = (self.id, (self.counter << 16) + self.device_id);
        self.counter += 1;

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

        let comment = self.slice.comments.entry(id.1);

        comment.reply_to.insert(parent);
        comment.content.entry(0).join_assign(Redactable::Data(message));

        id
    }

    fn edit(&mut self, id: MessageID, message: String) -> MessageID {
        let content = &mut self.slice.comments.entry(id.1).content;
        let version: u64 = content.len().try_into().unwrap();

        content.entry((version << 16) + self.device_id).join_assign(Redactable::Data(message));

        id
    }

    fn redact(&mut self, id: MessageID, version: u64) {
        self.slice.comments.entry(id.1).content.entry(version).join_assign(Redactable::Redacted);
    }

    fn react(&mut self, id: MessageID, reaction: Reaction, vote: u64) {
        self.slice.reactions.entry(id).entry(reaction).join_assign(Max(vote));
    }
}

fn main() {
    // Alice has multiple devices
    let mut alice_0 = Actor::new(AID::from(*b"Alice_6789abcdef0123456789abcdef"), 0);
    let mut alice_1 = Actor::new(AID::from(*b"Alice_6789abcdef0123456789abcdef"), 1);

    // Bob has one
    let mut bob = Actor::new(AID::from(*b"Bob_456789abcdef0123456789abcdef"), 0);

    // Alice creates a new issue from her laptop
    let a0 = alice_0.new_thread("Hello world. I have this issue [..]".to_owned());
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

    // CBOR encode each slice

    let alice_0_slice = alice_0.extract_slice();
    let alice_1_slice = alice_1.extract_slice();
    let bob_slice = bob.extract_slice();

    let mut buffer = Vec::new();
    minicbor::encode(&alice_0_slice, &mut buffer).expect("Failed to encode Alice#0' slice to CBOR.");
    eprintln!("Alice#0: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&alice_1_slice, &mut buffer).expect("Failed to encode Alice#1' slice to CBOR.");
    eprintln!("Alice#1: {}", minicbor::display(&buffer));

    let alice_combined_slice = NamedSlice(alice_0_slice.0, alice_0_slice.1.join(alice_1_slice.1));

    buffer.clear();
    minicbor::encode(&alice_combined_slice, &mut buffer).expect("Failed to encode Alice' slice to CBOR.");
    eprintln!("Alice: {}", minicbor::display(&buffer));

    buffer.clear();
    minicbor::encode(&bob_slice, &mut buffer).expect("Failed to encode Bob's slice to CBOR");
    eprintln!("Bob: {}", minicbor::display(&buffer));

    // Aggregate slices into a thread. CBOR encode the materialized view.

    let mut thread = Thread::default();
    thread.absorb_slice(alice_combined_slice);
    thread.absorb_slice(bob_slice);

    buffer.clear();
    minicbor::encode(&thread, &mut buffer)
        .expect("Failed to encode the materialized view to CBOR.");
    eprintln!("Materialized: {}", minicbor::display(&buffer));
}
