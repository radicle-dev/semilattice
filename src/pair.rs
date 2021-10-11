use crate::{self as semilattice, SemiLattice};

/// An anonymous pair of semilattices.
#[derive(Clone, Copy, Default, Debug, PartialEq, SemiLattice)]
pub struct Pair<A, B>(pub A, pub B);

#[macro_export]
macro_rules! HList {
    ($a:ty $(,)*) => {
        $a
    };
    ($a:ty, $($rest:ty),+ $(,)*) => {
        Pair<$a, HList!($($rest),*)>
    }
}

#[macro_export]
macro_rules! hlist {
    ($a:expr $(,)*) => {
        $a
    };
    ($a:expr, $($rest:expr),+ $(,)*) => {
        Pair($a, hlist!($($rest),*))
    }
}

#[test]
fn check_laws() {
    use crate::{fold, partially_verify_semilattice_laws, Max, Min};

    let _: HList!(u8, u16, u32) = hlist!(0u8, 1u16, 2u32);

    partially_verify_semilattice_laws((-5..5).map(|x| Pair(Min(x), Max(x))));

    assert_eq!(
        fold((-5..5).map(|x| Pair(Min(x), Max(x)))),
        Pair(Min(-5), Max(4))
    );
}
