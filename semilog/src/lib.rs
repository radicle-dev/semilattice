#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(feature = "alloc", feature(slice_partition_dedup))]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{cmp, fmt, mem};

pub use semilog_macros::Semilattice;

mod datalog;
mod guarded_pair;
mod ord;
mod pair;
mod redactable;

#[cfg(feature = "alloc")]
mod map;
#[cfg(feature = "alloc")]
mod set;
#[cfg(feature = "alloc")]
mod vec;

pub use {
    datalog::{DeferredRestore, Iteration, Simple},
    guarded_pair::GuardedPair,
    ord::{Interval, Max, Min},
    pair::Pair,
    redactable::Redactable,
};

#[cfg(feature = "alloc")]
pub use {
    map::{Map, MapLattice},
    set::{Set, SetLattice},
    vec::VecLattice,
};

/// A bounded join-semilattice whose `PartialOrd` obeys the lattice semantics
/// and whose `Default` is the bottom element of the lattice.
pub trait Semilattice: Default + PartialOrd {
    fn join(self, other: Self) -> Self;

    fn join_assign(&mut self, other: Self) {
        *self = mem::take(self).join(other);
    }
}

impl Semilattice for () {
    fn join(self, _: Self) -> Self {}
}

/// Reduce an iterator of semilattice values to its least upper bound.
pub fn fold<S>(i: impl IntoIterator<Item = S>) -> S
where
    S: Semilattice,
{
    i.into_iter().fold(S::default(), S::join)
}

/// Partially verify the semantics of a `Semilattice`. For all provided samples
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
pub fn partially_verify_semilattice_laws<S: Semilattice + fmt::Debug + Clone>(
    samples: impl IntoIterator<Item = S> + Clone,
) {
    let bottom = S::default();

    for a in samples.clone() {
        // All samples must be greater or equal to the bottom element.
        assert!(
            bottom <= a,
            "Bottom is not less than or equal to: {:?}, {:?}",
            a,
            bottom.partial_cmp(&a)
        );

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
                Some(cmp::Ordering::Greater | cmp::Ordering::Equal) => assert_eq!(&ab, &a),
                Some(cmp::Ordering::Less) => assert_eq!(&ab, &b),
                None => {
                    assert!(
                        ab > a,
                        "Expected {:?} > {:?}; {:?}",
                        ab,
                        a,
                        ab.partial_cmp(&a)
                    );
                    assert!(
                        ab > b,
                        "Expected {:?} > {:?}; {:?}",
                        ab,
                        b,
                        ab.partial_cmp(&b)
                    );
                }
            }
        }
        // idempotent
        assert_eq!(&a, &fold([a.clone(), a.clone()]));
    }
}

/// A helper function intended for `core::cmp::PartialOrd::partial_cmp`. This
/// is used by the derive macro `#[derive(Semilattice)]`.
pub fn partial_ord_helper(
    orders: impl IntoIterator<Item = Option<cmp::Ordering>>,
) -> Option<cmp::Ordering> {
    let mut greater = false;
    let mut less = false;

    for ord in orders {
        match ord {
            Some(cmp::Ordering::Less) if !greater => less = true,
            Some(cmp::Ordering::Greater) if !less => greater = true,
            Some(cmp::Ordering::Equal) => (),
            _ => return None,
        }
    }

    Some(greater.cmp(&less))
}
