use semilattice::{Map, Max, Redactable, SemiLattice, Set};

/// A local identifier for an actor. This does not need to be globally
/// consistent.
pub type AID = String;

/// A locally unique ID. This does not need to be globally consistent.
// (owner ID, local total order of unique events observed under an AID)
pub type LUID = (AID, usize);

pub type Reaction = String;

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
// FIXME: Set a syntactic constraint to eliminate impersonation.
#[cbor(transparent)]
pub struct Vote<const N: usize>(#[n(0)] Map<AID, Max<u64>>);

impl<const N: usize> Vote<N> {
    pub fn aggregate(&self) -> [usize; N] {
        let mut res = [0; N];

        for v in (*self.0).values() {
            res[v.0 as usize % N] += 1;
        }

        res
    }
}

#[derive(Clone, Default, Debug, PartialEq, SemiLattice, minicbor::Encode, minicbor::Decode)]
struct Comment {
    // Which comments are we responding to?
    // FIXME: Set a syntactic constraint against the partial order DAG such
    // that this cannot express cycles.
    #[n(0)]
    reply_to: Set<LUID>,
    // Redactable versioned content of a comment.
    #[n(1)]
    content: Map<u64, Redactable<String>>,
    #[n(2)]
    reactions: Map<Reaction, Vote<2>>,
}

// Actor ID remapping

// Each actor maintains a private mapping of Actor IDs to public keys; where a
// public subset of this mapping is revealed only after the actor references
// another actor's objects.

// Exported slices contain only the contributions from each actor, which may
// contain references to indices under other actors.

// Importing slices thus requires rewriting any of these IDs to correspond with
// the local mapping.

// Actors are responsible for creating unique object IDs. Colliding IDs will
// cause corresponding semilattice values to join, which may result in
// unexpected data loss or further confusion.

// An actor is a single device. Identities contain sets of actors.

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
pub struct ThreadActor {
    #[n(0)]
    id: AID,
    #[n(1)]
    counter: usize,
    // Each comment is assigned a Locally Unique ID.
    #[n(2)]
    threads: Map<LUID, Comment>,
}

impl ThreadActor {
    fn new(id: AID) -> Self {
        Self {
            threads: Default::default(),
            id,
            counter: 0,
        }
    }

    fn new_thread(&mut self, message: String) -> LUID {
        let count = self.counter;
        self.counter += 1;

        self.threads
            .entry((self.id.clone(), count))
            .content
            .entry(0)
            .join_assign(Redactable::Data(message));

        (self.id.clone(), count)
    }

    fn reply(&mut self, parent: LUID, message: String) -> LUID {
        let count = self.counter;
        self.counter += 1;

        self.threads
            .entry((self.id.clone(), count))
            .join_assign(Comment {
                reply_to: Set::singleton(parent),
                content: Map::singleton(0, Redactable::Data(message)),
                reactions: Map::default(),
            });

        (self.id.clone(), count)
    }

    // FIXME: should not take the edit_count
    fn edit(&mut self, target: LUID, message: String, edit_count: u64) {
        self.threads
            .entry(target)
            .content
            .entry(edit_count)
            .join_assign(Redactable::Data(message));
    }

    fn redact(&mut self, target: LUID, version: u64) {
        self.threads
            .entry(target)
            .content
            .entry(version)
            .join_assign(Redactable::Redacted);
    }

    fn react(&mut self, target: LUID, reaction: Reaction, vote: u64) {
        self.threads
            .entry(target)
            .reactions
            .entry(reaction)
            .0
            .entry(self.id.clone())
            .join_assign(Max(vote));
    }

    fn validate(&self) -> bool {
        for (k, v) in &*self.threads {
            // if we are not the author of a comment, we better not edit it.
            if k.0 != self.id {
                if v.content.len() > 0 {
                    return false;
                }
            }

            // likewise, we cannot react on anyone else' behalf.
            if v.reactions.values().any(|x| x.0.keys().any(|y| y != &self.id)) {
                return false;
            }
        }

        true
    }
}

fn main() {
    let mut alice = ThreadActor::new("Alice".to_owned());

    let a0 = alice.new_thread("Hello world. I have this issue. [..]".to_owned());

    let mut bob = ThreadActor::new("Bob".to_owned());

    let b0 = bob.reply(a0.clone(), "Huh. Did you run the tests?".to_owned());

    alice.react(b0.clone(), ":hourglass:".to_owned(), 1);

    let a1 = alice.reply(b0, "Ah! Test #3 failed. [..]".to_owned());
    // Alice may redact her message.
    alice.redact(a1.clone(), 0);
    // and submit a new version.
    alice.edit(a1, "Ah! Test #4 failed. [..]".to_owned(), 1);

    let mut alice_enc = Vec::new();
    minicbor::encode(&alice.threads, &mut alice_enc)
        .expect("Failed to encode Alice' state to CBOR");
    eprintln!("Alice: {}", minicbor::display(&alice_enc));

    let mut bob_enc = Vec::new();
    minicbor::encode(&bob.threads, &mut bob_enc).expect("Failed to encode Alice' state to CBOR");
    eprintln!("Bob: {}", minicbor::display(&bob_enc));

    let materialized = alice.threads.clone().join(bob.threads.clone());

    let mut materialized_enc = Vec::new();
    minicbor::encode(&materialized, &mut materialized_enc)
        .expect("Failed to encode Alice' state to CBOR");
    eprintln!("Materialized: {}", minicbor::display(&materialized_enc));

    // Alice and Bob have valid states
    assert!(alice.validate());
    assert!(bob.validate());

    // Let Eve assume Bob's state for the following.
    let mut eve = bob.clone();

    // If Eve tries to edit Alice' message...
    eve.edit(a0.clone(), "I am Alice!".to_owned(), 1);

    // her slice will no longer be valid.
    assert!(!eve.validate());

    // restore eve to a valid state
    eve = bob;

    // Likewise, she may not react on someone else' behalf.
    eve.threads
        .entry(a0)
        .reactions
        .entry(":love:".to_owned())
        .0
        .entry("Alice".to_owned())
        .join_assign(Max(0));
    assert!(!eve.validate());

    /*
    use std::fs;

    fs::write("alice.cbor", alice_enc);
    fs::write("bob.cbor", bob_enc);
    fs::write("materialized.cbor", materialized_enc);
    */
}
