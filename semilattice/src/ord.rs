use core::cmp::{Ord, Ordering, PartialOrd};

use crate::SemiLattice;

/// Selects the smallest value of a totally ordered and bounded type.
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct Min<T>(#[cfg_attr(feature = "minicbor", n(0))] pub T);

/// Selects the largest value of a totally ordered and bounded type.
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct Max<T>(#[cfg_attr(feature = "minicbor", n(0))] pub T);

impl<T> Default for Min<T>
where
    T: num::Bounded,
{
    fn default() -> Self {
        Self(T::max_value())
    }
}

impl<T> Default for Max<T>
where
    T: num::Bounded,
{
    fn default() -> Self {
        Self(T::min_value())
    }
}

impl<T> PartialOrd for Min<T>
where
    T: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // invert ordering
        Some(match self.0.cmp(&other.0) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => Ordering::Equal,
        })
    }
}

impl<T> PartialOrd for Max<T>
where
    T: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T> SemiLattice for Min<T>
where
    T: Ord + num::Bounded,
{
    fn join(self, other: Self) -> Self {
        Self(core::cmp::min(self.0, other.0))
    }
}

impl<T> SemiLattice for Max<T>
where
    T: Ord + num::Bounded,
{
    fn join(self, other: Self) -> Self {
        Self(core::cmp::max(self.0, other.0))
    }
}

#[test]
fn check_laws() {
    use crate::{fold, partially_verify_semilattice_laws};

    partially_verify_semilattice_laws((-5..5).map(Min));
    partially_verify_semilattice_laws((-5..5).map(Max));

    assert_eq!(fold((-5..5).map(Min)), Min(-5));
    assert_eq!(fold((-5..5).map(Max)), Max(4));
}
