use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use std::sync::{Arc, Weak};

use sitrep::{Event, Observer, PriorityLevel, Progress, Reporter, Task};

struct NopObserver;

impl Observer for NopObserver {
    fn observe(&self, event: Event) {
        black_box(event);
    }
}

const THREAD_STEPS: [usize; 11] = [1, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const ITERATIONS: usize = 10_000;

fn make_stand_alone() -> (Arc<Progress>, Weak<impl Reporter>) {
    Progress::new(Task::default(), Arc::new(NopObserver))
}

fn make_hierarchy() -> (Arc<Vec<Arc<Progress>>>, Weak<impl Reporter>) {
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

pub fn message_stand_alone(c: &mut Criterion) {
    let mut group = c.benchmark_group("message(): stand-alone");
    for threads in THREAD_STEPS {
        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{threads} threads")),
            &threads,
            |b, &threads| {
                let (progress, _) = make_stand_alone();

                b.iter(|| {
                    let mut handles = vec![];

                    for _ in 1..=threads {
                        let progress = Arc::clone(&progress);

                        let handle = std::thread::spawn(move || {
                            for _ in 0..(ITERATIONS / threads) {
                                progress.message(|| "test", PriorityLevel::Error);
                            }
                        });

                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });

                drop(progress);
            },
        );
    }
    group.finish();
}

pub fn message_hierarchical(c: &mut Criterion) {
    let mut group = c.benchmark_group("message(): hierarchical");
    for threads in THREAD_STEPS {
        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{threads} threads")),
            &threads,
            |b, &threads| {
                let (progresses, _) = make_hierarchy();

                b.iter(|| {
                    let mut handles = vec![];

                    for t in 1..=threads {
                        let progresses = Arc::clone(&progresses);

                        let handle = std::thread::spawn(move || {
                            for i in 0..(ITERATIONS / threads) {
                                // Poor man's deterministic pseudo-random sample using a prime-number:
                                let idx = (t * i * 13) % progresses.len();

                                progresses[idx].message(|| "test", PriorityLevel::Error);
                            }
                        });

                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });

                drop(progresses);
            },
        );
    }
    group.finish();
}

pub fn update_stand_alone(c: &mut Criterion) {
    let mut group = c.benchmark_group("update(): stand-alone");
    for threads in THREAD_STEPS {
        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{threads} threads")),
            &threads,
            |b, &threads| {
                let (progress, _) = make_stand_alone();

                b.iter(|| {
                    let mut handles = vec![];

                    for _ in 1..=threads {
                        let progress = Arc::clone(&progress);

                        let handle = std::thread::spawn(move || {
                            for _ in 0..(ITERATIONS / threads) {
                                progress.update(|_| ());
                            }
                        });

                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });

                drop(progress);
            },
        );
    }
    group.finish();
}

pub fn update_hierarchical(c: &mut Criterion) {
    let mut group = c.benchmark_group("update(): hierarchical");
    for threads in THREAD_STEPS {
        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{threads} threads")),
            &threads,
            |b, &threads| {
                let (progresses, _) = make_hierarchy();

                b.iter(|| {
                    let mut handles = vec![];

                    for t in 1..=threads {
                        let progresses = Arc::clone(&progresses);

                        let handle = std::thread::spawn(move || {
                            for i in 0..(ITERATIONS / threads) {
                                // Poor man's deterministic pseudo-random sample using a prime-number:
                                let idx = (t * i * 13) % progresses.len();

                                progresses[idx].update(|_| ());
                            }
                        });

                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });

                drop(progresses);
            },
        );
    }
    group.finish();
}

pub fn report_stand_alone(c: &mut Criterion) {
    c.bench_function("report(): stand-alone", |b| {
        let (progress, reporter) = make_stand_alone();

        // Make sure we actually have stuff to compute for the report:
        progress.update(|task: &mut Task| {
            task.set_label("label".to_owned());
            task.set_completed(1);
            task.set_total(10);
        });

        let reporter = reporter.upgrade().unwrap();

        b.iter(|| {
            for _ in 0..ITERATIONS {
                black_box(reporter.report());
            }
        });

        drop(progress);
    });
}

pub fn report_hierarchical(c: &mut Criterion) {
    c.bench_function("report(): hierarchical", |b| {
        let (progresses, reporter) = make_hierarchy();

        // Make sure we actually have stuff to compute for the report:
        for progress in progresses.iter() {
            progress.update(|task: &mut Task| {
                task.set_label("label".to_owned());
                task.set_completed(1);
                task.set_total(10);
            });
        }

        let reporter = reporter.upgrade().unwrap();

        b.iter(|| {
            for _ in 0..ITERATIONS {
                black_box(reporter.report());
            }
        });

        drop(progresses);
    });
}

criterion_group!(
    benches,
    message_stand_alone,
    message_hierarchical,
    update_stand_alone,
    update_hierarchical,
    report_stand_alone,
    report_hierarchical
);
criterion_main!(benches);
