//! A progress.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock, Weak,
    },
};

use crate::{
    event::Event, report::Report, task::Task, Generation, MessageEvent, PriorityLevel,
    ProgressEvent, ProgressEventKind,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A progress' unique identifier.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ProgressId(pub(crate) usize);

impl ProgressId {
    pub(crate) fn new_unique() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

/// Types for observing events of a progress.
pub trait Observer: Send + Sync {
    /// Observes an event emitted by a progress.
    fn observe(&self, event: Event);
}

/// Types for generating progress reports.
pub trait Reporter: Send + Sync {
    /// Generates the report for a progress.
    fn report(&self) -> Report;
}

/// The progress' state.
struct ProgressState {
    /// An associated task.
    task: Task,
    /// The progress tree's `Observer`.
    ///
    /// All progresses in a progress tree share the same observer.
    observer: Arc<dyn Observer>,
    /// An atomic counter for obtaining the tree's generation.
    ///
    /// All progresses in a progress tree share the same counter.
    max_generation: Arc<AtomicUsize>,
    /// The minimum priority level.
    min_priority_level: Option<PriorityLevel>,
}

/// The progress' relationships.
struct ProgressRelationships {
    /// The progress' parent progress, if there is one.
    parent: Weak<Progress>,
    /// The progress' child progresses, if there are any.
    children: HashMap<ProgressId, Arc<Progress>>,
}

/// A progress.
pub struct Progress {
    /// The progress' unique identifier.
    id: ProgressId,
    /// The progress' relationships.
    relationships: RwLock<ProgressRelationships>,
    /// The progress' state.
    state: RwLock<ProgressState>,
}

impl Progress {
    /// Creates a progress object for the given `task`,
    /// emitting relevant events to `observer`.
    ///
    /// Returned are the progress itself, as well as a `Reporter`
    /// which is used on the receiving end of the channel for obtaining reports.
    pub fn new(task: Task, observer: Arc<dyn Observer>) -> (Arc<Self>, Weak<impl Reporter>) {
        let parent = Weak::new();
        let max_generation = Arc::new(AtomicUsize::default());

        let progress = Self::new_impl(task, parent, observer, max_generation);
        let reporter = Arc::downgrade(&progress);

        (progress, reporter)
    }

    /// Creates a progress object for the given `task` as a sub-progress of `parent`,
    /// emitting relevant events to `observer`.
    ///
    /// Returned are the progress itself, as well as a `Reporter`
    /// which is used on the receiving end of the channel for obtaining reports.
    pub fn new_with_parent(task: Task, parent: &Arc<Self>) -> Arc<Self> {
        let parent_state = parent.state.read().unwrap();

        // Children share the observer of their parent:
        let observer = parent_state.observer.clone();
        // Children share the generation of their parent:
        let max_generation = Arc::clone(&parent_state.max_generation);

        let child = Self::new_impl(task, Arc::downgrade(parent), observer, max_generation);

        parent
            .relationships
            .write()
            .unwrap()
            .children
            .insert(child.id(), Arc::clone(&child));

        let parent_state = parent.state.read().unwrap();

        parent.emit_update_event(&*parent_state.observer);

        child
    }

    fn new_impl(
        task: Task,
        parent: Weak<Self>,
        observer: Arc<dyn Observer>,
        max_generation: Arc<AtomicUsize>,
    ) -> Arc<Self> {
        let id = ProgressId::new_unique();
        let parent = parent;
        let children = HashMap::new();

        let min_priority_level = None;

        let relationships = RwLock::new(ProgressRelationships { parent, children });

        let state = RwLock::new(ProgressState {
            task,
            observer,
            max_generation,
            min_priority_level,
        });

        Arc::new(Self {
            id,
            relationships,
            state,
        })
    }

    /// Attaches `child` to `self`, returning the `child's` own and now no longer used `Observer`.
    pub fn attach_child(self: &Arc<Self>, child: &Arc<Self>) -> Arc<dyn Observer> {
        let parent_state = self.state.read().unwrap();

        let max_generation = {
            let parent_max_generation = parent_state.max_generation.load(Ordering::Relaxed);
            let child_max_generation = child
                .state
                .read()
                .unwrap()
                .max_generation
                .load(Ordering::Relaxed);
            parent_max_generation.max(child_max_generation)
        };

        // Bump the parent's generation, if necessary:
        parent_state
            .max_generation
            .store(max_generation, Ordering::SeqCst);

        // Children share the generation of their parent:
        child.state.write().unwrap().max_generation = Arc::clone(&parent_state.max_generation);

        // Make sure the child uses the parent's observer from now on:
        let observer = {
            let parent_observer = parent_state.observer.clone();
            std::mem::replace(&mut child.state.write().unwrap().observer, parent_observer)
        };

        self.relationships
            .write()
            .unwrap()
            .children
            .insert(child.id(), Arc::clone(child));

        let state = self.state.read().unwrap();

        self.emit_update_event(&*state.observer);

        observer
    }

    /// Detaches `child` from `self`, giving it a new `observer`.
    pub fn detach_child(self: &Arc<Self>, child: &Arc<Self>, observer: Arc<dyn Observer>) {
        debug_assert!(
            self.relationships
                .read()
                .unwrap()
                .children
                .contains_key(&child.id),
            "not a child"
        );

        child.state.write().unwrap().observer = observer;
        child.relationships.write().unwrap().parent = Weak::new();

        self.relationships
            .write()
            .unwrap()
            .children
            .remove(&child.id);

        let state = self.state.read().unwrap();

        self.emit_update_event(&*state.observer);
    }

    /// Returns the associated unique ID.
    pub fn id(&self) -> ProgressId {
        self.id
    }

    /// Emits a message event with a priority level of `MessageLevel::Error`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn error<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<String>,
    {
        self.message(message, PriorityLevel::Error);
    }

    /// Emits a message event with a priority level of `MessageLevel::Warn`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn warn<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<String>,
    {
        self.message(message, PriorityLevel::Warn);
    }

    /// Emits a message event with a priority level of `MessageLevel::Debug`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn debug<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<String>,
    {
        self.message(message, PriorityLevel::Debug);
    }

    /// Emits a message event with a priority level of `MessageLevel::Info`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn info<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<String>,
    {
        self.message(message, PriorityLevel::Info);
    }

    /// Emits a message event with a priority level of `MessageLevel::Trace`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn trace<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<String>,
    {
        self.message(message, PriorityLevel::Trace);
    }

    /// Emits a message event with a priority level of `level`.
    ///
    /// # Filtering
    ///
    /// By default, `Progress` emits all message events with a minimum priority level of `trace`.
    ///
    /// The `SITREP_PRIORITY` environment variable controls filtering with the syntax:
    ///
    /// ```terminal
    /// SITREP_PRIORITY=[level]
    /// ```
    pub fn message<T>(self: &Arc<Self>, message: impl FnOnce() -> T, level: PriorityLevel)
    where
        T: Into<String>,
    {
        if level < self.min_priority_level() {
            return;
        }

        let state = self.state.read().unwrap();
        self.emit_message_event(&*state.observer, message().into(), level);
    }

    /// Overrides the global minimum priority level.
    ///
    /// # Global default
    ///
    /// By default the minimum priority level is `PriorityLevel::Trace`.
    ///
    /// # Global environment override
    ///
    /// The `SITREP_PRIORITY` environment variable allows for overriding with the syntax:
    ///
    /// ```terminal
    /// SITREP_PRIORITY=[level]
    /// ```
    ///
    /// where `level` is one of `[trace, debug, info, warn, error]`.
    pub fn set_min_priority_level(&self, level: Option<PriorityLevel>) {
        self.state.write().unwrap().min_priority_level = level;
    }

    fn min_priority_level(&self) -> PriorityLevel {
        self.state
            .read()
            .unwrap()
            .min_priority_level
            .unwrap_or_else(|| PriorityLevel::from_env().unwrap_or_default())
    }

    /// Sets the task's label to `label`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_label(self: &Arc<Self>, label: impl Into<Option<String>>) {
        self.update(|task| task.set_label(label));
    }

    /// Increments the task's completed unit count by `1`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn increment_completed(self: &Arc<Self>) {
        self.update(|task| task.increment_completed());
    }

    /// Increments the task's completed unit count by `increment`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn increment_completed_by(self: &Arc<Self>, increment: u32) {
        self.update(|task| task.increment_completed_by(increment));
    }

    /// Sets the task's completed unit count to `completed`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_completed(self: &Arc<Self>, completed: u32) {
        self.update(|task| task.set_completed(completed));
    }

    /// Sets the task's total unit count to `total`.
    ///
    /// A `self.total` of `0` results in an indeterminate task progress.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_total(self: &Arc<Self>, total: u32) {
        self.update(|task| task.set_total(total));
    }

    /// Sets the task's weight to `weight`.
    ///
    /// # Panics
    ///
    /// Panics if `weight <= 0.0`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_weight(self: &Arc<Self>, weight: f32) {
        self.update(|task| task.set_weight(weight));
    }

    /// Updates the associated task, emitting a corresponding event afterwards.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use this method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn update(self: &Arc<Self>, update_task: impl FnOnce(&mut Task)) {
        let guard = &mut self.state.write().unwrap();

        update_task(&mut guard.task);

        let next_generation = Generation(guard.max_generation.fetch_add(1, Ordering::Relaxed));

        if next_generation < guard.task.generation {
            guard.observer.observe(Event::GenerationOverflow);
        }

        guard.task.set_generation(next_generation);

        self.emit_update_event(&*guard.observer);
    }

    fn emit_message_event(
        self: &Arc<Self>,
        observer: &dyn Observer,
        message: String,
        level: PriorityLevel,
    ) {
        observer.observe(Event::Progress(ProgressEvent {
            id: self.id(),
            kind: ProgressEventKind::Message(MessageEvent { message, level }),
        }));
    }

    fn emit_update_event(self: &Arc<Self>, observer: &dyn Observer) {
        observer.observe(Event::Progress(ProgressEvent {
            id: self.id(),
            kind: ProgressEventKind::Update,
        }));
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.state
            .read()
            .unwrap()
            .observer
            .observe(Event::Progress(ProgressEvent {
                id: self.id(),
                kind: ProgressEventKind::Removed,
            }));
    }
}

impl Reporter for Progress {
    fn report(&self) -> Report {
        let task = &self.state.read().unwrap().task;

        let progress_id = self.id;
        let label = task.label.clone();
        let weight = task.weight;

        let subreports: Vec<_> = self
            .relationships
            .read()
            .unwrap()
            .children
            .values()
            .map(|progress| progress.report())
            .collect();

        let discrete = if subreports.is_empty() {
            task.discrete()
        } else {
            subreports
                .iter()
                .filter_map(|report| report.discrete)
                .reduce(|sum, item| (sum.0 + item.0, sum.1 + item.1))
        };

        let total_weight = subreports
            .iter()
            .filter(|&report| report.fraction.is_some())
            .map(|report| report.weight)
            .fold(0.0, |sum, item| sum + item);

        let fraction = if subreports.is_empty() {
            task.fraction()
        } else {
            subreports
                .iter()
                .filter_map(|report| {
                    report
                        .fraction
                        .map(|fraction| fraction * report.weight / total_weight)
                })
                .reduce(|sum, item| sum + item)
        };

        let generation = subreports
            .iter()
            .map(|report| report.generation)
            .fold(task.generation, |max, item| max.max(item));

        Report {
            progress_id,
            label,
            discrete,
            fraction,
            subreports,
            weight,
            generation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_auto_incrementing() {
        let ids: Vec<_> = (0..100).map(|_| ProgressId::new_unique()).collect();

        for window in ids.windows(2) {
            let [prev, next] = window else {
                panic!("expected window of size 2");
            };

            assert_eq!(prev.0 + 1, next.0);
        }
    }
}
