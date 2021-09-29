#![feature(array_zip)]
#![cfg_attr(not(feature = "std"), no_std)]

// fixme: #[derive(SemiLattice)]
pub trait SemiLattice {
    fn join(self, rhs: Self) -> Self;
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

#[cfg(feature = "std")]
mod btree_semilattice {
    use crate::SemiLattice;

    use std::collections::btree_map;

    impl<K, V> SemiLattice for btree_map::BTreeMap<K, V>
    where
        K: Ord,
        // Default only because we steal the old value from the map.
        V: SemiLattice + Default,
    {
        fn join(mut self, rhs: Self) -> Self {
            // Ugh, why is there no linear-time BTreeMap::merge_with ... ????
            if self.is_empty() {
                self = rhs;
            } else {
                for (k, v) in rhs {
                    use std::collections::btree_map::Entry;
                    match self.entry(k) {
                        Entry::Vacant(ve) => {
                            ve.insert(v);
                        }
                        Entry::Occupied(mut oe) => {
                            let value = oe.get_mut();
                            let stolen = core::mem::take(value);
                            *value = v.join(stolen);
                        }
                    }
                }
            }

            self
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

pub mod ord {
    use core::cmp;

    use crate::SemiLattice;

    #[derive(Debug, Default, PartialOrd, Ord, PartialEq, Eq)]
    pub struct Min<T>(pub T);

    #[derive(Debug, Default, PartialOrd, Ord, PartialEq, Eq)]
    pub struct Max<T>(pub T);

    impl<T> SemiLattice for Min<T>
    where
        T: cmp::Ord,
    {
        fn join(self, rhs: Self) -> Self {
            Self(self.0.min(rhs.0))
        }
    }

    impl<T> SemiLattice for Max<T>
    where
        T: cmp::Ord,
    {
        fn join(self, rhs: Self) -> Self {
            Self(self.0.max(rhs.0))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{fold, ord::*};

    #[test]
    #[should_panic]
    fn empty_fold_panics() {
        let _ = fold::<Max<i32>, _>([]);
    }

    #[test]
    fn min_max_i32_and_str() {
        assert_eq!(fold([-1, 0, 1].map(Min)), Min(-1));
        assert_eq!(fold([-1, 0, 1].map(Max)), Max(1));
        assert_eq!(
            fold((-1..).into_iter().take(5).map(|x| (Min(x), Max(x)))),
            (Min(-1), Max(3))
        );
        assert_eq!(fold(["Hello world!", "Hello"].map(Min)), Min("Hello"));
        assert_eq!(
            fold(["Hello world!", "Hello"].map(Max)),
            Max("Hello world!")
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn map_lattice() {
        use crate::SemiLattice;
        use std::collections::BTreeMap;

        let a = BTreeMap::from([
            ("k1", (Min(0), Max(1))),
            ("k2", (Min(1), Max(2))),
            ("k3", (Min(2), Max(3))),
        ]);
        let b = BTreeMap::from([
            ("k1", (Min(3), Max(4))),
            ("k2", (Min(4), Max(5))),
            ("k4", (Min(5), Max(6))),
        ]);
        let c = BTreeMap::from([
            ("k1", (Min(0), Max(4))),
            ("k2", (Min(1), Max(5))),
            ("k3", (Min(2), Max(3))),
            ("k4", (Min(5), Max(6))),
        ]);
        assert_eq!(a.join(b), c);
    }
}
