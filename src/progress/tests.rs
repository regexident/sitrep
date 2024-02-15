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

    fn progress_events(&self) -> Vec<ProgressEvent> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Progress(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn message_events(&self) -> Vec<(ProgressId, MessageEvent)> {
        self.progress_events()
            .into_iter()
            .filter_map(|event| match event.kind {
                ProgressEventKind::Message(message_event) => {
                    Some((event.id, message_event.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn update_events_len(&self) -> usize {
        self.progress_events()
            .iter()
            .filter(|event| matches!(event.kind, ProgressEventKind::Update))
            .count()
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

        for (_id, event) in observer.message_events() {
            if !expected_levels.contains(&event.level) {
                panic!("unexpected level: {:?}", event.level);
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
            .map(|level| {
                (
                    progress.id(),
                    MessageEvent {
                        message: message.to_owned(),
                        level,
                    },
                )
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

        assert_eq!(observer.update_events_len(), 1);
    }

    #[test]
    fn hierarchy() {
        let (observer, erased_observer) = SpyObserver::new();

        let (parent, _reporter) = Progress::new(Task::default(), erased_observer);
        assert_eq!(observer.update_events_len(), 0);
        let child = Progress::new_with_parent(Task::default(), &parent);
        assert_eq!(observer.update_events_len(), 1);
        let grandchild = Progress::new_with_parent(Task::default(), &child);
        assert_eq!(observer.update_events_len(), 2);

        parent.update(|_| {});
        child.update(|_| {});
        grandchild.update(|_| {});

        assert_eq!(observer.update_events_len(), 5);
    }
}

mod report {
    use super::*;

    #[test]
    fn stand_alone() {
        let (_observer, erased_observer) = SpyObserver::new();

        let (progress, weak_reporter) = Progress::new(Task::default(), erased_observer);

        progress.update(|task| {
            task.label = Some("label".to_owned());
            task.completed = 5;
            task.total = 10;
        });

        let reporter = weak_reporter.upgrade().unwrap();

        let report = reporter.report();

        assert_eq!(report.progress_id, progress.id);
        assert_eq!(report.label.unwrap(), "label".to_owned());
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
            task.label = Some("parent".to_owned());
            task.completed = 1;
            task.total = 2;
        });

        child.update(|task| {
            task.label = Some("child".to_owned());
            task.completed = 1;
            task.total = 2;
        });

        grandchild.update(|task| {
            task.label = Some("grandchild".to_owned());
            task.completed = 1;
            task.total = 2;
        });

        let reporter = weak_reporter.upgrade().unwrap();

        let parent_report = reporter.report();

        assert_eq!(parent_report.progress_id, parent.id);
        assert_eq!(parent_report.label.unwrap(), "parent".to_owned());
        assert_eq!(parent_report.completed, 3);
        assert_eq!(parent_report.total, 6);
        assert_eq!(parent_report.fraction, 0.5);
        assert_eq!(parent_report.subreports.len(), 1);

        let child_report = parent_report.subreports[0].clone();

        assert_eq!(child_report.progress_id, child.id);
        assert_eq!(child_report.label.unwrap(), "child".to_owned());
        assert_eq!(child_report.completed, 2);
        assert_eq!(child_report.total, 4);
        assert_eq!(child_report.fraction, 0.5);
        assert_eq!(child_report.subreports.len(), 1);

        let grandchild_report = child_report.subreports[0].clone();

        assert_eq!(grandchild_report.progress_id, grandchild.id);
        assert_eq!(grandchild_report.label.unwrap(), "grandchild".to_owned());
        assert_eq!(grandchild_report.completed, 1);
        assert_eq!(grandchild_report.total, 2);
        assert_eq!(grandchild_report.fraction, 0.5);
        assert_eq!(grandchild_report.subreports.len(), 0);
    }
}
