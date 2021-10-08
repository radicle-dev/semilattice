use crate::{self as semilattice, SemiLattice, SemiLatticeOrd};

#[derive(Clone, Copy, Default, Debug, PartialEq, SemiLattice, SemiLatticeOrd)]
pub struct Pair<A, B>(A, B);

#[test]
fn check_laws() {
    use crate::{fold, partially_verify_semilattice_laws, Max, Min};

    partially_verify_semilattice_laws((-5..5).map(|x| Pair(Min(x), Max(x))));

    assert_eq!(
        fold((-5..5).map(|x| Pair(Min(x), Max(x)))),
        Pair(Min(-5), Max(4))
    );
}
