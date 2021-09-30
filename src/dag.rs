use alloc::collections::BTreeMap;
use blake3::{hash, Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventKind {
    Root,
    CrossReference,
    Message,
}

#[derive(Debug, Clone)]
pub struct Event {
    kind: EventKind,
    hash: Hash,
}

impl<'a> From<&'a Dag> for Event {
    fn from(dag: &'a Dag) -> Self {
        Self {
            kind: EventKind::CrossReference,
            hash: dag.current,
        }
    }
}

impl<T> From<T> for Event
where
    T: AsRef<[u8]>,
{
    fn from(message: T) -> Self {
        Self {
            kind: EventKind::Message,
            hash: hash(message.as_ref()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DagEvent {
    root: Hash,
    hash: Hash,
    kind: EventKind,
}

impl DagEvent {
    pub fn hash(&self) -> Hash {
        if self.kind == EventKind::Root {
            self.root
        } else {
            let mut hasher = Hasher::new_keyed(self.root.as_bytes());
            hasher.update(&[self.kind as u8]);
            hasher.update(self.hash.as_bytes());
            hasher.finalize()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Dag {
    current: Hash,
    events: BTreeMap<OrdHash, DagEvent>,
}

// ugh, why isn't Hash deriving Ord?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdHash(Hash);

impl core::cmp::PartialOrd for OrdHash {
    fn partial_cmp(&self, other: &OrdHash) -> Option<core::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}
impl core::cmp::Ord for OrdHash {
    fn cmp(&self, other: &OrdHash) -> core::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl Dag {
    pub fn new(id: &[u8]) -> Self {
        let current = {
            let mut hasher = Hasher::new_derive_key(concat!(module_path!(), "::Dag::new"));
            hasher.update(id);
            hasher.finalize()
        };
        Self {
            current,
            events: BTreeMap::from([
                (OrdHash(current), DagEvent {
                    root: current,
                    hash: current,
                    kind: EventKind::Root,
                })
            ]),
        }
    }

    pub fn update(&mut self, event: impl Into<Event>) {
        let Event { kind, hash } = event.into();
        let event = DagEvent {
            root: self.current,
            hash,
            kind,
        };
        self.current = event.hash();
        self.events.insert(OrdHash(self.current), event);
    }

    // The DAG itself is not a semilattice but it contains one, the contained
    // event map is really a lattice set, which we need to look up by one of
    // its values.
    pub fn merge(&mut self, other: &Self) -> Result<(), usize> {
        let mut errors = 0;
        for (expect, event) in &other.events {
            // don't check the validity of events we already store.
            // Alternatively an event should not exist unless it is valid by
            // construction, so the check should occur at parse time instead.
            if self.events.contains_key(expect) {
                continue;
            }
            if expect.0 == event.hash() {
                // should do a linear merge insert
                self.events.insert(*expect, event.clone());
            } else {
                errors += 1;
            }
        }

        if errors == 0 {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn is_complete(&self) -> bool {
        self.events.iter().all(|(_, event)| {
            self.events.contains_key(&OrdHash(event.root))
                && event.kind == EventKind::Message || self.events.contains_key(&OrdHash(event.hash))
        })
    }
}

#[test]
fn dag() {
    // Alice and Bob share a common root
    let root = Dag::new(b"Private messages between Alice and Bob");
    let mut alice = root.clone();
    let mut bob = root.clone();

    alice.update(b"Welcome back Bob.");

    bob.update(&alice);
    bob.update(b"Hey Alice!");

    alice.update(&bob);
    alice.update(b"Are you free for lunch today?");

    // Bob experiences a hardware fault and reverts to the common root
    let mut bob_2 = root;
    // Bob sees Alice' new messages and his own
    bob_2.update(&alice);
    bob_2.update(&bob);
    bob_2.update(b"Huh, my computer crashed. Yeah, I'm free. 1230 at our place?");

    alice.update(&bob_2);
    alice.update(b"Ouch. Yeah, 1230 is good. See you there.");

    bob_2.update(&alice);

    // Please insert 1 dollar to continue the story.

    // Alice and Bob currently have incomplete graphs.
    assert!(!alice.is_complete());
    assert!(!bob.is_complete());
    assert!(!bob_2.is_complete());

    // If Alice wishes to distribute Bob's events, she only needs to
    // merge his events into her own DAG. This validates any entries
    // she did not already have and discards any invalid entries.
    alice.merge(&bob).expect("Bob produced illegal events!");
    alice.merge(&bob_2).expect("Bob produced illegal events!");
    // The merge did not commit to any of Bob's new events.
    alice.update(&bob);
    alice.update(&bob_2);

    // The DAG is complete if it contains all events.
    assert!(alice.is_complete());
}
