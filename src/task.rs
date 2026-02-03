//! A progress' associated task.

use std::borrow::Cow;

/// A task's state.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
#[repr(u8)]
pub enum State {
    /// A running task.
    #[default]
    Running,
    /// A paused task.
    Paused,
    /// A finished task.
    Finished,
    /// A cancelled task.
    Canceled,
}

/// The task associated with a given progress object.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Task {
    /// The task's label.
    pub label: Option<Cow<'static, str>>,
    /// The task's completed unit count.
    pub completed: usize,
    /// The task's total unit count.
    pub total: usize,
    /// The task's state.
    pub state: State,
    /// Whether or not the task is cancelable.
    pub is_cancelable: bool,
    /// Whether or not the task is pausable.
    pub is_pausable: bool,
}

impl Task {
    /// Builder-style method for setting the task's initial label.
    ///
    /// The default label is `None`.
    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
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

    /// Builder-style method for marking the task as being cancelable.
    ///
    /// The default is `false` (i.e. non-cancelable).
    pub fn cancelable(mut self) -> Self {
        self.is_cancelable = true;
        self
    }

    /// Builder-style method for marking the task as being pausable.
    ///
    /// The default is `false` (i.e. non-pausable).
    pub fn pausable(mut self) -> Self {
        self.is_pausable = true;
        self
    }

    // Returns the effective completed count, clamped to not exceed the total.
    //
    // Regular arithmetic is safe here since we're only comparing two `usize` values,
    // not aggregating multiple values that could overflow.
    pub(crate) fn effective_completed(&self) -> usize {
        self.completed.min(self.total)
    }

    // Returns the effective total, ensuring it's at least as large as completed.
    //
    // Regular arithmetic is safe here since we're only comparing two `usize` values,
    // not aggregating multiple values that could overflow.
    pub(crate) fn effective_total(&self) -> usize {
        self.completed.max(self.total)
    }

    pub(crate) fn effective_discrete(&self) -> (usize, usize) {
        (self.effective_completed(), self.effective_total())
    }
}
