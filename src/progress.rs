//! A progress.

use std::{
    borrow::Cow,
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
    task::{State, Task},
    DetachmentEvent, Generation, MessageEvent, PriorityLevel, UpdateEvent,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A progress' unique identifier.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ProgressId(pub(crate) usize);

impl Default for ProgressId {
    fn default() -> Self {
        Self::new_unique()
    }
}

impl ProgressId {
    pub(crate) fn new_unique() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }

    /// Returns the raw internal identifier value.
    pub fn as_raw(&self) -> usize {
        self.0
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
    fn report(self: &Arc<Self>) -> Report;
}

/// Types for controlling progress-tracked tasks.
pub trait Controller: Send + Sync {
    /// Returns the sub-progress with the given `id` within the tree,
    /// or `None` if it doesn't exist.
    fn get(self: &Arc<Self>, progress_id: ProgressId) -> Option<Arc<Self>>;

    /// Returns `true` if the task is cancelable, otherwise `false`.
    fn is_cancelable(self: &Arc<Self>) -> bool;

    /// Returns `true` if the task is pausable, otherwise `false`.
    fn is_pausable(self: &Arc<Self>) -> bool;

    /// Returns `true` if the task is canceled, otherwise `false`.
    fn is_canceled(self: &Arc<Self>) -> bool;

    /// Returns `true` if the task is paused, otherwise `false`.
    fn is_paused(self: &Arc<Self>) -> bool;

    /// Sets the state of the corresponding `Progress` task
    /// (and all its running sub-tasks) to `Paused`, recursively.
    fn pause(self: &Arc<Self>);

    /// Sets the state of the corresponding `Progress` task
    /// (and all its paused sub-tasks) to `Running`, recursively.
    fn resume(self: &Arc<Self>);

    /// Sets the state of the corresponding `Progress` task
    /// (and all its running/paused sub-tasks) to `Canceled`, recursively.
    fn cancel(self: &Arc<Self>);
}

/// The progress' state.
struct ProgressState {
    /// An associated task.
    task: Task,
    /// The progress tree's `Observer`.
    ///
    /// All progresses in a progress tree share the same observer.
    observer: Arc<dyn Observer>,
    /// An atomic counter for obtaining the tree's last change generation.
    ///
    /// All progresses in a progress tree share the same counter.
    last_tree_change: Arc<AtomicUsize>,
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
    pub fn new(
        task: Task,
        observer: Arc<dyn Observer>,
    ) -> (Arc<Self>, Weak<impl Reporter + Controller>) {
        let parent = Weak::new();
        let last_tree_change = Arc::new(AtomicUsize::default());

        let progress = Self::new_impl(task, parent, observer, last_tree_change);
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
        let last_tree_change = Arc::clone(&parent_state.last_tree_change);

        let child = Self::new_impl(task, Arc::downgrade(parent), observer, last_tree_change);

        parent
            .relationships
            .write()
            .children
            .insert(child.id(), Arc::clone(&child));

        let parent_state = parent.state.read();

        parent.emit_update_event(&*parent_state.observer, parent.id);

        child
    }

    fn new_impl(
        task: Task,
        parent: Weak<Self>,
        observer: Arc<dyn Observer>,
        last_tree_change: Arc<AtomicUsize>,
    ) -> Arc<Self> {
        let id = ProgressId::new_unique();
        let parent = parent;
        let children = HashMap::new();

        let relationships = RwLock::new(ProgressRelationships { parent, children });

        let state = RwLock::new(ProgressState {
            task,
            observer,
            last_tree_change,
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
        let last_tree_change = {
            let parent_last_tree_change =
                self.state.read().last_tree_change.load(Ordering::Relaxed);
            let child_last_tree_change =
                child.state.read().last_tree_change.load(Ordering::Relaxed);

            parent_last_tree_change.max(child_last_tree_change)
        };

        // Bump the parent's generation, if necessary:
        self.state
            .read()
            .last_tree_change
            .store(last_tree_change, Ordering::SeqCst);

        let observer = {
            let parent_state = self.state.read();
            let mut child_state = child.state.write();

            // Children share the generation of their parent:
            child_state.last_tree_change = Arc::clone(&self.state.read().last_tree_change);

            // Make sure the child uses the parent's observer from now on:
            std::mem::replace(&mut child_state.observer, parent_state.observer.clone())
        };

        self.relationships
            .write()
            .children
            .insert(child.id(), Arc::clone(child));

        self.bump_last_change();

        self.emit_update_event(&*self.state.read().observer, self.id);

        observer
    }

    /// Detaches `child` from `self`, giving it a new `observer`.
    pub fn detach_child(self: &Arc<Self>, child: &Arc<Self>, observer: Arc<dyn Observer>) {
        assert!(
            self.relationships.read().children.contains_key(&child.id),
            "not a child"
        );

        child.detach_from_parent(observer);
    }

    /// Detaches `self` from its parent, giving it a new `observer`.
    pub fn detach_from_parent(self: &Arc<Self>, observer: Arc<dyn Observer>) {
        let Some(parent) = self.relationships.read().parent.upgrade() else {
            return;
        };

        self.state.write().observer = observer;
        self.relationships.write().parent = Weak::new();

        parent.relationships.write().children.remove(&self.id);

        parent.bump_last_change();

        let state = parent.state.read();

        parent.emit_removed_event(&*state.observer, self.id);
        parent.emit_update_event(&*state.observer, parent.id);
    }

    /// Returns the progress' parent, or `None` if `self` has no parent.
    pub fn parent(self: &Arc<Self>) -> Option<Arc<Self>> {
        self.relationships.read().parent.upgrade()
    }

    /// Returns the progress' children.
    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> {
        self.relationships
            .read()
            .children
            .values()
            .map(Arc::clone)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Returns the child with the given `id` within the tree, or `None` if it doesn't exist.
    pub fn child(self: &Arc<Self>, id: ProgressId) -> Option<Arc<Progress>> {
        self.relationships.read().children.get(&id).cloned()
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
        T: Into<Cow<'static, str>>,
    {
        self.message(message, PriorityLevel::Error);
    }

    /// Emits a message event with a priority level of `MessageLevel::Warn`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn warn<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.message(message, PriorityLevel::Warn);
    }

    /// Emits a message event with a priority level of `MessageLevel::Debug`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn debug<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.message(message, PriorityLevel::Debug);
    }

    /// Emits a message event with a priority level of `MessageLevel::Info`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn info<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.message(message, PriorityLevel::Info);
    }

    /// Emits a message event with a priority level of `MessageLevel::Trace`.
    ///
    /// See [`message()`](method@message) for more info (e.g. filtering).
    pub fn trace<T>(self: &Arc<Self>, message: impl FnOnce() -> T)
    where
        T: Into<Cow<'static, str>>,
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
        T: Into<Cow<'static, str>>,
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
    pub fn set_label(self: &Arc<Self>, label: impl Into<Option<Cow<'static, str>>>) {
        self.update(|task| task.label = label.into());
    }

    /// Returns the task's label to `label`.
    pub fn label(self: &Arc<Self>) -> Option<Cow<'static, str>> {
        self.state.read().task.label.clone()
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

    /// Returns the task's completed unit count.
    pub fn completed(self: &Arc<Self>) -> usize {
        self.state.read().task.completed
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

    /// Returns the task's total unit count.
    pub fn total(self: &Arc<Self>) -> usize {
        self.state.read().task.total
    }

    /// Sets the task's state to `state`.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_state(self: &Arc<Self>, state: State) {
        self.update(|task| task.state = state);
    }

    /// Returns the task's state.
    pub fn state(self: &Arc<Self>) -> State {
        self.state.read().task.state
    }

    /// Sets whether or not the task is cancelable.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_cancelable(self: &Arc<Self>, cancelable: bool) {
        self.update(|task| task.is_cancelable = cancelable);
    }

    /// Sets whether or not the task is pausable.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use the `update(…)` method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn set_pausable(self: &Arc<Self>, pausable: bool) {
        self.update(|task| task.is_pausable = pausable);
    }

    /// Updates the associated task, emitting a corresponding event afterwards.
    ///
    /// # Performance
    ///
    /// When making multiple changes prefer to use this method over multiple
    /// individual calls to setters as those would emit one event per setter call,
    /// while `progress.update(|task| … )` only emits a single event at the very end.
    pub fn update(self: &Arc<Self>, update_task: impl FnOnce(&mut Task)) {
        update_task(&mut self.state.write().task);

        self.bump_last_change();

        self.emit_update_event(&*self.state.read().observer, self.id);
    }

    fn bump_last_change(self: &Arc<Self>) {
        let guard = &mut self.state.write();

        let latest_change = Generation(guard.last_tree_change.fetch_add(1, Ordering::Relaxed));

        if latest_change < guard.task.last_change {
            guard.observer.observe(Event::GenerationOverflow);
        }

        guard.task.last_change = latest_change;
    }

    fn emit_message_event(
        self: &Arc<Self>,
        observer: &dyn Observer,
        message: Cow<'static, str>,
        priority: PriorityLevel,
    ) {
        observer.observe(Event::Message(MessageEvent {
            id: self.id(),
            message,
            priority,
        }));
    }

    fn emit_update_event(self: &Arc<Self>, observer: &dyn Observer, id: ProgressId) {
        observer.observe(Event::Update(UpdateEvent { id }));
    }

    fn emit_removed_event(self: &Arc<Self>, observer: &dyn Observer, id: ProgressId) {
        observer.observe(Event::Detachment(DetachmentEvent { id }));
    }
}

impl Reporter for Progress {
    fn report(self: &Arc<Self>) -> Report {
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

        let state = task.state;

        let last_change = subreports
            .iter()
            .map(|report| report.last_change)
            .fold(task.last_change, |max, item| max.max(item));

        Report::new(
            progress_id,
            label,
            completed,
            total,
            state,
            subreports,
            last_change,
        )
    }
}

impl Controller for Progress {
    fn get(self: &Arc<Self>, progress_id: ProgressId) -> Option<Arc<Self>> {
        if self.id == progress_id {
            return Some(Arc::clone(self));
        }

        let children = &self.relationships.read().children;

        let child = children.get(&progress_id);

        if child.is_some() {
            return child.cloned();
        }

        children
            .values()
            .find_map(|progress| progress.get(progress_id))
    }

    fn is_cancelable(self: &Arc<Self>) -> bool {
        self.state.read().task.is_cancelable
    }

    fn is_pausable(self: &Arc<Self>) -> bool {
        self.state.read().task.is_pausable
    }

    fn is_canceled(self: &Arc<Self>) -> bool {
        self.state.read().task.state == State::Canceled
    }

    fn is_paused(self: &Arc<Self>) -> bool {
        self.state.read().task.state == State::Paused
    }

    fn pause(self: &Arc<Self>) {
        if !self.is_pausable() {
            panic!("not pausable");
        }

        let guard = &mut self.state.write();

        if guard.task.state == State::Running {
            guard.task.state = State::Paused;
        }

        for child in self.relationships.read().children.values() {
            child.pause();
        }
    }

    fn resume(self: &Arc<Self>) {
        if !self.is_pausable() {
            panic!("not resumable");
        }

        let guard = &mut self.state.write();

        if guard.task.state == State::Paused {
            guard.task.state = State::Running;
        }

        for child in self.relationships.read().children.values() {
            child.resume();
        }
    }

    fn cancel(self: &Arc<Self>) {
        if !self.is_cancelable() {
            panic!("not cancelable");
        }

        let guard = &mut self.state.write();

        if [State::Paused, State::Running].contains(&guard.task.state) {
            guard.task.state = State::Canceled;
        }

        for child in self.relationships.read().children.values() {
            child.cancel();
        }
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
