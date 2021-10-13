#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{
    cmp::{Ordering, PartialOrd},
    fmt::Debug,
    mem,
};

pub use semilattice_macros::SemiLattice;

pub mod guarded_pair;
pub mod option;
pub mod ord;
pub mod pair;
pub mod redactable;

pub use crate::{
    guarded_pair::GuardedPair,
    option::UpsideDownOption,
    ord::{Max, Min},
    pair::Pair,
    redactable::Redactable,
};

#[cfg(feature = "alloc")]
pub mod map;
#[cfg(feature = "alloc")]
pub mod set;

#[cfg(feature = "alloc")]
pub use crate::{map::Map, set::Set};

/// A bounded join-semilattice whose `PartialOrd` obeys the lattice
/// semantics and whose `Default` is the bottom element of the lattice.
pub trait SemiLattice: Default + PartialOrd {
    fn join(self, other: Self) -> Self;

    fn join_assign(&mut self, other: Self) {
        *self = mem::take(self).join(other);
    }
}

/// Reduce an iterator of semilattice values to its least upper bound.
pub fn fold<S>(i: impl IntoIterator<Item = S>) -> S
where
    S: SemiLattice,
{
    i.into_iter().fold(S::default(), S::join)
}

/// Partially verify the semantics of a SemiLattice. For all provided samples
/// of the structure: the ACI properties must hold, the partial order must be
/// consistent with the least upper bound, and the bottom element must be the
/// least element.
///
/// ```lean
/// ∀ a b c ∈ S,
///   (a + b) + c = a + (b + c)
///   ∧ a + b = b + a
///   ∧ a + a = a
/// ```
pub fn partially_verify_semilattice_laws<S: SemiLattice + Debug + Clone>(
    samples: impl IntoIterator<Item = S> + Clone,
) {
    let bottom = S::default();

    for a in samples.clone() {
        // All samples must be greater or equal to the bottom element.
        assert!(bottom <= a);

        // ACI properties & partial order consistency
        for b in samples.clone() {
            // associative
            let ab = fold([a.clone(), b.clone()]);
            for c in samples.clone() {
                assert_eq!(
                    fold([ab.clone(), c.clone()]),
                    fold([a.clone(), fold([b.clone(), c])])
                )
            }
            // commutative
            assert_eq!(&ab, &fold([b.clone(), a.clone()]));

            // The least upper bound is consistent with the partial order
            match a.partial_cmp(&b) {
                Some(Ordering::Greater | Ordering::Equal) => assert_eq!(&ab, &a),
                Some(Ordering::Less) => assert_eq!(&ab, &b),
                None => {
                    assert_ne!(&ab, &a);
                    assert_ne!(&ab, &b);
                }
            }
        }
        // idempotent
        assert_eq!(&a, &fold([a.clone(), a.clone()]));
    }
}

/// A helper function intended for `core::cmp::PartialOrd::partial_cmp`. This
/// is used by the derive macro `#[derive(SemiLattice)]`.
pub fn partial_ord_helper(orders: impl IntoIterator<Item = Option<Ordering>>) -> Option<Ordering> {
    let mut greater = false;
    let mut less = false;

    for ord in orders {
        match ord {
            Some(Ordering::Less) if !greater => less = true,
            Some(Ordering::Greater) if !less => greater = true,
            Some(Ordering::Equal) => (),
            _ => return None,
        }
    }

    Some(greater.cmp(&less))
}
