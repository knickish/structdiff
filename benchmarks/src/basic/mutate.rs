use std::time::Duration;

use criterion::{black_box, criterion_group, BatchSize, Criterion};
use nanorand::WyRand;
use structdiff::StructDiff;

use crate::TestBench;

const SAMPLE_SIZE: usize = 1000;
const MEASUREMENT_TIME: Duration = Duration::from_secs(25);
const SEED: u64 = 42;

#[cfg(feature = "compare")]
criterion_group!(
    benches,
    mutate,
    diff_struct_bench::mutate,
    serde_diff_bench::mutate
);
#[cfg(not(feature = "compare"))]
criterion_group!(benches, mutate);

const GROUP_NAME: &str = "mutate";

fn mutate(c: &mut Criterion) {
    const BENCH_NAME: &str = "mutate";
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function(BENCH_NAME, |b| {
        b.iter_batched(
            || {
                let mut rng = WyRand::new_seed(SEED);
                let first = TestBench::generate_random(&mut rng);
                let second = first.clone().random_mutate(&mut rng);
                (first, second)
            },
            |(first, second)| {
                let diff = black_box(StructDiff::diff(&first, &second));
                black_box(StructDiff::apply(first, diff));
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

#[cfg(feature = "compare")]
mod diff_struct_bench {
    use super::{
        black_box, Criterion, TestBench, WyRand, GROUP_NAME, MEASUREMENT_TIME, SAMPLE_SIZE, SEED,
    };
    use criterion::BatchSize;
    use diff::Diff;

    pub(super) fn mutate(c: &mut Criterion) {
        const BENCH_NAME: &str = "diff_struct_mutate";

        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(BENCH_NAME, |b| {
            b.iter_batched(
                || {
                    let mut rng = WyRand::new_seed(SEED);
                    let first = black_box(TestBench::generate_random(&mut rng));
                    let second = black_box(first.clone().random_mutate(&mut rng));
                    (first, second)
                },
                |(mut first, second)| {
                    let diff = black_box(Diff::diff(&first, &second));
                    black_box(Diff::apply(&mut first, &diff))
                },
                BatchSize::LargeInput,
            )
        });
        group.finish();
    }
}

#[cfg(feature = "compare")]
mod serde_diff_bench {
    use super::{
        black_box, Criterion, TestBench, WyRand, GROUP_NAME, MEASUREMENT_TIME, SAMPLE_SIZE, SEED,
    };
    use bincode::Options;
    use criterion::BatchSize;

    pub(super) fn mutate(c: &mut Criterion) {
        const BENCH_NAME: &str = "serde_diff_mutate";
        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(BENCH_NAME, |b| {
            b.iter_batched(
                || {
                    let mut rng = WyRand::new_seed(SEED);
                    let first = black_box(TestBench::generate_random(&mut rng));
                    let second = black_box(first.clone().random_mutate(&mut rng));
                    let options = bincode::DefaultOptions::new()
                        .with_fixint_encoding()
                        .allow_trailing_bytes();
                    (first, second, options)
                },
                |(mut first, second, options)| {
                    let mut diff = black_box(
                        options
                            .serialize(&serde_diff::Diff::serializable(&first, &second))
                            .unwrap(),
                    );
                    let mut deserializer =
                        black_box(bincode::Deserializer::from_slice(&mut diff[..], options));
                    serde_diff::Apply::apply(&mut deserializer, &mut first).unwrap();
                },
                BatchSize::LargeInput,
            )
        });
        group.finish();
    }
}
