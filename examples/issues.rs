use core::cmp::{Ord, PartialOrd};

#[cfg(not(feature = "alloc"))]
compile_error!("This example requires the alloc feature.");

use semilattice::{Map, Max, Set};

use blake3::Hasher;

mod dag {
    use core::cmp::{Ord, Ordering, PartialOrd};

    use semilattice::{map::Map, set::Set};

    use blake3::{Hash, Hasher};

    // Why is blake3::Hash not Ord?
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OrdHash(pub blake3::Hash);

    impl PartialOrd for OrdHash {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for OrdHash {
        fn cmp(&self, other: &Self) -> Ordering {
            self.0.as_bytes().cmp(other.0.as_bytes())
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Event {
        Root,
        CrossReference(OrdHash, OrdHash),
        Hash(OrdHash, OrdHash),
    }

    #[derive(Debug, Clone)]
    pub struct Dag {
        events: Map<Event, Set<Event>>,
        tag: Hash,
    }

    impl Default for Dag {
        fn default() -> Self {
            Self {
                events: Default::default(),
                tag: blake3::hash(concat!(module_path!(), "::Dag::new").as_bytes()),
            }
        }
    }

    impl Dag {
        pub fn reference(&mut self, other: &Self) -> OrdHash {
            self.events.insert(
                Event::CrossReference(OrdHash(self.tag), OrdHash(other.tag)),
                Set::default(),
            );
            self.tag = {
                let mut hasher =
                    Hasher::new_derive_key(concat!(module_path!(), "::Dag::reference"));
                hasher.update(self.tag.as_bytes());
                hasher.update(other.tag.as_bytes());
                hasher.finalize()
            };
            OrdHash(self.tag)
        }

        pub fn commit_hash(&mut self, hash: Hash) -> OrdHash {
            self.events.insert(
                Event::Hash(OrdHash(self.tag), OrdHash(hash)),
                Set::default(),
            );
            self.tag = {
                let mut hasher = Hasher::new_derive_key(concat!(module_path!(), "::Dag::hash"));
                hasher.update(self.tag.as_bytes());
                hasher.update(hash.as_bytes());
                hasher.finalize()
            };
            OrdHash(self.tag)
        }
    }
}

use crate::dag::{Dag, OrdHash};

pub type Author = &'static str;

#[derive(Clone, Default, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Comment {
    author: Author,
    content: &'static str,
}

pub type Reaction = &'static str;
// Interpret a vote value modulo 3 as {neutral, positive, negative}.
// (2^2k - 1) modulo 3 is always 0, thus the first and last elements are
// "neutral", for all unsigned integer types.
pub type Vote = Map<Author, Max<u64>>;

#[derive(Default, Debug, Clone)]
struct Thread {
    comments: Set<Comment>,
    edits: Map<OrdHash, Set<Comment>>,
    reactions: Map<OrdHash, Map<Reaction, Vote>>,
    dag: Dag,
}

impl Thread {
    fn comment(&mut self, comment: Comment) -> OrdHash {
        let hash = self.dag.commit_hash({
            let mut hasher = Hasher::new_derive_key(concat!(module_path!(), "::Thread::comment"));
            hasher.update(comment.author.as_bytes());
            hasher.update(comment.content.as_bytes());
            hasher.finalize()
        });
        self.comments.insert(comment);
        hash
    }

    fn edit(&mut self, old: OrdHash, comment: Comment) -> OrdHash {
        let event = self.dag.commit_hash({
            let mut hasher = Hasher::new_derive_key(concat!(module_path!(), "::Thread::edit"));
            hasher.update(old.0.as_bytes());
            hasher.update(comment.author.as_bytes());
            hasher.update(comment.content.as_bytes());
            hasher.finalize()
        });
        self.edits.insert(old, Set::singleton(comment));
        event
    }
}

fn main() {
    // The set of comments is pure without the costs and complexities of
    // partial orders. The partial order describes who has seen which messages
    // as of authoring their own messages. Both are independent to
    // authorization to contribute to the various sets (thus no signatures.)

    // Alice creates a new thread
    let mut alice = Thread::default();

    // She sends her first message.
    let a0 = alice.comment(Comment {
        author: "Alice",
        content: "Hello world. I have this issue. [..]",
    });

    // Bob observes the thread, he starts where she left off.
    let mut bob = alice.clone();
    let b0 = bob.comment(Comment {
        author: "Bob",
        content: "Huh. Can you run the tests?",
    });
    /*
    // Bob observes the thread and opts-in to contributing.
    // He acknowledges Alice' DAG
    let b0 = dag.insert(Event::CrossReference(root, a0.clone()));
    // and responds with his own comment for the thread.
    let b1 = dag.insert(Event::Comment(
        b0,
        comments.insert("Huh, can you run the tests?"),
    ));

    // Alice observes Bob's response
    let a1 = dag.insert(Event::CrossReference(a0, b1.clone()));
    // She reacts to Bob's message.
    let mut reactions = Set::default();
    let a2 = dag.insert(Event::Comment(
        a1,
        reactions.insert((b1.clone(), ":hourglass:")),
    ));
    // and later responds
    let a3 = dag.insert(Event::Comment(
        a2,
        comments.insert("Ah! Test #3 failed. [..]"),
    ));

    // Alice notices a typo in her last message and edits.
    let edits = Set::default();
    let a4 = dag.insert(Event::Comment(
        a3.clone(),
        edits.insert((a3, "Ah! Test #4 failed. [..]")),
    ));
    */
}

// For performance, safety and weaker assumptions; we should use local
// indices internally, use an efficient UHF for local deduplication, and
// only use globally consistent collision-resistant public hashes for
// replication purposes.

// Ideally content-addressing is scoped with strict delegation graphs. Ex.
// Alice' data can only be replicated by Bob if she signs her data under a
// "context key" which she has signed Bob as permitted to replicate.

// Alice cannot revoke Bob's right to replicate data she has produced under
// this context key, but she is free to rotate her replica context key and
// delegate replication rights as she likes.

// In public settings, Alice may delegate replication rights to the world.

// Ideally a rogue replica (which Alice has not authorized) cannot prove
// Alice authored her data they replicate.
