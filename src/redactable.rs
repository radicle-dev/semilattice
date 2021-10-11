use core::cmp::{Ordering, PartialEq, PartialOrd};

use crate::SemiLattice;

/// Redactable data. The contained data is arbitrary, not a semilattice. Any
/// attempts to change the underlying value, will collapse to the redacted
/// state.
#[derive(Debug, Clone, PartialEq)]
pub enum Redactable<T> {
    // FIXME: It is syntactically invalid to use this variant.
    Uninitialized,
    Data(T),
    Redacted,
}

impl<T> Default for Redactable<T> {
    fn default() -> Self {
        Self::Uninitialized
    }
}

impl<T> PartialOrd for Redactable<T>
where
    T: PartialEq,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Redactable::{Data, Redacted, Uninitialized};

        match (self, other) {
            (a, b) if a == b => Some(Ordering::Equal),
            (_, Uninitialized) => Some(Ordering::Greater),
            (Uninitialized, _) => Some(Ordering::Less),
            (Redacted, Data(_)) => Some(Ordering::Greater),
            (Data(_), Redacted) => Some(Ordering::Less),
            _ => None,
        }
    }
}

impl<T> SemiLattice for Redactable<T>
where
    T: PartialEq,
{
    fn join(self, other: Self) -> Self {
        use Redactable::{Data, Redacted, Uninitialized};

        match (self, other) {
            (Uninitialized, Data(a)) | (Data(a), Uninitialized) => Data(a),
            (a, b) if a == b => a,
            _ => Redacted,
        }
    }
}

#[test]
fn check_laws() {
    use crate::partially_verify_semilattice_laws;

    use Redactable::{Data, Redacted};

    partially_verify_semilattice_laws([Redacted, Data("Hello world."), Data("Hello kitty.")]);
}
