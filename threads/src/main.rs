use core::mem;

use semilattice::{hlist, HList, Map, Max, Pair, Redactable, SemiLattice, Set};

/// A local identifier for an actor. This does not need to be globally
/// consistent.
pub type AID = String;

/// A locally unique ID. This does not need to be globally consistent.
// (owner ID, local total order of unique events observed under an AID)
pub type LUID = (AID, usize);

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
}

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

#[derive(Debug, minicbor::Encode, minicbor::Decode)]
pub struct ThreadActor {
    #[n(0)]
    id: AID,
    #[n(1)]
    counter: usize,
    // Each comment is assigned a Locally Unique ID.
    #[n(2)]
    threads: Map<
        LUID,
        HList![
            // Versioned set of redactable comments.
            Comment,

            // A mapping of reactions to per-user votes.
            Map<Reaction, Vote<2>>,
        ],
    >,
}

impl ThreadActor {
    fn new(id: AID) -> Self {
        Self {
            threads: Default::default(),
            id,
            counter: 0,
        }
    }

    fn fork(&self, id: AID) -> Self {
        Self {
            threads: self.threads.clone(),
            id,
            counter: 0,
        }
    }

    fn join(&mut self, other: Self) {
        self.threads = mem::take(&mut self.threads).join(other.threads);
    }

    fn new_thread(&mut self, message: String) -> LUID {
        let count = self.counter;
        self.counter += 1;

        self.threads.insert(
            (self.id.clone(), count),
            hlist![
                Comment {
                    reply_to: Set::default(),
                    content: Map::singleton(0, Redactable::Data(message)),
                },
                Map::default(),
            ],
        );

        (self.id.clone(), count)
    }

    fn reply(&mut self, parent: LUID, message: String) -> LUID {
        let count = self.counter;
        self.counter += 1;

        self.threads.insert(
            (self.id.clone(), count),
            hlist![
                Comment {
                    reply_to: Set::singleton(parent),
                    content: Map::singleton(0, Redactable::Data(message)),
                },
                Map::default(),
            ],
        );

        (self.id.clone(), count)
    }

    // FIXME: should not take the edit_count
    fn edit(&mut self, edit: LUID, message: String, edit_count: u64) {
        self.threads.insert(
            edit,
            hlist![
                Comment {
                    // we do not modify the reply-to
                    reply_to: Set::default(),
                    // we record another version
                    content: Map::singleton(edit_count, Redactable::Data(message)),
                },
                // we do not set any reactions
                Map::default(),
            ],
        );
    }

    fn redact(&mut self, message: LUID, version: u64) {
        self.threads.insert(
            message,
            hlist![
                Comment {
                    // we do not modify the reply-to
                    reply_to: Set::default(),
                    // we redact our message at this version
                    content: Map::singleton(version, Redactable::Redacted),
                },
                // we do not set any reactions
                Map::default(),
            ],
        );
    }

    fn react(&mut self, react_to: LUID, reaction: Reaction, vote: u64) {
        self.threads.insert(
            react_to,
            hlist![
                // we do not modify the comments
                Comment::default(),
                Map::singleton(reaction, Vote(Map::singleton(self.id.clone(), Max(vote)))),
            ],
        );
    }
}

fn main() {
    let mut alice = ThreadActor::new("Alice".to_owned());

    let a0 = alice.new_thread("Hello world. I have this issue. [..]".to_owned());

    let mut bob = ThreadActor::new("Bob".to_owned());

    let b0 = bob.reply(a0, "Huh. Did you run the tests?".to_owned());

    alice.react(b0.clone(), ":hourglass:".to_owned(), 1);

    let a1 = alice.reply(b0, "Ah! Test #3 failed. [..]".to_owned());
    // Alice may redact her message.
    alice.redact(a1.clone(), 0);
    // and submit a new version.
    alice.edit(a1, "Ah! Test #4 failed. [..]".to_owned(), 1);

    let mut alice_enc = Vec::new();
    minicbor::encode(&alice, &mut alice_enc).expect("Failed to encode Alice' state to CBOR");
    eprintln!("Alice: {}", minicbor::display(&alice_enc));

    let mut bob_enc = Vec::new();
    minicbor::encode(&bob, &mut bob_enc).expect("Failed to encode Alice' state to CBOR");
    eprintln!("Bob: {}", minicbor::display(&bob_enc));

    let materialized = alice.threads.join(bob.threads);

    let mut materialized_enc = Vec::new();
    minicbor::encode(&materialized, &mut materialized_enc).expect("Failed to encode Alice' state to CBOR");
    eprintln!("Materialized: {}", minicbor::display(&materialized_enc));

    /*
    use std::fs;

    fs::write("alice.cbor", alice_enc);
    fs::write("bob.cbor", bob_enc);
    fs::write("materialized.cbor", materialized_enc);
    */
}
