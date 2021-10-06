#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{
    cmp::{Ordering, PartialOrd},
    fmt::Debug,
};

pub mod guarded_pair;
pub mod map;
pub mod ord;
pub mod pair;
pub mod set;

/// A bounded join-semilattice whose `PartialOrd` obeys the lattice
/// semantics and whose `Default` is the bottom element of the lattice.
pub trait SemiLattice: Default + PartialOrd {
    fn join(self, other: Self) -> Self;
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
        assert!(&bottom <= &a);

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
