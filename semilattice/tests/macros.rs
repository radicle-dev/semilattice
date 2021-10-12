use semilattice::SemiLattice;

#[derive(Default, PartialEq, SemiLattice)]
struct PairR<A, B> {
    a: A,
    b: B,
}

#[derive(Default, PartialEq, SemiLattice)]
struct PairT<A, B>(A, B);

#[derive(Default, PartialEq, SemiLattice)]
struct Singleton;
