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
    pub(crate) last_change: Generation,
}

impl Report {
    pub(crate) fn new(
        progress_id: ProgressId,
        label: Option<String>,
        completed: usize,
        total: usize,
        subreports: Vec<Report>,
        last_change: Generation,
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
            last_change,
        }
    }

    /// Returns a pruned version with all subreports older than
    /// `min_last_change` removed, or `None` if `self` itself is older.
    pub fn to_pruned(&self, min_last_change: Generation) -> Option<Self> {
        self.clone().into_pruned(min_last_change)
    }

    /// Consumes the `Report` and returns a pruned version with all subreports
    /// older than `min_last_change` removed, or `None` if `self` itself is older.
    pub fn into_pruned(mut self, min_last_change: Generation) -> Option<Self> {
        if self.prune(min_last_change) {
            Some(self)
        } else {
            {
                None
            }
        }
    }

    fn prune(&mut self, min_last_change: Generation) -> bool {
        self.subreports
            .retain_mut(|report| report.prune(min_last_change));

        self.last_change >= min_last_change
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

#[cfg(test)]
mod tests {
    use super::*;

    mod to_pruned {
        use super::*;

        #[test]
        fn prunes_self() {
            let report = Report {
                progress_id: ProgressId::new_unique(),
                last_change: Generation(0),
                ..Default::default()
            };

            assert_eq!(report.to_pruned(Generation(1)), None);
        }

        #[test]
        fn prunes_subreports() {
            let parent_id = ProgressId::new_unique();
            let child_id = ProgressId::new_unique();
            let grand_child_id = ProgressId::new_unique();

            let report = Report {
                progress_id: parent_id,
                subreports: vec![
                    Report {
                        progress_id: ProgressId::new_unique(),
                        last_change: Generation(1),
                        ..Default::default()
                    },
                    Report {
                        progress_id: child_id,
                        subreports: vec![Report {
                            progress_id: grand_child_id,
                            last_change: Generation(2),
                            ..Default::default()
                        }],
                        last_change: Generation(2),
                        ..Default::default()
                    },
                ],
                last_change: Generation(2),
                ..Default::default()
            };

            let parent = report.to_pruned(Generation(2)).unwrap();

            assert_eq!(parent.progress_id, parent_id);
            assert_eq!(parent.subreports.len(), 1);

            let child = &parent.subreports[0];
            assert_eq!(child.progress_id, child_id);
            assert_eq!(child.subreports.len(), 1);

            let grand_child = &child.subreports[0];
            assert_eq!(grand_child.progress_id, grand_child_id);
            assert_eq!(grand_child.subreports.len(), 0);
        }
    }
}
