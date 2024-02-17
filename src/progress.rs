//! A progress.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
};

use parking_lot::RwLock;

use crate::{
    event::Event,
    priority::{global_min_priority_level, AtomicPriorityLevel},
    report::Report,
    task::Task,
    Generation, MessageEvent, PriorityLevel, RemovalEvent, UpdateEvent,
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
}

/// The progress' atomic state.
struct ProgressAtomicState {
    /// The minimum priority level.
    min_priority_level: AtomicPriorityLevel,
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
    /// The progress' atomic state.
    atomic_state: ProgressAtomicState,
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
        let parent_state = parent.state.read();

        // Children share the observer of their parent:
        let observer = parent_state.observer.clone();
        // Children share the generation of their parent:
        let max_generation = Arc::clone(&parent_state.max_generation);

        let child = Self::new_impl(task, Arc::downgrade(parent), observer, max_generation);

        parent
            .relationships
            .write()
            .children
            .insert(child.id(), Arc::clone(&child));

        let parent_state = parent.state.read();

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

        let relationships = RwLock::new(ProgressRelationships { parent, children });

        let state = RwLock::new(ProgressState {
            task,
            observer,
            max_generation,
        });

        let min_priority_level = AtomicPriorityLevel::from(PriorityLevel::MIN);

        let atomic_state = ProgressAtomicState { min_priority_level };

        Arc::new(Self {
            id,
            relationships,
            state,
            atomic_state,
        })
    }

    /// Attaches `child` to `self`, returning the `child's` own and now no longer used `Observer`.
    pub fn attach_child(self: &Arc<Self>, child: &Arc<Self>) -> Arc<dyn Observer> {
        let parent_state = self.state.read();

        let max_generation = {
            let child_state = child.state.read();

            let parent_max_generation = parent_state.max_generation.load(Ordering::Relaxed);
            let child_max_generation = child_state.max_generation.load(Ordering::Relaxed);

            parent_max_generation.max(child_max_generation)
        };

        // Bump the parent's generation, if necessary:
        parent_state
            .max_generation
            .store(max_generation, Ordering::SeqCst);

        let mut child_state = child.state.write();

        // Children share the generation of their parent:
        child_state.max_generation = Arc::clone(&parent_state.max_generation);

        // Make sure the child uses the parent's observer from now on:
        let observer = {
            let parent_observer = parent_state.observer.clone();
            std::mem::replace(&mut child_state.observer, parent_observer)
        };

        self.relationships
            .write()
            .children
            .insert(child.id(), Arc::clone(child));

        self.emit_update_event(&*parent_state.observer);

        observer
    }

    /// Detaches `child` from `self`, giving it a new `observer`.
    pub fn detach_child(self: &Arc<Self>, child: &Arc<Self>, observer: Arc<dyn Observer>) {
        debug_assert!(
            self.relationships.read().children.contains_key(&child.id),
            "not a child"
        );

        child.state.write().observer = observer;
        child.relationships.write().parent = Weak::new();

        self.relationships.write().children.remove(&child.id);

        let state = self.state.read();

        self.emit_update_event(&*state.observer);
    }

    /// Returns the sub-progress with the given `id` within the tree, or `None` if it doesn't exist.
    pub fn get(self: &Arc<Self>, id: ProgressId) -> Option<Arc<Progress>> {
        if self.id == id {
            return Some(Arc::clone(self));
        }

        let children = &self.relationships.read().children;

        let child = children.get(&id);

        if child.is_some() {
            return child.cloned();
        }

        children.values().find_map(|progress| progress.get(id))
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

        let state = self.state.read();
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
        self.atomic_state
            .min_priority_level
            .store(level, Ordering::SeqCst)
    }

    /// Returns the effective minimum priority level.
    ///
    /// If no local level has been overridden it returns
    /// a fallback in the following order of precedence:
    ///
    /// - environment (i.e. `SITREP_PRIORITY=[level]`)
    /// - default (i.e. `PriorityLevel::Trace`)
    pub fn min_priority_level(&self) -> PriorityLevel {
        self.atomic_state
            .min_priority_level
            .load(Ordering::Relaxed)
            .unwrap_or_else(global_min_priority_level)
    }

    /// Sets the task's label to `label`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_label(self: &Arc<Self>, label: impl Into<Option<String>>) {
        self.update(|task| task.label = label.into());
    }

    /// Increments the task's completed unit count by `1`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn increment_completed(self: &Arc<Self>) {
        self.update(|task| task.completed += 1);
    }

    /// Increments the task's completed unit count by `increment`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn increment_completed_by(self: &Arc<Self>, increment: usize) {
        self.update(|task| task.completed += increment);
    }

    /// Sets the task's completed unit count to `completed`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_completed(self: &Arc<Self>, completed: usize) {
        self.update(|task| task.completed = completed);
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
    pub fn set_total(self: &Arc<Self>, total: usize) {
        self.update(|task| task.total = total);
    }

    /// Updates the associated task, emitting a corresponding event afterwards.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use this method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn update(self: &Arc<Self>, update_task: impl FnOnce(&mut Task)) {
        let guard = &mut self.state.write();

        update_task(&mut guard.task);

        let next_generation = Generation(guard.max_generation.fetch_add(1, Ordering::Relaxed));

        if next_generation < guard.task.generation {
            guard.observer.observe(Event::GenerationOverflow);
        }

        guard.task.generation = next_generation;

        self.emit_update_event(&*guard.observer);
    }

    fn emit_message_event(
        self: &Arc<Self>,
        observer: &dyn Observer,
        message: String,
        priority: PriorityLevel,
    ) {
        observer.observe(Event::Message(MessageEvent {
            id: self.id(),
            message,
            priority,
        }));
    }

    fn emit_update_event(self: &Arc<Self>, observer: &dyn Observer) {
        observer.observe(Event::Update(UpdateEvent { id: self.id() }));
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.state
            .read()
            .observer
            .observe(Event::Removed(RemovalEvent { id: self.id() }));
    }
}

impl Reporter for Progress {
    fn report(&self) -> Report {
        let task = &self.state.read().task;

        let progress_id = self.id;
        let label = task.label.clone();

        let subreports: Vec<_> = self
            .relationships
            .read()
            .children
            .values()
            .map(|progress| progress.report())
            .collect();

        let determinate_reports = subreports.iter().filter(|&report| !report.is_indeterminate);

        let (completed, total) = determinate_reports
            .map(|report| report.discrete())
            .fold(task.effective_discrete(), |sum, item| {
                (sum.0.saturating_add(item.0), sum.1.saturating_add(item.1))
            });

        let generation = subreports
            .iter()
            .map(|report| report.generation)
            .fold(task.generation, |max, item| max.max(item));

        Report::new(progress_id, label, completed, total, subreports, generation)
    }
}

#[doc(hidden)]
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;

    #[doc(hidden)]
    pub struct NopObserver;

    impl Observer for NopObserver {
        fn observe(&self, event: Event) {
            std::hint::black_box(event);
        }
    }

    #[doc(hidden)]
    pub fn make_stand_alone(
        observer: Option<Arc<dyn Observer>>,
    ) -> (Arc<Progress>, Weak<impl Reporter>) {
        let observer = observer.unwrap_or_else(|| Arc::new(NopObserver));
        Progress::new(Task::default(), observer)
    }

    #[doc(hidden)]
    pub fn make_hierarchy() -> (Arc<Vec<Arc<Progress>>>, Weak<impl Reporter>) {
        let (parent, reporter) = Progress::new(Task::default(), Arc::new(NopObserver));

        let mut progresses = vec![Arc::clone(&parent)];

        for _ in 1..=10 {
            let child = Progress::new_with_parent(Task::default(), &parent);
            progresses.push(Arc::clone(&child));

            for _ in 1..=10 {
                let grandchild = Progress::new_with_parent(Task::default(), &child);
                progresses.push(Arc::clone(&grandchild));
            }
        }

        let progresses = Arc::new(progresses);

        (progresses, reporter)
    }
}

#[cfg(test)]
mod tests;
