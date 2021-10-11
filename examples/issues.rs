#[cfg(not(feature = "alloc"))]
compile_error!("This example requires the alloc feature.");

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use semilattice::{hlist, HList, Map, Max, Pair, Redactable, SemiLattice, Set};

/// A local identifier for an actor. This does not need to be globally
/// consistent.
pub type AID = &'static str;

/// A locally unique ID. This does not need to be globally consistent.
// (owner ID, local total order of unique events observed under an AID)
pub type LUID = (AID, usize);

#[derive(Clone, Default, Debug, PartialEq, SemiLattice)]
pub struct Comment {
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

        for (_, v) in &*self.0 {
            res[v.0 as usize % 2] += 1;
        }

        res
    }
}

#[derive(Default, Debug, Clone, SemiLattice, PartialEq)]
struct ThreadGraph {
    // Each comment is assigned a Locally Unique ID.
    graph: Map<
        LUID,
        HList![
            // Versioned set of redactable comments.
            Comment,

            // A mapping of reactions to per-user votes.
            Map<Reaction, Vote<2>>,
        ],
    >,
}

fn main() {
    let mut alice = ThreadGraph::default();

    alice.graph.insert(
        ("Alice", 0),
        hlist![
            Comment {
                reply_to: Set::default(),
                content: Map::from(BTreeMap::from([(
                    0,
                    Redactable::Data("Hello world. I have this issue. [..]")
                ),])),
            },
            Map::default(),
        ],
    );

    let mut bob = alice.clone();

    bob.graph.insert(
        ("Bob", 0),
        hlist![
            Comment {
                reply_to: Set::from(BTreeSet::from([("Alice", 0)])),
                content: Map::from(BTreeMap::from([(
                    0,
                    Redactable::Data("Huh. Did you run the tests?")
                ),])),
            },
            Map::default(),
        ],
    );
}
