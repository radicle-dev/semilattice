use core::{cmp, ops};

use crate::{partial_ord_helper, Semilattice};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct Max<T>(#[cfg_attr(feature = "minicbor", n(0))] pub T);

impl<T> Default for Max<T>
where
    T: num_traits::bounds::Bounded,
{
    fn default() -> Self {
        Self(T::min_value())
    }
}

impl<T> ops::Deref for Max<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Max<T> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

impl<T> Semilattice for Max<T>
where
    T: num_traits::bounds::Bounded + Ord,
{
    fn join(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

#[allow(clippy::derive_ord_xor_partial_ord)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "minicbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct Min<T>(#[cfg_attr(feature = "minicbor", n(0))] pub T);

impl<T> Default for Min<T>
where
    T: num_traits::bounds::Bounded,
{
    fn default() -> Self {
        Self(T::max_value())
    }
}

impl<T> ops::Deref for Min<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Min<T> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

impl<T> cmp::PartialOrd for Min<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl<T> Semilattice for Min<T>
where
    T: num_traits::bounds::Bounded + Ord,
{
    fn join(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

#[derive(Debug, PartialEq)]
pub struct Interval<T> {
    pub lower: Max<T>,
    pub upper: Min<T>,
}

impl<T> Interval<T>
where
    T: num_traits::bounds::Bounded,
{
    fn top() -> Self {
        Self {
            lower: Max(T::max_value()),
            upper: Min(T::min_value()),
        }
    }
}

impl<T> Default for Interval<T>
where
    T: num_traits::bounds::Bounded,
{
    fn default() -> Self {
        Self {
            lower: Default::default(),
            upper: Default::default(),
        }
    }
}

impl<T> From<(T, T)> for Interval<T>
where
    T: num_traits::bounds::Bounded + Ord,
{
    fn from((lower, upper): (T, T)) -> Self {
        if lower <= upper {
            Self {
                lower: lower.into(),
                upper: upper.into(),
            }
        } else {
            Self::top()
        }
    }
}

impl<T> cmp::PartialOrd for Interval<T>
where
    T: num_traits::bounds::Bounded + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        partial_ord_helper([
            self.lower.partial_cmp(&other.lower),
            self.upper.partial_cmp(&other.upper),
        ])
    }
}

impl<T> Semilattice for Interval<T>
where
    T: num_traits::bounds::Bounded + Ord,
{
    fn join(self, other: Self) -> Self {
        let lower = self.lower.join(other.lower);
        let upper = self.upper.join(other.upper);

        if lower.0 <= upper.0 {
            Self { lower, upper }
        } else {
            // collapse to the top element.
            Self::top()
        }
    }
}
