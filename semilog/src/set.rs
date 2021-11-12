use core::{cmp, ops};

use crate::{DeferredRestore, Map, MapLattice, Semilattice};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "minicbor",
    derive(minicbor::Encode, minicbor::Decode),
    cbor(transparent)
)]
pub struct SetLattice<V> {
    #[cfg_attr(feature = "minicbor", n(0))]
    pub inner: MapLattice<V, ()>,
}

impl<V> SetLattice<V>
where
    V: Ord,
{
    pub fn singleton(val: V) -> Self {
        Self {
            inner: MapLattice::singleton(val, ()),
        }
    }

    pub fn insert(&mut self, val: V) {
        self.inner.insert(val, ());
    }
}

impl<V> ops::Deref for SetLattice<V> {
    type Target = MapLattice<V, ()>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<V> ops::DerefMut for SetLattice<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<V> Default for SetLattice<V>
where
    MapLattice<V, ()>: Default,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<'a, V: 'a> IntoIterator for &'a SetLattice<V> {
    type Item = &'a V;

    // Ohh. Don't need #![feature(type_alias_impl_trait)] for `impl
    // Iterator<Item = Self::Item>` because the closure doesn't capture its
    // environment, thus is also an ordinary `fn`.
    #[allow(clippy::type_complexity)]
    type IntoIter = core::iter::Map<core::slice::Iter<'a, (V, ())>, fn(&'a (V, ())) -> &'a V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter().map(|(v, _)| v)
    }
}

impl<V> FromIterator<V> for SetLattice<V>
where
    V: Ord,
{
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        Self {
            inner: MapLattice::from_iter(iter.into_iter().map(|v| (v, ()))),
        }
    }
}

impl<V> PartialOrd for SetLattice<V>
where
    V: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<V> Semilattice for SetLattice<V>
where
    V: Ord,
{
    fn join(self, other: Self) -> Self {
        Self {
            inner: self.inner.join(other.inner),
        }
    }
}

pub struct Set<K> {
    inner: Map<K, ()>,
}

impl<K> Default for Set<K> {
    fn default() -> Self {
        Self {
            inner: Map::default(),
        }
    }
}

impl<K> DeferredRestore for Set<K>
where
    K: Ord,
{
    type Value = K;

    fn for_each_stable(&self, mut func: impl FnMut(&Self::Value)) {
        self.inner.for_each_stable(|x| func(&x.0));
    }

    fn for_each_recent(&self, mut func: impl FnMut(&Self::Value)) {
        self.inner.for_each_recent(|x| func(&x.0));
    }

    fn insert(&mut self, val: impl Into<Self::Value>) {
        self.inner.insert((val.into(), ()));
    }

    fn restore(&mut self) -> bool {
        self.inner.restore()
    }

    fn join<T, Y>(&mut self, other: &T, mut func: impl FnMut(&Self::Value, &T::Value) -> Y)
    where
        T: DeferredRestore,
        Y: Into<Self::Value>,
    {
        self.inner.join(other, |x, y| (func(&x.0, y).into(), ()))
    }
}
