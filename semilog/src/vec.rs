use core::{
    cmp::{Ordering, PartialOrd},
    ops,
};

use crate::{partial_ord_helper, Semilattice};

use alloc::{vec, vec::Vec};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "minicbor",
    derive(minicbor::Encode, minicbor::Decode),
    cbor(transparent)
)]
pub struct VecLattice<T> {
    #[cfg_attr(feature = "minicbor", n(0))]
    pub inner: Vec<T>,
}

impl<T> Default for VecLattice<T> {
    fn default() -> Self {
        Self {
            inner: Vec::default(),
        }
    }
}

impl<T> PartialOrd for VecLattice<T>
where
    T: Semilattice,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        partial_ord_helper([
            self.inner.len().partial_cmp(&other.inner.len()),
            partial_ord_helper(
                self.inner
                    .iter()
                    .zip(&other.inner)
                    .map(|(a, b)| a.partial_cmp(b)),
            ),
        ])
    }
}

impl<T> Semilattice for VecLattice<T>
where
    T: Semilattice,
{
    fn join(mut self, other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(Ordering::Greater | Ordering::Equal) => self,
            Some(Ordering::Less) => other,
            None => {
                for (l, r) in self.inner.iter_mut().zip(other.inner) {
                    l.join_assign(r);
                }

                self
            }
        }
    }
}

impl<T> ops::Deref for VecLattice<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> VecLattice<T>
where
    T: Semilattice,
{
    pub fn singleton(v: T) -> Self {
        Self { inner: vec![v] }
    }

    pub fn push(&mut self, v: T) {
        self.inner.push(v);
    }

    pub fn entry(&self, key: u64) -> Option<&T> {
        self.inner.get(key as usize)
    }

    pub fn entry_mut(&mut self, key: u64) -> &mut T {
        if self.inner.len() <= key as usize {
            self.inner.resize_with(1 + key as usize, T::default);
        }

        self.inner.get_mut(key as usize).expect("BUG!")
    }
}
