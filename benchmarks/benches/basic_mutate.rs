#![cfg(test)]

extern crate structdiff_benchmarks;

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nanorand::WyRand;
use structdiff::StructDiff;
use structdiff_benchmarks::TestBench;
const SAMPLE_SIZE: usize = 1000;
const MEASUREMENT_TIME: Duration = Duration::from_secs(25);
const SEED: u64 = 42;

#[cfg(feature = "compare")]
criterion_group!(
    benches,
    bench_basic_mutate_generation,
    bench_basic_mutate_full,
    diff_struct_bench::bench_basic_mutate,
    serde_diff_bench::bench_basic_mutate
);
#[cfg(not(feature = "compare"))]
criterion_group!(
    benches,
    bench_basic_mutate_generation,
    bench_basic_mutate_full
);

criterion_main!(benches);

fn bench_basic_mutate_generation(c: &mut Criterion) {
    const GROUP_NAME: &str = "bench_basic_mutate_ref_gen";
    let mut rng = WyRand::new_seed(SEED);
    let first = black_box(TestBench::generate_random(&mut rng));
    let second = black_box(first.clone().random_mutate(&mut rng));
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function(GROUP_NAME, |b| {
        b.iter(|| {
            black_box(StructDiff::diff_ref(&first, &second));
        })
    });
    group.finish();
}

fn bench_basic_mutate_full(c: &mut Criterion) {
    const GROUP_NAME: &str = "bench_basic_mutate_owned";
    let mut rng = WyRand::new_seed(SEED);
    let mut first = black_box(TestBench::generate_random(&mut rng));
    let second = black_box(first.clone().random_mutate(&mut rng));
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function(GROUP_NAME, |b| {
        b.iter(|| {
            let diff = black_box(StructDiff::diff(&first, &second));
            black_box(first.apply_mut(diff));
        })
    });
    group.finish();
}

#[cfg(feature = "compare")]
mod diff_struct_bench {
    use super::{black_box, Criterion, TestBench, WyRand, MEASUREMENT_TIME, SAMPLE_SIZE, SEED};
    use diff::Diff;

    pub(super) fn bench_basic_mutate(c: &mut Criterion) {
        const GROUP_NAME: &str = "diff_struct_bench_basic_mutate";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random(&mut rng));
        let second = black_box(first.clone().random_mutate(&mut rng));
        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(GROUP_NAME, |b| {
            b.iter(|| {
                let diff = black_box(Diff::diff(&first, &second));
                black_box(Diff::apply(&mut first, &diff))
            })
        });
        group.finish();
        assert_eq!(first.b, second.b);
    }
}

#[cfg(feature = "compare")]
mod serde_diff_bench {
    use super::{black_box, Criterion, TestBench, WyRand, MEASUREMENT_TIME, SAMPLE_SIZE, SEED};
    use bincode::Options;

    pub(super) fn bench_basic_mutate(c: &mut Criterion) {
        const GROUP_NAME: &str = "serde_diff_bench_basic_mutate";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random(&mut rng));
        let second = black_box(first.clone().random_mutate(&mut rng));
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(GROUP_NAME, |b| {
            b.iter(|| {
                let mut diff = black_box(
                    options
                        .serialize(&serde_diff::Diff::serializable(&first, &second))
                        .unwrap(),
                );
                let mut deserializer =
                    black_box(bincode::Deserializer::from_slice(&mut diff[..], options));
                serde_diff::Apply::apply(&mut deserializer, &mut first).unwrap();
            })
        });
        group.finish();
        assert_eq!(first.b, second.b);
    }
}
