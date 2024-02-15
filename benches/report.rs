use criterion::{black_box, criterion_group, criterion_main, Criterion};

use sitrep::{
    test_utils::{make_hierarchy, make_stand_alone},
    Reporter, Task,
};

const ITERATIONS: usize = 10_000;

pub fn stand_alone(c: &mut Criterion) {
    c.bench_function("report(): stand-alone", |b| {
        let (progress, reporter) = make_stand_alone(None);

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

pub fn hierarchical(c: &mut Criterion) {
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

criterion_group!(benches, stand_alone, hierarchical);
criterion_main!(benches);
