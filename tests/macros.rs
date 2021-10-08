use semilattice::{SemiLattice, SemiLatticeOrd};

#[derive(Default, PartialEq, SemiLattice, SemiLatticeOrd)]
struct PairR<A, B> {
    a: A,
    b: B,
}

#[derive(Default, PartialEq, SemiLattice, SemiLatticeOrd)]
struct PairT<A, B>(A, B);

#[derive(Default, PartialEq, SemiLattice, SemiLatticeOrd)]
struct Singleton;
