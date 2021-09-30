#![feature(array_zip)]

#[cfg(feature = "alloc")]
extern crate alloc;

// This is an allocation inefficient PoC. A better design would likely use an
// arena and immutable-first data structures.

use core::cmp::Ordering;

#[cfg(feature = "btree")]
pub mod btree;
#[cfg(feature = "dag")]
pub mod dag;
pub mod ord;

// fixme: #[derive(SemiLattice)]
pub trait SemiLattice {
    fn join(self, rhs: Self) -> Self;

    fn compare(&self, _rhs: &Self) -> Ordering {
        // this should obviously be removed...
        Ordering::Less
    }
}

impl<T, const N: usize> SemiLattice for [T; N]
where
    T: SemiLattice,
{
    fn join(self, rhs: Self) -> Self {
        self.zip(rhs).map(|(lhs, rhs)| lhs.join(rhs))
    }
}

impl<A, B> SemiLattice for (A, B)
where
    A: SemiLattice,
    B: SemiLattice,
{
    fn join(self, rhs: Self) -> Self {
        (
            SemiLattice::join(self.0, rhs.0),
            SemiLattice::join(self.1, rhs.1),
        )
    }
}

// This is a "PairLattice" as named in Anna's paper. I consider this name
// confusing because a pair is typically just a 2-tuple; but this lattice uses
// the first field as a version guard to determine if it picks or merges its
// values. When the guard is a map from node IDs to vector clocks, the value
// exhibits causal consistency.
pub struct GuardedPair<Guard, Value> {
    guard: Guard,
    value: Value,
}

impl<A, B> SemiLattice for GuardedPair<A, B>
where
    A: SemiLattice,
    B: SemiLattice,
{
    fn join(self, rhs: Self) -> Self {
        match self.guard.compare(&rhs.guard) {
            Ordering::Less => rhs,
            Ordering::Greater => self,
            Ordering::Equal => GuardedPair {
                guard: self.guard.join(rhs.guard),
                value: self.value.join(rhs.value),
            },
        }
    }
}

/// Panics if the input iterator is empty
pub fn fold<T, I>(i: I) -> T
where
    I: IntoIterator<Item = T>,
    T: SemiLattice + Default,
{
    let mut iter = i.into_iter();
    let first = iter.next().expect(concat!(
        "Called ",
        module_path!(),
        "::fold with an empty iterator."
    ));
    iter.fold(first, SemiLattice::join)
}
