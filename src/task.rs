//! A progress' associated task.

/// A monotonically increasing generation counter.
///
/// Specifies the generation at which a value was last changed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct Generation(pub(crate) usize);

/// The task associated with a given progress object.
#[derive(Clone, PartialEq, Debug)]
pub struct Task {
    pub(crate) label: Option<String>,
    pub(crate) completed: usize,
    pub(crate) total: usize,
    pub(crate) weight: f64,
    pub(crate) generation: Generation,
}

impl Task {
    /// Builder-style method for setting the task's initial label.
    ///
    /// The default label is `None`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.set_label(label.into());
        self
    }

    /// Builder-style method for setting the task's initial completed unit count.
    ///
    /// The default completed unit count is `0`.
    pub fn completed(mut self, completed: usize) -> Self {
        self.set_completed(completed);
        self
    }

    /// Builder-style method for setting the task's initial total unit count.
    ///
    /// The default total unit count is `0`.
    ///
    /// A `self.total` of `0` results in an indeterminate task progress.
    pub fn total(mut self, total: usize) -> Self {
        self.set_total(total);
        self
    }

    /// Builder-style method for setting the task's initial weight.
    ///
    /// The default weight is `1.0`.
    ///
    /// # Panics
    ///
    /// Panics if `weight <= 0.0`.
    pub fn weight(mut self, weight: f64) -> Self {
        self.set_weight(weight);
        self
    }

    /// Sets the task's label to `label`.
    pub fn set_label(&mut self, label: impl Into<Option<String>>) {
        self.label = label.into();
    }

    /// Increments the task's completed unit count by `1`.
    pub fn increment_completed(&mut self) {
        self.increment_completed_by(1);
    }

    /// Increments the task's completed unit count by `increment`.
    pub fn increment_completed_by(&mut self, increment: usize) {
        self.set_completed(self.completed + increment);
    }

    /// Sets the task's completed unit count to `completed`.
    pub fn set_completed(&mut self, completed: usize) {
        self.completed = completed;
    }

    /// Sets the task's total unit count to `total`.
    ///
    /// A `self.total` of `0` results in an indeterminate task progress.
    pub fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    /// Sets the task's weight to `weight`.
    ///
    /// # Panics
    ///
    /// Panics if `weight <= 0.0`.
    pub fn set_weight(&mut self, weight: f64) {
        assert!(weight > 0.0);
        self.weight = weight;
    }

    /// Returns `true` if `self.total == 0`, otherwise `false`.
    pub fn is_indeterminate(&self) -> bool {
        self.total == 0
    }

    pub(crate) fn set_generation(&mut self, generation: Generation) {
        self.generation = generation;
    }

    pub(crate) fn discrete(&self) -> Option<(usize, usize)> {
        if self.total > 0 {
            let completed = self.completed.min(self.total);
            let total = self.completed.max(self.total);
            Some((completed, total))
        } else {
            None
        }
    }

    pub(crate) fn fraction(&self) -> Option<f64> {
        self.discrete()
            .map(|(completed, total)| (1.0 * (completed as f64) / (total as f64)) as f64)
    }
}

impl Default for Task {
    /// Returns a task with following default values:
    ///
    /// - `label: None`,
    /// - `completed: 0`,
    /// - `total: 0`,
    /// - `weight: 1.0`,
    fn default() -> Self {
        Self {
            label: Default::default(),
            completed: Default::default(),
            total: Default::default(),
            weight: 1.0,
            generation: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn set_weight_with_zero_panics() {
        let mut task = Task::default();

        task.set_weight(0.0);
    }

    #[test]
    fn indeterminate() {
        let task = Task {
            ..Default::default()
        };

        assert_eq!(task.discrete(), None);
        assert_eq!(task.fraction(), None);
    }

    #[test]
    fn determinate() {
        let task = Task {
            completed: 1,
            total: 10,
            ..Default::default()
        };

        assert_eq!(task.discrete(), Some((1, 10)));
        assert_eq!(task.fraction(), Some(0.1));
    }
}
