use semilattice::{Map, Max, Redactable, SemiLattice, Set};

/// A local identifier for an actor. This does not need to be globally
/// consistent.
pub type AID = String;

/// A locally unique ID. This does not need to be globally consistent.
pub type MessageID = (AID, u64);

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
    reply_to: Set<MessageID>,
    // Redactable versioned content of a comment.
    #[n(1)]
    content: Map<u64, Redactable<String>>,
    #[n(2)]
    reactions: Map<Reaction, Vote<2>>,
}

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
pub struct ThreadActor {
    #[n(0)]
    id: AID,
    #[n(1)]
    device_id: u64,
    #[n(2)]
    counter: u64,
    #[n(3)]
    threads: Map<AID, Map<u64, Comment>>,
}

impl ThreadActor {
    fn new(id: AID, device_id: u64) -> Self {
        Self {
            threads: Default::default(),
            id,
            device_id,
            counter: 0,
        }
    }

    fn comment(&mut self, id: MessageID) -> &mut Comment {
        self.threads.entry(id.0).entry(id.1)
    }

    fn new_thread(&mut self, message: String) -> MessageID {
        let count = (self.counter << 16) + self.device_id;
        self.counter += 1;

        self.comment((self.id.clone(), count))
            .content
            .entry(0)
            .join_assign(Redactable::Data(message));

        (self.id.clone(), count)
    }

    fn reply(&mut self, parent: MessageID, message: String) -> MessageID {
        let count = (self.counter << 16) + self.device_id;
        self.counter += 1;

        self.comment((self.id.clone(), count)).join_assign(Comment {
            reply_to: Set::singleton(parent),
            content: Map::singleton(0, Redactable::Data(message)),
            reactions: Map::default(),
        });

        (self.id.clone(), count)
    }

    fn edit(&mut self, target: MessageID, message: String) {
        let device_id = self.device_id;
        let content = &mut self.comment(target).content;

        content
            .entry((u64::try_from(content.len()).unwrap() << 16) + device_id)
            .join_assign(Redactable::Data(message));
    }

    fn redact(&mut self, target: MessageID, version: u64) {
        self.comment(target)
            .content
            .entry(version)
            .join_assign(Redactable::Redacted);
    }

    fn react(&mut self, target: MessageID, reaction: Reaction, vote: u64) {
        let id = self.id.clone();

        self.comment(target)
            .reactions
            .entry(reaction)
            .0
            .entry(id)
            .join_assign(Max(vote));
    }

    fn validate(&self) -> bool {
        for (k, v) in &*self.threads {
            // we cannot impersonate other users
            if !v.values().all(|c| {
                (k == &self.id || c.content.len() == 0)
                    && c.reactions
                        .values()
                        .all(|x| x.0.keys().all(|y| y == &self.id))
            }) {
                return false;
            }
        }

        true
    }
}

fn main() {
    let mut alice = ThreadActor::new("Alice".to_owned(), 0);

    let a0 = alice.new_thread("Hello world. I have this issue. [..]".to_owned());

    let mut bob = ThreadActor::new("Bob".to_owned(), 0);

    let b0 = bob.reply(a0.clone(), "Huh. Did you run the tests?".to_owned());

    alice.react(b0.clone(), ":hourglass:".to_owned(), 1);

    let a1 = alice.reply(b0, "Ah! Test #3 failed. [..]".to_owned());
    // Alice may redact her message.
    alice.redact(a1.clone(), 0);
    // and submit a new version.
    alice.edit(a1, "Ah! Test #4 failed. [..]".to_owned());

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
    eve.edit(a0.clone(), "I am Alice!".to_owned());

    // her slice will no longer be valid.
    assert!(!eve.validate());

    // restore eve to a valid state
    eve = bob;

    // Likewise, she may not react on someone else' behalf.
    eve.comment(a0)
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
