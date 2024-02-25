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
            task.label = Some("label".into());
            task.completed = 1;
            task.total = 10;
        });

        let reporter = reporter.upgrade().unwrap();

        b.iter(|| {
            for _ in 0..ITERATIONS {
                progress.update(|_| ());

                let report = reporter.report();

                black_box(report);
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
                task.label = Some("label".into());
                task.completed = 1;
                task.total = 10;
            });
        }

        let reporter = reporter.upgrade().unwrap();

        b.iter(|| {
            for i in 0..ITERATIONS {
                // Poor man's deterministic pseudo-random sample using a prime-number:
                let idx = (i * 13) % progresses.len();

                progresses[idx].update(|_| ());

                let report = reporter.report();

                black_box(report);
            }
        });

        drop(progresses);
    });
}

criterion_group!(benches, stand_alone, hierarchical);
criterion_main!(benches);
