use core::cmp::{Ordering, PartialOrd};

use crate::SemiLattice;

impl<T> SemiLattice for Option<T>
where
    T: SemiLattice,
{
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (None, a) | (a, None) => a,
            (Some(a), Some(b)) => Some(a.join(b)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub enum UpsideDownOption<T> {
    #[n(0)]
    Some(#[n(0)] T),
    #[n(1)]
    None,
}

impl<T> Default for UpsideDownOption<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Some(T::default())
    }
}

impl<T> PartialOrd for UpsideDownOption<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::None, Self::None) => Some(Ordering::Equal),
            (Self::None, Self::Some(_)) => Some(Ordering::Greater),
            (Self::Some(_), Self::None) => Some(Ordering::Less),
            (Self::Some(a), Self::Some(b)) => a.partial_cmp(b),
        }
    }
}

impl<T> SemiLattice for UpsideDownOption<T>
where
    T: SemiLattice,
{
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Some(a), Self::Some(b)) => Self::Some(a.join(b)),
            _ => Self::None,
        }
    }
}

impl<T> From<Option<T>> for UpsideDownOption<T> {
    fn from(o: Option<T>) -> Self {
        match o {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

#[test]
fn check_laws() {
    use crate::{fold, partially_verify_semilattice_laws, Max};

    let samples = [None, Some(Max(0)), Some(Max(5))];

    partially_verify_semilattice_laws(samples.clone());

    assert_eq!(fold(samples), Some(Max(5)));

    // Rust's imports are not lexically scoped; lets hack around that.
    {
        use UpsideDownOption::{None, Some};

        let samples = [None, Some(Max(0)), Some(Max(5))];

        partially_verify_semilattice_laws(samples.clone());

        assert_eq!(fold(samples), None);
    }
}
