use core::{cell, cmp, mem, ops};

use crate::Semilattice;

pub trait DeferredRestore {
    type Value;

    // fixme: accelerate ordered joins
    fn for_each_stable(&self, func: impl FnMut(&Self::Value));
    fn for_each_recent(&self, func: impl FnMut(&Self::Value));

    fn insert(&mut self, val: impl Into<Self::Value>);

    /// Merge recent into Stable. Combine ToAdd into Recent. Repeat until
    /// nothing changes.
    fn restore(&mut self) -> bool;

    fn join<T, Y>(&mut self, other: &T, func: impl FnMut(&Self::Value, &T::Value) -> Y)
    where
        T: DeferredRestore,
        Y: Into<Self::Value>;
}

pub struct Iteration {
    rounds: usize,
    changed: cell::Cell<bool>,
}

impl Iteration {
    pub fn new(rounds: usize) -> Self {
        Self {
            rounds,
            changed: cell::Cell::new(true),
        }
    }

    pub fn unfinished(&mut self) -> bool {
        self.rounds = self.rounds.saturating_sub(1);
        self.rounds > 0 && self.changed.replace(false)
    }

    pub fn guard<'a, T>(&'a self, inner: &'a mut T) -> Guard<'a, T>
    where
        T: DeferredRestore,
    {
        Guard {
            inner,
            changed: &self.changed,
        }
    }
}

pub struct Guard<'a, T: ?Sized>
where
    T: DeferredRestore,
{
    inner: &'a mut T,
    changed: &'a cell::Cell<bool>,
}

impl<T> ops::Deref for Guard<'_, T>
where
    T: DeferredRestore,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T> ops::DerefMut for Guard<'_, T>
where
    T: DeferredRestore,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<T: ?Sized> Drop for Guard<'_, T>
where
    T: DeferredRestore,
{
    fn drop(&mut self) {
        extern crate std;
        if std::dbg!(self.inner.restore()) {
            self.changed.set(true);
        }
    }
}

#[derive(Debug, Default)]
pub struct Simple<S> {
    stable: S,
    recent: S,
    pending: S,
}

impl<S> DeferredRestore for Simple<S>
where
    S: Semilattice + core::fmt::Debug,
{
    type Value = S;

    fn for_each_stable(&self, mut func: impl FnMut(&Self::Value)) {
        if self.stable > S::default() {
            func(&self.stable)
        }
    }

    fn for_each_recent(&self, mut func: impl FnMut(&Self::Value)) {
        if self.recent > S::default() {
            func(&self.recent)
        }
    }

    fn insert(&mut self, val: impl Into<Self::Value>) {
        self.pending.join_assign(val.into());
    }

    fn restore(&mut self) -> bool {
        self.stable
            .join_assign(mem::replace(&mut self.recent, mem::take(&mut self.pending)));

        extern crate std;
        std::dbg!((&self.stable, &self.recent));

        // continue until stable >= recent
        matches!(
            std::dbg!(self.stable.partial_cmp(&self.recent)),
            None | Some(cmp::Ordering::Less)
        )
    }

    fn join<T, Y>(&mut self, other: &T, mut func: impl FnMut(&Self::Value, &T::Value) -> Y)
    where
        T: DeferredRestore,
        Y: Into<Self::Value>,
    {
        if self.recent > S::default() {
            other.for_each_stable(|b| self.pending.join_assign(func(&self.recent, b).into()));
        }

        match (self.stable > S::default(), self.recent > S::default()) {
            (true, true) => other.for_each_recent(|b| {
                self.insert(func(&self.stable, b));
                self.insert(func(&self.recent, b));
            }),
            (true, false) => other.for_each_recent(|b| {
                self.insert(func(&self.stable, b));
            }),
            (false, true) => other.for_each_recent(|b| {
                self.insert(func(&self.recent, b));
            }),
            _ => (),
        }
    }
}

#[test]
fn check() {
    use crate::{Interval, Max};

    let mut interval = Simple::<Interval<i32>>::default();
    let mut x = Simple::<Max<i32>>::default();

    x.insert(5);

    interval.insert((5, 100));
    interval.insert((3, 50));

    let mut iteration = Iteration::new(50);
    while iteration.unfinished() {
        let mut interval = iteration.guard(&mut interval);
        let mut x = iteration.guard(&mut x);

        let q = (*x.recent + 1).min(7);
        x.insert(q);
        interval.join(&*x, |a, b| (a.lower.0 - 1, **b));
    }

    assert_eq!(iteration.rounds, 45);
}
