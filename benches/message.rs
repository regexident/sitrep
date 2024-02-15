use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use sitrep::{
    test_utils::{make_hierarchy, make_stand_alone},
    PriorityLevel,
};

const THREAD_STEPS: [usize; 11] = [1, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const ITERATIONS: usize = 10_000;

pub fn stand_alone(c: &mut Criterion) {
    let mut group = c.benchmark_group("message(): stand-alone");
    for threads in THREAD_STEPS {
        group.throughput(Throughput::Elements(threads as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{threads} threads")),
            &threads,
            |b, &threads| {
                let (progress, _) = make_stand_alone(None);

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

pub fn hierarchical(c: &mut Criterion) {
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

criterion_group!(benches, stand_alone, hierarchical,);
criterion_main!(benches);
