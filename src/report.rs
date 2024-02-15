//! A progress' report.

use crate::{task::Generation, ProgressId};

/// A progress' report.
#[derive(Clone, PartialEq, Debug)]
pub struct Report {
    pub(crate) progress_id: ProgressId,
    pub(crate) label: Option<String>,
    pub(crate) discrete: Option<(usize, usize)>,
    pub(crate) fraction: Option<f32>,
    pub(crate) subreports: Vec<Report>,
    pub(crate) weight: f32,
    pub(crate) generation: Generation,
}

impl Report {
    /// The associated progress' identifier.
    pub fn progress_id(&self) -> ProgressId {
        self.progress_id
    }

    /// The associated progress' label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns a discrete representation of progress as `Some((completed, total))`
    /// if the associated task is determinate, otherwise returns `None`.
    ///
    /// # Invariants:
    ///
    /// - `total > 0`
    /// - `completed <= total`
    pub fn discrete(&self) -> Option<(usize, usize)> {
        self.discrete
    }

    /// Returns a fractional representation of progress as `Some(fraction)`
    /// if the associated task is determinate, otherwise returns `None`.
    ///
    /// If the associated task has NO children, then the value of `fraction`
    /// conceptually corresponds to the following formula:
    ///
    /// ```ignore
    /// let (completed, total) = self.discrete().unwrap();
    /// let fraction = 1.0 * (completed as f32) / (total as f32);
    /// ```
    ///
    /// If the associated task has children, then the value of `fraction`
    /// conceptually corresponds a weighted sum of its children's fraction values:
    ///
    /// ```ignore
    /// let mut fraction = 0.0;
    /// let mut total_weight = 0.0;
    /// for subreport in self.subreports() {
    ///     fraction += subreport.fraction() * subreport.weight();
    ///     total_weight += subreport.weight();
    /// }
    /// fraction /= total_weight;
    /// ```
    ///
    /// # Invariants:
    ///
    /// - `fraction >= 0.0`
    /// - `fraction <= 1.0`
    pub fn fraction(&self) -> Option<f32> {
        self.fraction
    }

    /// Returns the reports of the associated progress' children.
    pub fn subreports(&self) -> &[Report] {
        &self.subreports
    }

    /// Returns the generation at which the associated task, or any of its sub-tasks, were last changed.
    pub fn generation(&self) -> Generation {
        self.generation
    }
}
