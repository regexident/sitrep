//! A progress' associated task.

/// A monotonically increasing generation counter.
///
/// Specifies the generation at which a value was last changed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct Generation(pub(crate) usize);

impl Generation {
    /// Returns the raw internal generational counter value.
    pub fn as_raw(&self) -> usize {
        self.0
    }
}

/// The task associated with a given progress object.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Task {
    /// The task's label.
    pub label: Option<String>,
    /// The task's completed unit count.
    pub completed: usize,
    /// The task's total unit count.
    pub total: usize,
    /// The task's current generation.
    pub(crate) last_change: Generation,
}

impl Task {
    /// Builder-style method for setting the task's initial label.
    ///
    /// The default label is `None`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder-style method for setting the task's initial completed unit count.
    ///
    /// The default completed unit count is `0`.
    pub fn completed(mut self, completed: usize) -> Self {
        self.completed = completed;
        self
    }

    /// Builder-style method for setting the task's initial total unit count.
    ///
    /// The default total unit count is `0`.
    ///
    /// A `self.total` of `0` results in an indeterminate task progress.
    pub fn total(mut self, total: usize) -> Self {
        self.total = total;
        self
    }

    pub(crate) fn effective_completed(&self) -> usize {
        self.completed.min(self.total)
    }

    pub(crate) fn effective_total(&self) -> usize {
        self.completed.max(self.total)
    }

    pub(crate) fn effective_discrete(&self) -> (usize, usize) {
        (self.effective_completed(), self.effective_total())
    }
}
