#![feature(test)]
#![cfg(test)]

extern crate test;
extern crate structdiff_benchmarks;

use nanorand::WyRand;
use structdiff::StructDiff;
use structdiff_benchmarks::TestBench;
use test::Bencher;

#[bench]
fn bench_large(b: &mut Bencher) {
    let mut rng = WyRand::new();
    let mut first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    b.iter(|| {
        let diff = StructDiff::diff(&first, &second);
        std::hint::black_box(first.apply_mut(diff))
    });
}

#[cfg(feature = "compare")]
mod diff_struct_bench {
    use super::*;
    use diff::Diff;

    #[bench]
    fn bench_large(b: &mut Bencher) {
        let mut rng = WyRand::new();
        let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
        let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
        b.iter(|| {
            let diff = Diff::diff(&first, &second);
            std::hint::black_box(Diff::apply(&mut first.clone(), &diff))
        });
    }
}

#[cfg(feature = "compare")]
mod serde_diff_bench {
    use super::*;
    use bincode::Options;
    use serde_diff::Diff;

    #[bench]
    fn bench_large(b: &mut Bencher) {
        let mut rng = WyRand::new();
        let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
        let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        b.iter(|| {
            let mut target = second.clone();
            let mut diff = std::hint::black_box(
                options
                    .serialize(&serde_diff::Diff::serializable(&first, &second))
                    .unwrap(),
            );
            let mut deserializer = bincode::Deserializer::from_slice(&mut diff[..], options);
            serde_diff::Apply::apply(&mut deserializer, &mut target).unwrap();
        });
    }
}
