//! A progress' report.

use crate::{task::Generation, ProgressId};

/// A progress' report.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Report {
    /// The associated progress' identifier.
    pub progress_id: ProgressId,
    /// The associated progress' label.
    pub label: Option<String>,
    /// The number of accumulative completed units of work
    /// (i.e. including sub-reports' completed units).
    pub completed: usize,
    /// The number of accumulative total units of work
    /// (i.e. including sub-reports' total units).
    pub total: usize,
    /// A fractional representation of accumulative progress
    /// (i.e. including sub-reports) within range of `0.0..=1.0`.
    pub fraction: f64,
    /// A boolean value that indicates whether the tracked progress is indeterminate.
    pub is_indeterminate: bool,
    /// The reports of the associated progress' children.
    pub subreports: Vec<Report>,

    /// The generation at which the associated task,
    /// or any of its sub-tasks, were most recently changed.
    pub(crate) generation: Generation,
}

impl Report {
    pub(crate) fn new(
        progress_id: ProgressId,
        label: Option<String>,
        completed: usize,
        total: usize,
        subreports: Vec<Report>,
        generation: Generation,
    ) -> Self {
        let completed = Self::completed(completed, total);
        let total = Self::total(completed, total);
        let fraction = Self::fraction(completed, total);
        let is_indeterminate = Self::is_indeterminate(completed, total);

        Self {
            progress_id,
            label,
            completed,
            total,
            fraction,
            is_indeterminate,
            subreports,
            generation,
        }
    }

    fn completed(completed: usize, total: usize) -> usize {
        completed.min(total)
    }

    fn total(completed: usize, total: usize) -> usize {
        completed.max(total)
    }

    fn fraction(completed: usize, total: usize) -> f64 {
        match (completed, total) {
            (0, 0) => 0.0,
            (_, 0) => 1.0,
            (completed, total) => 1.0 * (completed as f64) / (total as f64),
        }
    }

    fn is_indeterminate(completed: usize, total: usize) -> bool {
        (completed == 0) && (total == 0)
    }

    pub(crate) fn discrete(&self) -> (usize, usize) {
        (self.completed, self.total)
    }
}
