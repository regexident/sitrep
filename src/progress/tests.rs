use std::sync::Mutex;

use super::*;

struct NopObserver;

impl Observer for NopObserver {
    fn observe(&self, _event: Event) {}
}

#[derive(Default)]
struct SpyObserver {
    events: Mutex<Vec<Event>>,
}

impl SpyObserver {
    fn new() -> (Arc<Self>, Arc<dyn Observer>) {
        let observer = Arc::new(Self::default());
        let erased_observer = Arc::<Self>::clone(&observer);
        (observer, erased_observer)
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn events_len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    fn update_events(&self) -> Vec<UpdateEvent> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Update(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn message_events(&self) -> Vec<MessageEvent> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Message(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn detachment_events(&self) -> Vec<DetachmentEvent> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Detachment(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Observer for SpyObserver {
    fn observe(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn id_monotonically_increments() {
    let ids: Vec<_> = (0..1000).map(|_| ProgressId::new_unique()).collect();

    for window in ids.windows(2) {
        let [prev, next] = window else {
            panic!("expected window of size 2");
        };

        assert!(prev.0 < next.0);
    }
}

mod no_reference_cycles {
    use super::*;

    #[test]
    fn stand_alone() {
        let observer = Arc::new(NopObserver);

        let (stand_alone, weak_reporter) = Progress::new(Task::default(), observer);
        let weak_stand_alone = Arc::downgrade(&stand_alone);

        assert_eq!(Arc::strong_count(&stand_alone), 1);

        drop(stand_alone);

        assert_eq!(Weak::strong_count(&weak_stand_alone), 0);
        assert_eq!(Weak::strong_count(&weak_reporter), 0);
    }

    #[test]
    fn parent_with_children() {
        let observer = Arc::new(NopObserver);

        let (parent, weak_reporter) = Progress::new(Task::default(), observer);
        let weak_parent = Arc::downgrade(&parent);

        let children = [
            Progress::new_with_parent(Task::default(), &parent),
            Progress::new_with_parent(Task::default(), &parent),
            Progress::new_with_parent(Task::default(), &parent),
        ];
        let weak_children: Vec<_> = children.iter().map(Arc::downgrade).collect();

        assert_eq!(Arc::strong_count(&parent), 1);

        for child in children.iter() {
            assert_eq!(Arc::strong_count(child), 2);
        }

        drop(parent);

        for child in children {
            drop(child);
        }

        assert_eq!(Weak::strong_count(&weak_parent), 0);
        assert_eq!(Weak::strong_count(&weak_reporter), 0);

        for weak_child in weak_children {
            assert_eq!(Weak::strong_count(&weak_child), 0);
        }
    }

    #[test]
    fn parent_with_grandchildren() {
        let observer = Arc::new(NopObserver);

        let (parent, weak_reporter) = Progress::new(Task::default(), observer);
        let weak_parent = Arc::downgrade(&parent);

        let child = Progress::new_with_parent(Task::default(), &parent);
        let weak_child = Arc::downgrade(&child);

        let grandchildren = [
            Progress::new_with_parent(Task::default(), &child),
            Progress::new_with_parent(Task::default(), &child),
            Progress::new_with_parent(Task::default(), &child),
        ];
        let weak_grandchildren: Vec<_> = grandchildren.iter().map(Arc::downgrade).collect();

        assert_eq!(Arc::strong_count(&parent), 1);

        assert_eq!(Arc::strong_count(&child), 2);

        for grandchild in grandchildren.iter() {
            assert_eq!(Arc::strong_count(grandchild), 2);
        }

        drop(parent);

        drop(child);

        for grandchild in grandchildren {
            drop(grandchild);
        }

        assert_eq!(Weak::strong_count(&weak_parent), 0);
        assert_eq!(Weak::strong_count(&weak_reporter), 0);

        assert_eq!(Weak::strong_count(&weak_child), 0);

        for weak_grandchild in weak_grandchildren {
            assert_eq!(Weak::strong_count(&weak_grandchild), 0);
        }
    }
}

mod removal {
    use super::*;

    #[test]
    fn detach_from_parent() {
        let (observer, erased_observer) = SpyObserver::new();

        let (parent, _reporter) = Progress::new(Task::default(), erased_observer);
        let child = Progress::new_with_parent(Task::default(), &parent);

        child.detach_from_parent(Arc::new(NopObserver));

        assert_eq!(observer.detachment_events().len(), 1);
    }

    #[test]
    fn detach_child() {
        let (observer, erased_observer) = SpyObserver::new();

        let (parent, _reporter) = Progress::new(Task::default(), erased_observer);
        let child = Progress::new_with_parent(Task::default(), &parent);

        parent.detach_child(&child, Arc::new(NopObserver));

        assert_eq!(observer.detachment_events().len(), 1);
    }
}

mod message {
    use super::*;

    #[test]
    fn gets_filtered_by_min_priority_level() {
        let (observer, erased_observer) = SpyObserver::new();

        let (progress, _reporter) = Progress::new(Task::default(), erased_observer);

        progress.set_min_priority_level(Some(PriorityLevel::Warn));

        for level in PriorityLevel::ALL {
            progress.message(|| "test", level);
        }

        let expected_levels = [PriorityLevel::Warn, PriorityLevel::Error];

        for event in observer.message_events() {
            if !expected_levels.contains(&event.priority) {
                panic!("unexpected priority level: {:?}", event.priority);
            }
        }
    }

    #[test]
    fn gets_delivered() {
        let (observer, erased_observer) = SpyObserver::new();

        let (progress, _reporter) = Progress::new(Task::default(), erased_observer);

        let message = "test";

        for level in PriorityLevel::ALL {
            progress.message(|| message, level);
        }

        assert_eq!(observer.events_len(), PriorityLevel::ALL.len());

        let actual = observer.message_events();
        let expected: Vec<_> = PriorityLevel::ALL
            .into_iter()
            .map(|priority| MessageEvent {
                id: progress.id(),
                message: message.into(),
                priority,
            })
            .collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn stand_alone() {
        let (observer, erased_observer) = SpyObserver::new();

        let (progress, _reporter) = Progress::new(Task::default(), erased_observer);

        progress.message(|| "test", PriorityLevel::Error);

        assert_eq!(observer.message_events().len(), 1);
    }

    #[test]
    fn hierarchy() {
        let (observer, erased_observer) = SpyObserver::new();

        let (parent, _reporter) = Progress::new(Task::default(), erased_observer);
        let child = Progress::new_with_parent(Task::default(), &parent);
        let grandchild = Progress::new_with_parent(Task::default(), &child);

        parent.message(|| "test", PriorityLevel::Error);
        child.message(|| "test", PriorityLevel::Error);
        grandchild.message(|| "test", PriorityLevel::Error);

        assert_eq!(observer.message_events().len(), 3);
    }
}

mod update {
    use super::*;

    #[test]
    fn stand_alone() {
        let (observer, erased_observer) = SpyObserver::new();

        let (progress, _reporter) = Progress::new(Task::default(), erased_observer);

        progress.update(|_| {});

        assert_eq!(observer.update_events().len(), 1);
    }

    #[test]
    fn hierarchy() {
        let (observer, erased_observer) = SpyObserver::new();

        let (parent, _reporter) = Progress::new(Task::default(), erased_observer);
        assert_eq!(observer.update_events().len(), 0);
        let child = Progress::new_with_parent(Task::default(), &parent);
        assert_eq!(observer.update_events().len(), 1);
        let grandchild = Progress::new_with_parent(Task::default(), &child);
        assert_eq!(observer.update_events().len(), 2);

        parent.update(|_| {});
        child.update(|_| {});
        grandchild.update(|_| {});

        assert_eq!(observer.update_events().len(), 5);
    }
}

mod debug {
    use super::*;

    #[test]
    fn fmt() {
        let observer = Arc::new(NopObserver);

        let (progress, _) = Progress::new(Task::default(), observer);

        let id = ProgressId(0);
        let report = Report {
            progress_id: id,
            label: None,
            completed: 0,
            total: 0,
            fraction: 0.0,
            is_indeterminate: true,
            state: State::Running,
            subreports: vec![],
            last_change: Generation(0),
        };

        let actual = format!("{progress:?}");
        let expected =
            format!("Progress {{ id: {id:?}, parent: None, children: [], report: {report:?} }}");

        assert_eq!(actual, expected);
    }
}

mod report {
    use super::*;

    #[test]
    fn stand_alone() {
        let (_observer, erased_observer) = SpyObserver::new();

        let (progress, weak_reporter) = Progress::new(Task::default(), erased_observer);

        progress.update(|task| {
            task.label = Some("label".into());
            task.completed = 5;
            task.total = 10;
        });

        let reporter = weak_reporter.upgrade().unwrap();

        let report = reporter.report();

        assert_eq!(report.progress_id, progress.id);
        assert_eq!(report.label.unwrap(), "label");
        assert_eq!(report.completed, 5);
        assert_eq!(report.total, 10);
        assert_eq!(report.fraction, 0.5);
        assert_eq!(report.subreports, vec![]);
    }

    #[test]
    fn hierarchy() {
        let (_observer, erased_observer) = SpyObserver::new();

        let (parent, weak_reporter) = Progress::new(Task::default(), erased_observer);
        let child = Progress::new_with_parent(Task::default(), &parent);
        let grandchild = Progress::new_with_parent(Task::default(), &child);

        parent.update(|task| {
            task.label = Some("parent".into());
            task.completed = 1;
            task.total = 2;
        });

        child.update(|task| {
            task.label = Some("child".into());
            task.completed = 1;
            task.total = 2;
        });

        grandchild.update(|task| {
            task.label = Some("grandchild".into());
            task.completed = 1;
            task.total = 2;
        });

        let reporter = weak_reporter.upgrade().unwrap();

        let parent_report = reporter.report();

        assert_eq!(parent_report.progress_id, parent.id);
        assert_eq!(parent_report.label.unwrap(), "parent");
        assert_eq!(parent_report.completed, 3);
        assert_eq!(parent_report.total, 6);
        assert_eq!(parent_report.fraction, 0.5);
        assert_eq!(parent_report.subreports.len(), 1);

        let child_report = parent_report.subreports[0].clone();

        assert_eq!(child_report.progress_id, child.id);
        assert_eq!(child_report.label.unwrap(), "child");
        assert_eq!(child_report.completed, 2);
        assert_eq!(child_report.total, 4);
        assert_eq!(child_report.fraction, 0.5);
        assert_eq!(child_report.subreports.len(), 1);

        let grandchild_report = child_report.subreports[0].clone();

        assert_eq!(grandchild_report.progress_id, grandchild.id);
        assert_eq!(grandchild_report.label.unwrap(), "grandchild");
        assert_eq!(grandchild_report.completed, 1);
        assert_eq!(grandchild_report.total, 2);
        assert_eq!(grandchild_report.fraction, 0.5);
        assert_eq!(grandchild_report.subreports.len(), 0);
    }
}

#[test]
fn get() {
    let (_observer, erased_observer) = SpyObserver::new();

    let (parent, _weak_reporter) = Progress::new(Task::default(), erased_observer);
    let child = Progress::new_with_parent(Task::default(), &parent);
    let grandchild = Progress::new_with_parent(Task::default(), &child);

    let missing_id = ProgressId(42);

    assert!(parent.get(missing_id).is_none());
    assert_eq!(parent.get(parent.id).unwrap().id, parent.id);
    assert_eq!(parent.get(child.id).unwrap().id, child.id);
    assert_eq!(parent.get(grandchild.id).unwrap().id, grandchild.id);

    assert!(child.get(missing_id).is_none());
    assert!(child.get(parent.id).is_none());
    assert_eq!(child.get(child.id).unwrap().id, child.id);
    assert_eq!(child.get(grandchild.id).unwrap().id, grandchild.id);

    assert!(grandchild.get(missing_id).is_none());
    assert!(grandchild.get(parent.id).is_none());
    assert!(grandchild.get(child.id).is_none());
    assert_eq!(grandchild.get(grandchild.id).unwrap().id, grandchild.id);
}
