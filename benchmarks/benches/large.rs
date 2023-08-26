#![cfg(test)]

extern crate structdiff_benchmarks;

use std::time::Duration;

use nanorand::WyRand;
use structdiff::StructDiff;
use structdiff_benchmarks::TestBench;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
const SAMPLE_SIZE: usize = 500;
const MEASUREMENT_TIME: Duration = Duration::from_secs(40);
const SEED: u64 = 42;

#[cfg(feature = "compare")]
criterion_group!(benches, bench_large, diff_struct_bench::bench_large, serde_diff_bench::bench_large);
#[cfg(not(feature = "compare"))]
criterion_group!(benches, bench_large);

criterion_main!(benches);

fn bench_large(c: &mut Criterion) {
    const GROUP_NAME: &str = "bench_large";
    let mut rng = WyRand::new_seed(SEED);
    let mut first = black_box(TestBench::generate_random_large(&mut rng));
    let second = black_box(TestBench::generate_random_large(&mut rng));
    let mut group = c.benchmark_group(GROUP_NAME);
    group.sample_size(SAMPLE_SIZE).measurement_time(MEASUREMENT_TIME);
    group.bench_function(GROUP_NAME,|b| b.iter(|| {
        let diff = black_box(StructDiff::diff(&first, &second));
        black_box(first.apply_mut(diff));
    }));
    group.finish();
    assert_eq!(first.b, second.b);
}

#[cfg(feature = "compare")]
mod diff_struct_bench {
    use super::{Criterion, black_box, TestBench, WyRand, SAMPLE_SIZE, MEASUREMENT_TIME, SEED};
    use diff::Diff;

    pub(super) fn bench_large(c: &mut Criterion) {
        const GROUP_NAME: &str = "diff_struct_bench_large";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random_large(&mut rng));
        let second = black_box(TestBench::generate_random_large(&mut rng));
        let mut group = c.benchmark_group(GROUP_NAME);
        group.sample_size(SAMPLE_SIZE).measurement_time(MEASUREMENT_TIME);
        group.bench_function(GROUP_NAME,|b| b.iter(|| {
            let diff = black_box(Diff::diff(&first, &second));
            black_box(Diff::apply(&mut first, &diff))
        }));
        group.finish();
        assert_eq!(first.b, second.b);
    }
}

#[cfg(feature = "compare")]
mod serde_diff_bench {
    use super::{Criterion, black_box, TestBench, WyRand, SAMPLE_SIZE, MEASUREMENT_TIME, SEED};
    use bincode::Options;

    pub(super) fn bench_large(c: &mut Criterion) {
        const GROUP_NAME: &str = "serde_diff_bench_large";
        let mut rng = WyRand::new_seed(SEED);
        let mut first = black_box(TestBench::generate_random_large(&mut rng));
        let second = black_box(TestBench::generate_random_large(&mut rng));
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let mut group = c.benchmark_group(GROUP_NAME);
        group.sample_size(SAMPLE_SIZE).measurement_time(MEASUREMENT_TIME);
        group.bench_function(GROUP_NAME,|b| b.iter(|| {
            let mut diff = black_box(
                options
                    .serialize(&serde_diff::Diff::serializable(&first, &second))
                    .unwrap(),
            );
            let mut deserializer = bincode::Deserializer::from_slice(&mut diff[..], options);
            serde_diff::Apply::apply(&mut deserializer, &mut first).unwrap();
        }));
        group.finish();
        assert_eq!(first.b, second.b);
    }
}
