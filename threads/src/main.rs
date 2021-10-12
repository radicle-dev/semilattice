use core::mem;

use semilattice::{hlist, HList, Map, Max, Pair, Redactable, SemiLattice, Set};

/// A local identifier for an actor. This does not need to be globally
/// consistent.
pub type AID = &'static str;

/// A locally unique ID. This does not need to be globally consistent.
// (owner ID, local total order of unique events observed under an AID)
pub type LUID = (AID, usize);

#[derive(Clone, Default, Debug, PartialEq, SemiLattice)]
struct Comment {
    // Which comments are we responding to?
    // FIXME: Set a syntactic constraint against the partial order DAG such
    // that this cannot express cycles.
    reply_to: Set<LUID>,
    // Redactable versioned content of a comment.
    content: Map<u64, Redactable<&'static str>>,
}

pub type Reaction = &'static str;

#[derive(Default, Debug, Clone, SemiLattice, PartialEq)]
// FIXME: Set a syntactic constraint to eliminate impersonation.
pub struct Vote<const N: usize>(Map<AID, Max<u64>>);

impl<const N: usize> Vote<N> {
    pub fn aggregate(&self) -> [usize; N] {
        let mut res = [0; N];

        for v in (*self.0).values() {
            res[v.0 as usize % N] += 1;
        }

        res
    }
}

#[derive(Debug)]
pub struct ThreadActor {
    // Each comment is assigned a Locally Unique ID.
    threads: Map<
        LUID,
        HList![
            // Versioned set of redactable comments.
            Comment,

            // A mapping of reactions to per-user votes.
            Map<Reaction, Vote<2>>,
        ],
    >,
    aid: &'static str,
    counter: usize,
}

impl ThreadActor {
    fn new(aid: &'static str) -> Self {
        Self {
            threads: Default::default(),
            aid,
            counter: 0,
        }
    }

    fn fork(&self, aid: &'static str) -> Self {
        Self {
            threads: self.threads.clone(),
            aid,
            counter: 0,
        }
    }

    fn join(&mut self, other: Self) {
        self.threads = mem::take(&mut self.threads).join(other.threads);
    }

    fn comment(&mut self, parent: Option<LUID>, message: &'static str) -> LUID {
        let count = self.counter;
        self.counter += 1;

        self.threads.insert(
            (self.aid, count),
            hlist![
                Comment {
                    reply_to: if let Some(parent) = parent {
                        Set::singleton(parent)
                    } else {
                        Set::default()
                    },
                    content: Map::singleton(0, Redactable::Data(message)),
                },
                Map::default(),
            ],
        );

        (self.aid, count)
    }

    // FIXME: should not take the edit_count
    fn edit(&mut self, edit: LUID, message: &'static str, edit_count: u64) {
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
                Map::singleton(reaction, Vote(Map::singleton(self.aid, Max(vote)))),
            ],
        );
    }
}

fn main() {
    let mut alice = ThreadActor::new("Alice");

    let a0 = alice.comment(None, "Hello world. I have this issue. [..]");

    let mut bob = alice.fork("Bob");

    let b0 = bob.comment(Some(a0), "Huh. Did you run the tests?");

    assert_ne!(alice.threads, bob.threads);
    alice.join(bob.fork("illegal actor id"));
    assert_eq!(alice.threads, bob.threads);
    alice.react(b0, ":hourglass:", 1);

    let a1 = alice.comment(Some(b0), "Ah! Test #3 failed. [..]");
    // Alice may redact her message.
    alice.redact(a1, 0);
    // and submit a new version.
    alice.edit(a1, "Ah! Test #4 failed. [..]", 1);

    assert_ne!(alice.threads, bob.threads);
    bob.join(alice.fork("illegal actor id"));

    assert_eq!(alice.threads, bob.threads);
}
