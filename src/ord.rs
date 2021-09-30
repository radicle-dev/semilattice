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

#[cfg(test)]
use crate::fold;

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
