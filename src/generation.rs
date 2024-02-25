use std::sync::atomic::{AtomicUsize, Ordering};

/// A monotonically increasing generation counter.
///
/// Specifies the generation at which a value was last changed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct Generation(pub(crate) usize);

impl Generation {
    /// Returns the smallest possible generation.
    pub const MIN: Self = Self(usize::MIN);

    /// Returns the largest possible generation.
    pub const MAX: Self = Self(usize::MAX);

    /// Returns the raw internal generational counter value.
    pub fn as_raw(&self) -> usize {
        self.0
    }

    pub(crate) fn add(self, increment: usize) -> (Self, bool) {
        let (value, overflow) = self.0.overflowing_add(increment);
        (Self(value), overflow)
    }

    pub(crate) fn max(self, other: Generation) -> Self {
        Self(self.0.max(other.0))
    }
}

pub(crate) struct AtomicGeneration(pub(crate) AtomicUsize);

impl From<Generation> for AtomicGeneration {
    fn from(generation: Generation) -> Self {
        Self(AtomicUsize::from(generation.0))
    }
}

impl AtomicGeneration {
    pub(crate) fn load(&self, order: Ordering) -> Generation {
        Generation(self.0.load(order))
    }

    pub(crate) fn swap(&self, generation: Generation, order: Ordering) -> Generation {
        Generation(self.0.swap(generation.0, order))
    }

    pub(crate) fn fetch_max(&self, generation: Generation, order: Ordering) -> Generation {
        Generation(self.0.fetch_max(generation.0, order))
    }

    pub(crate) fn fetch_add(&self, increment: usize, order: Ordering) -> Generation {
        Generation(self.0.fetch_add(increment, order))
    }
}
