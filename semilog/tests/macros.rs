use semilog::Semilattice;

#[derive(Default, PartialEq, Semilattice)]
struct PairR<A, B> {
    a: A,
    b: B,
}

#[derive(Default, PartialEq, Semilattice)]
struct PairT<A, B>(A, B);

#[derive(Default, PartialEq, Semilattice)]
struct Singleton;
