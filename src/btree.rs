use crate::SemiLattice;

use alloc::collections::{BTreeMap, BTreeSet};

impl<K> SemiLattice for BTreeSet<K>
where
    K: Ord,
{
    fn join(mut self, rhs: Self) -> Self {
        self.extend(rhs);
        self
    }
}

impl<K, V> SemiLattice for BTreeMap<K, V>
where
    K: Ord,
    // Default only because we steal the old value from the map.
    V: SemiLattice + Default,
{
    fn join(mut self, rhs: Self) -> Self {
        // Ugh, why is there no linear-time BTreeMap::merge_with ... ????
        if self.is_empty() {
            self = rhs;
        } else {
            for (k, v) in rhs {
                use alloc::collections::btree_map::Entry;
                match self.entry(k) {
                    Entry::Vacant(ve) => {
                        ve.insert(v);
                    }
                    Entry::Occupied(mut oe) => {
                        let value = oe.get_mut();
                        *value = v.join(core::mem::take(value));
                    }
                }
            }
        }

        self
    }
}

#[cfg(test)]
use crate::{fold, ord::*};

#[test]
fn set() {
    let a = BTreeSet::from(["k1", "k2", "k3"]);
    let b = BTreeSet::from(["k1", "k3", "k4"]);
    let c = BTreeSet::from(["k1", "k2", "k3", "k4"]);
    assert_eq!(fold([a, b]), c);
}

#[test]
fn map() {
    let a = BTreeMap::from([
        ("k1", (Min(0), Max(1))),
        ("k2", (Min(1), Max(2))),
        ("k3", (Min(2), Max(3))),
    ]);
    let b = BTreeMap::from([
        ("k1", (Min(3), Max(4))),
        ("k2", (Min(4), Max(5))),
        ("k4", (Min(5), Max(6))),
    ]);
    let c = BTreeMap::from([
        ("k1", (Min(0), Max(4))),
        ("k2", (Min(1), Max(5))),
        ("k3", (Min(2), Max(3))),
        ("k4", (Min(5), Max(6))),
    ]);
    assert_eq!(fold([a, b]), c);
}
