use core::cmp::{Ordering, PartialOrd};

use crate::SemiLattice;

#[derive(Default, Debug, PartialEq, Clone, Copy)]
// #[derive(SemiLattice, SemiLatticePartialOrd)]
pub struct Pair<A, B>(pub A, pub B);

impl<A, B> PartialOrd for Pair<A, B>
where
    A: SemiLattice,
    B: SemiLattice,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.0.partial_cmp(&other.0), self.1.partial_cmp(&other.1)) {
            (Some(a), Some(b)) if a == b => Some(a),
            (Some(Ordering::Equal), Some(c)) | (Some(c), Some(Ordering::Equal)) => Some(c),
            _ => None,
        }
    }
}

impl<A, B> SemiLattice for Pair<A, B>
where
    A: SemiLattice,
    B: SemiLattice,
{
    fn join(self, other: Self) -> Self {
        Self(self.0.join(other.0), self.1.join(other.1))
    }
}

#[test]
fn check_laws() {
    use crate::{
        fold,
        ord::{Max, Min},
        partially_verify_semilattice_laws,
    };

    partially_verify_semilattice_laws((-5..5).map(|x| Pair(Min(x), Max(x))));

    assert_eq!(
        fold((-5..5).map(|x| Pair(Min(x), Max(x)))),
        Pair(Min(-5), Max(4))
    );
}
