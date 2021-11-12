use alloc::{borrow::ToOwned, vec, vec::Vec};
use core::{borrow::Borrow, cmp, mem, ops};

use crate::{DeferredRestore, Semilattice};

fn gallop<T>(mut slice: &[T], mut cmp: impl FnMut(&T) -> bool) -> &[T] {
    // if empty slice, or already >= element, return
    if !slice.is_empty() && cmp(&slice[0]) {
        let mut step = 1;
        while step < slice.len() && cmp(&slice[step]) {
            slice = &slice[step..];
            step <<= 1;
        }

        step >>= 1;
        while step > 0 {
            if step < slice.len() && cmp(&slice[step]) {
                slice = &slice[step..];
            }
            step >>= 1;
        }

        // advance one, as we always stayed < value
        slice = &slice[1..];
    }

    slice
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "minicbor",
    derive(minicbor::Encode, minicbor::Decode),
    cbor(transparent)
)]
pub struct MapLattice<K, V> {
    #[cfg_attr(feature = "minicbor", n(0))]
    pub inner: Vec<(K, V)>,
}

impl<K, V> MapLattice<K, V>
where
    K: Ord,
    V: Semilattice,
{
    pub fn singleton(key: K, val: V) -> Self {
        Self {
            inner: vec![(key, val)],
        }
    }

    pub fn insert(&mut self, key: K, val: V) {
        match self.inner.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(i) => self.inner[i].1.join_assign(val),
            Err(i) => self.inner.insert(i, (key, val)),
        }
    }

    pub fn entry<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match self.inner.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
            Ok(i) => Some(&self.inner[i].1),
            _ => None,
        }
    }

    pub fn entry_mut<Q>(&mut self, key: &Q) -> &mut V
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord + ToOwned<Owned = K>,
    {
        let i = match self.inner.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
            Ok(i) => i,
            Err(i) => {
                self.inner.insert(i, (key.to_owned(), V::default()));
                i
            }
        };

        &mut self.inner[i].1
    }
}

impl<K, V> From<Vec<(K, V)>> for MapLattice<K, V>
where
    K: Ord,
{
    fn from(mut inner: Vec<(K, V)>) -> Self {
        inner.sort_unstable_by(|(k1, _), (k2, _)| k1.cmp(k2));

        Self { inner }
    }
}

impl<K, V> ops::Deref for MapLattice<K, V> {
    type Target = Vec<(K, V)>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K, V> ops::DerefMut for MapLattice<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<K, V> Default for MapLattice<K, V> {
    fn default() -> Self {
        Self {
            inner: Vec::default(),
        }
    }
}

impl<K, V> PartialOrd for MapLattice<K, V>
where
    K: Ord,
    V: Semilattice,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        let mut greater = self.inner.len() > other.inner.len();
        let mut less = self.inner.len() < other.inner.len();

        let mut left = &*self.inner;
        let mut right = &*other.inner;

        while let Some((k1, v1)) = left.first() {
            let step = gallop(right, |(k2, _)| k2 < k1);
            // other contains keys we skipped over
            if step != right {
                less = true;
                right = step;
            }

            match right.first() {
                Some((k2, v2)) if k1 == k2 => match v1.partial_cmp(v2) {
                    Some(cmp::Ordering::Less) if !greater => less = true,
                    Some(cmp::Ordering::Greater) if !less => greater = true,
                    Some(cmp::Ordering::Equal) => (),
                    _ => return None,
                },
                // self contains a key not in other
                _ => {
                    if !less {
                        greater = true;
                    } else {
                        return None;
                    }
                }
            }

            // alternate left/right roles
            mem::swap(&mut left, &mut right);
            mem::swap(&mut greater, &mut less);
        }

        Some(greater.cmp(&less))
    }
}

impl<K, V> Semilattice for MapLattice<K, V>
where
    K: Ord,
    V: Semilattice,
{
    fn join(mut self, mut other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(cmp::Ordering::Greater | cmp::Ordering::Equal) => self,
            Some(cmp::Ordering::Less) => other,
            None => {
                self.inner.append(&mut other.inner);

                // sort is faster than unstable_sort with sequences of sorted tuples.
                self.inner.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

                self
            }
        }
    }
}

impl<K, V> FromIterator<(K, V)> for MapLattice<K, V>
where
    K: Ord,
    V: Semilattice,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::from(Vec::from_iter(iter))
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Map<K, V> {
    // fully processed values
    stable: Vec<Vec<(K, V)>>,
    // recently added, unprocessed values
    recent: Vec<(K, V)>,
    // (potentially) new values, yet to be processed
    to_add: Vec<(K, V)>,
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self {
            stable: Default::default(),
            recent: Default::default(),
            to_add: Default::default(),
        }
    }
}

impl<K, V> DeferredRestore for Map<K, V>
where
    K: Ord,
    V: Semilattice,
{
    type Value = (K, V);

    fn for_each_stable(&self, mut func: impl FnMut(&Self::Value)) {
        for x in &self.stable {
            for y in x {
                func(y);
            }
        }
    }

    fn for_each_recent(&self, mut func: impl FnMut(&Self::Value)) {
        for x in &self.recent {
            func(x);
        }
    }

    fn insert(&mut self, val: impl Into<Self::Value>) {
        self.to_add.push(val.into());
    }

    fn restore(&mut self) -> bool {
        fn merge<K, V>(mut vec: Vec<(K, V)>, mut other: Vec<(K, V)>) -> Vec<(K, V)>
        where
            K: Ord,
            V: Semilattice,
        {
            match (vec.last(), other.first(), other.last(), vec.first()) {
                // other is empty
                (_, None, _, _) => vec,
                // vec is empty
                (None, _, _, _) => other,
                // vec is a prefix of other
                (Some(a), Some(b), _, _) if a.0 <= b.0 => {
                    vec.append(&mut other);
                    vec
                }
                // vec is a suffix of other
                (_, _, Some(c), Some(d)) if c.0 <= d.0 => {
                    other.append(&mut vec);
                    other
                }
                // neither are empty nor a prefix of the other
                _ => {
                    // NOTE: Would prefer to not copy the (visually correct)
                    // unsafe code from Datafrog because we can probably do a
                    // fair bit better using `gallop` to partition runs.  If
                    // `other` only "updates" elements already in `vec`, or it
                    // only introduces new elements near the end, then we don't
                    // need a new vector.

                    // sort_by is faster than unstable_sort_by when sorting
                    // sequences of sorted vectors
                    vec.append(&mut other);
                    vec.sort_by(|x, y| x.0.cmp(&y.0));

                    let (dedup, dups) = vec.partition_dedup_by(|x, y| x.0 == y.0);

                    // partition_dedup_by maintains the order of `dedup` but
                    // does not define the order of `dups`.
                    for dup in dups {
                        dedup[dedup
                            .binary_search_by(|x| x.0.cmp(&dup.0))
                            .expect("dedup contains dups by definition")]
                        .1
                        .join_assign(core::mem::take(&mut dup.1));
                    }

                    let len = dedup.len();
                    vec.truncate(len);

                    vec
                }
            }
        }

        // 1. Merge self.recent into self.stable.
        if !self.recent.is_empty() {
            let mut recent = core::mem::take(&mut self.recent);
            while self.stable.last().map(|x| x.len() <= 2 * recent.len()) == Some(true) {
                recent = merge(
                    recent,
                    self.stable.pop().expect("We just checked last exists"),
                );
            }
            self.stable.push(recent);
        }

        // 2. Move self.to_add into self.recent.

        // 2a. Restore ordering for `self.to_add`
        let mut to_add = mem::take(&mut self.to_add);
        to_add.sort_by(|x, y| x.0.cmp(&y.0));

        // 2b. filter elements which are already greater in stable
        for batch in &self.stable {
            let mut slice = &batch[..];
            to_add.retain(|x| {
                slice = gallop(slice, |y| y.0 < x.0);
                if let Some(y) = slice.first() {
                    y.0 != x.0 || matches!(x.1.partial_cmp(&y.1), None | Some(cmp::Ordering::Less))
                } else {
                    false
                }
            });
        }
        self.recent = to_add;

        // continue until recent is empty.
        !self.recent.is_empty()
    }

    fn join<T, Y>(&mut self, other: &T, mut func: impl FnMut(&Self::Value, &T::Value) -> Y)
    where
        T: DeferredRestore,
        Y: Into<Self::Value>,
    {
        for s in &self.stable {
            for a in s {
                other.for_each_recent(|b| self.to_add.push(func(a, b).into()));
            }
        }

        for a in &self.recent {
            other.for_each_stable(|b| self.to_add.push(func(a, b).into()));
            other.for_each_recent(|b| self.to_add.push(func(a, b).into()));
        }
    }
}
