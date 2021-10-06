use core::cmp::{Ord, Ordering, PartialOrd};

use alloc::collections::btree_set::BTreeSet;

use crate::SemiLattice;

#[derive(Debug, Clone, PartialEq)]
pub struct Set<K> {
    inner: BTreeSet<K>,
}

impl<K> Default for Set<K> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<K> Set<K>
where
    K: Ord,
{
    pub fn singleton(value: K) -> Self {
        Self {
            inner: BTreeSet::from([value]),
        }
    }

    pub fn insert(&mut self, value: K) {
        self.inner.insert(value);
    }
}

impl<K> From<BTreeSet<K>> for Set<K> {
    fn from(inner: BTreeSet<K>) -> Self {
        Self { inner }
    }
}

impl<K> PartialOrd for Set<K>
where
    K: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.inner.is_superset(&other.inner) {
            Some(Ordering::Greater)
        } else if self.inner.is_subset(&other.inner) {
            Some(Ordering::Less)
        } else {
            None
        }
    }
}

impl<K> SemiLattice for Set<K>
where
    K: Ord,
{
    fn join(mut self, mut other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(Ordering::Greater | Ordering::Equal) => self,
            Some(Ordering::Less) => other,
            None => {
                if self.inner.len() < other.inner.len() {
                    core::mem::swap(&mut self, &mut other);
                }
                self.inner.extend(other.inner);
                self
            }
        }
    }
}

#[test]
fn check_laws() {
    use crate::partially_verify_semilattice_laws;

    partially_verify_semilattice_laws([
        Set::from(BTreeSet::from([1, 2, 3])),
        Set::from(BTreeSet::from([1, 2, 4])),
        Set::from(BTreeSet::from([1, 2, 5])),
        Set::from(BTreeSet::from([1, 2, 3, 4, 5])),
    ]);
}

#[test]
fn check_hashes() {
    use crate::fold;

    let mut a = Set::default();

    a.insert("Alice");
    a.insert("Alice");

    let mut b = Set::default();
    b.insert("Bob");
    b.insert("Bob");

    let mut c = fold([a.clone(), b.clone()]);
    c.insert("Alice");
}
