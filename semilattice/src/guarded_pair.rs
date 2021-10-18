use core::cmp::{Ordering, PartialOrd};

use crate::SemiLattice;

/// A pair of semilattices, where the former acts as to version the latter.
#[derive(Default, Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct GuardedPair<G, V> {
    #[n(0)]
    pub guard: G,
    #[n(1)]
    pub value: V,
}

impl<G, V> PartialOrd for GuardedPair<G, V>
where
    G: SemiLattice,
    V: SemiLattice,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.guard.partial_cmp(&other.guard) {
            Some(Ordering::Equal) => self.value.partial_cmp(&other.value),
            x => x,
        }
    }
}

impl<G, V> SemiLattice for GuardedPair<G, V>
where
    G: SemiLattice,
    V: SemiLattice,
{
    fn join(self, other: Self) -> Self {
        match self.guard.partial_cmp(&other.guard) {
            Some(Ordering::Greater) => self,
            Some(Ordering::Less) => other,
            Some(Ordering::Equal) => Self {
                value: self.value.join(other.value),
                ..self
            },
            None => Self {
                guard: self.guard.join(other.guard),
                value: self.value.join(other.value),
            },
        }
    }
}

#[test]
fn check_laws() {
    use crate::{partially_verify_semilattice_laws, Max, Min};

    partially_verify_semilattice_laws([
        GuardedPair {
            guard: Min(0),
            value: Max(0),
        },
        GuardedPair {
            guard: Min(1),
            value: Max(1),
        },
        GuardedPair {
            guard: Min(0),
            value: Max(1),
        },
        GuardedPair {
            guard: Min(1),
            value: Max(0),
        },
    ]);
}
