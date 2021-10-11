use core::{
    cmp::{Ord, Ordering, PartialOrd},
    mem,
    ops::Deref,
};

use alloc::collections::btree_map::{BTreeMap, Entry};

use crate::SemiLattice;

/// A map from arbitrary keys to semilattices.
#[derive(Debug, Clone, PartialEq)]
pub struct Map<K, V> {
    inner: BTreeMap<K, V>,
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<K, V> PartialOrd for Map<K, V>
where
    K: Ord,
    V: SemiLattice,
{
    // FIXME: very inefficient implementation...
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut greater = false;
        let mut less = false;

        // self is greater if other is missing any keys
        for k in self.inner.keys() {
            if !other.inner.contains_key(k) {
                greater = true
            }
        }

        // self is less if other has additional keys
        for (k, v) in &other.inner {
            // mutual keys contribute the order of their values too
            if let Some(val) = self.inner.get(k) {
                if val > v {
                    greater = true
                } else if val < v {
                    less = true
                }
            } else {
                less = true
            }
            // an inconsistency means there is no order.
            if greater && less {
                return None;
            }
        }

        Some(greater.cmp(&less))
    }
}

impl<K, V> SemiLattice for Map<K, V>
where
    K: Ord,
    V: SemiLattice,
{
    fn join(mut self, mut other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(Ordering::Greater | Ordering::Equal) => self,
            Some(Ordering::Less) => other,
            None => {
                if self.inner.len() < other.inner.len() {
                    core::mem::swap(&mut self, &mut other);
                }
                // FIXME: very inefficient
                for (k, v) in other.inner {
                    self.insert(k, v);
                }
                self
            }
        }
    }
}

impl<K, V> From<BTreeMap<K, V>> for Map<K, V>
where
    K: Ord,
    V: SemiLattice,
{
    fn from(inner: BTreeMap<K, V>) -> Self {
        Self { inner }
    }
}

impl<K, V> Deref for Map<K, V> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K, V> Map<K, V>
where
    K: Ord,
    V: SemiLattice,
{
    pub fn singleton(key: K, value: V) -> Self {
        let mut res = Self::default();

        res.insert(key, value);

        res
    }

    pub fn insert(&mut self, key: K, value: V) {
        match self.inner.entry(key) {
            Entry::Vacant(ve) => {
                ve.insert(value);
            }
            Entry::Occupied(mut oe) => {
                let val = oe.get_mut();
                *val = mem::take(val).join(value);
            }
        }
    }
}

#[test]
fn check_laws() {
    use crate::{fold, partially_verify_semilattice_laws, Max};

    let samples = (-5..5).map(|x| {
        Map::from(BTreeMap::from([
            // x = 4
            ("a", Max(x)),
            // x = 2 or 3
            ("b", Max(x * (5 - x))),
            // x = -5
            ("c", Max(5 - x)),
        ]))
    });

    partially_verify_semilattice_laws(samples.clone());

    assert_eq!(
        fold(samples),
        Map::from(BTreeMap::from([
            ("a", Max(4)),
            ("b", Max(6)),
            ("c", Max(10)),
        ]))
    );

    let mut a = Map::default();

    a.insert("Alice", Max(0));
    a.insert("Alice", Max(1));

    let mut b = Map::default();
    b.insert("Bob", Max(1));
    b.insert("Bob", Max(0));

    let mut c = fold([a, b]);
    c.insert("Carol", Max(0));
    assert_eq!(
        c.inner,
        BTreeMap::from([("Alice", Max(1)), ("Bob", Max(1)), ("Carol", Max(0)),])
    );
}
