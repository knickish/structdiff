use std::time::Duration;

use criterion::{black_box, criterion_group, Criterion};
use nanorand::WyRand;
use structdiff::StructDiff;

use crate::TestBench;
const SAMPLE_SIZE: usize = 1000;
const MEASUREMENT_TIME: Duration = Duration::from_secs(25);
const SEED: u64 = 42;

#[cfg(feature = "compare")]
criterion_group!(
    benches,
    full,
    diff_struct_bench::bench,
    serde_diff_bench::bench
);
#[cfg(not(feature = "compare"))]
criterion_group!(benches, full);

const GROUP_NAME: &str = "large_full";

fn full(c: &mut Criterion) {
    const BENCH_NAME: &str = "owned";
    let mut rng = WyRand::new_seed(SEED);
    let mut first = black_box(TestBench::generate_random_large(&mut rng));

    let second = black_box(TestBench::generate_random_large(&mut rng));
    let mut diff: Vec<<TestBench as StructDiff>::Diff> = Vec::new();
    let mut group = c.benchmark_group(GROUP_NAME);
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME);
    group.bench_function(BENCH_NAME, |b| {
        b.iter(|| {
            diff = black_box(StructDiff::diff(&first, &second));
            black_box(first.apply_mut(diff.clone()));
        })
    });
    group.finish();
    first.assert_eq(second, &diff);
}

#[cfg(feature = "compare")]
mod diff_struct_bench {
    use super::{
        black_box, Criterion, TestBench, WyRand, GROUP_NAME, MEASUREMENT_TIME, SAMPLE_SIZE, SEED,
    };
    use diff::Diff;

    pub(super) fn bench(c: &mut Criterion) {
        const BENCH_NAME: &str = "diff_struct_full";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random_large(&mut rng));
        let second = black_box(TestBench::generate_random_large(&mut rng));
        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(BENCH_NAME, |b| {
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
    use super::{
        black_box, Criterion, TestBench, WyRand, GROUP_NAME, MEASUREMENT_TIME, SAMPLE_SIZE, SEED,
    };
    use bincode::Options;

    pub(super) fn bench(c: &mut Criterion) {
        const BENCH_NAME: &str = "serde_diff_full";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random_large(&mut rng));
        let second = black_box(TestBench::generate_random_large(&mut rng));
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let mut group = c.benchmark_group(GROUP_NAME);
        group
            .sample_size(SAMPLE_SIZE)
            .measurement_time(MEASUREMENT_TIME);
        group.bench_function(BENCH_NAME, |b| {
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
