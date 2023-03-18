#![feature(test)]
#![cfg(test)]

extern crate test;
use std::collections::HashSet;

use test::Bencher;

use diff::Diff;
use nanorand::{Rng, WyRand};
use structdiff::{Difference, StructDiff};

#[derive(
    Debug,
    Difference,
    PartialEq,
    Clone,
    Diff,
    serde::Serialize,
    serde::Deserialize,
    serde_diff::SerdeDiff,
)]
struct TestBench {
    a: String,
    b: i32,
    #[difference(collection_strategy = "unordered_array_like")]
    #[serde_diff(opaque)]
    c: HashSet<String>,
    #[difference(collection_strategy = "unordered_array_like")]
    d: Vec<String>,
}

fn rand_string(rng: &mut WyRand) -> String {
    let base = vec![(); rng.generate::<u8>() as usize];
    base.into_iter()
        .map(|_| rng.generate::<u8>() as u32)
        .filter_map(char::from_u32)
        .collect::<String>()
}

fn rand_string_large(rng: &mut WyRand) -> String {
    let base = vec![(); rng.generate::<u16>() as usize];
    base.into_iter()
        .map(|_| rng.generate::<u32>())
        .filter_map(char::from_u32)
        .collect::<String>()
}

impl TestBench {
    fn generate_random(rng: &mut WyRand) -> TestBench {
        TestBench {
            a: rand_string(rng),
            b: rng.generate::<i32>(),
            c: (0..rng.generate::<u8>())
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            d: (0..rng.generate::<u8>())
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
        }
    }

    fn generate_random_large(rng: &mut WyRand) -> TestBench {
        TestBench {
            a: rand_string_large(rng),
            b: rng.generate::<i32>(),
            c: (0..rng.generate::<u16>())
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            d: (0..rng.generate::<u16>())
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
        }
    }
}

#[bench]
fn bench_basic(b: &mut Bencher) {
    let mut rng = WyRand::new();
    let first = std::hint::black_box(TestBench::generate_random(&mut rng));
    let second = std::hint::black_box(TestBench::generate_random(&mut rng));
    b.iter(|| {
        let diff = StructDiff::diff(&first, &second);
        std::hint::black_box(first.apply_ref(diff))
    });
}

#[bench]
fn bench_large(b: &mut Bencher) {
    let mut rng = WyRand::new();
    let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    b.iter(|| {
        let diff = StructDiff::diff(&first, &second);
        std::hint::black_box(first.apply_ref(diff))
    });
}

mod diff_struct_bench {
    use super::*;

    #[bench]
    fn bench_basic(b: &mut Bencher) {
        let mut rng = WyRand::new();
        let first = std::hint::black_box(TestBench::generate_random(&mut rng));
        let second = std::hint::black_box(TestBench::generate_random(&mut rng));
        b.iter(|| {
            let diff = Diff::diff(&first, &second);
            std::hint::black_box(Diff::apply(&mut first.clone(), &diff))
        });
    }

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

mod serde_diff_bench {
    use bincode::Options;

    use super::*;

    #[bench]
    fn bench_basic(b: &mut Bencher) {
        let mut rng = WyRand::new();
        let first = std::hint::black_box(TestBench::generate_random(&mut rng));
        let second = std::hint::black_box(TestBench::generate_random(&mut rng));
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

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn test_sizes() {
        size_basic();
        serde_diff_size::size_basic();
        size_large();
        serde_diff_size::size_large();
    }

    fn size_basic() {
        let mut bytes = 0_u64;
        let mut rng = WyRand::new();
        for _ in 0..100 {
            let first = std::hint::black_box(TestBench::generate_random(&mut rng));
            let second = std::hint::black_box(TestBench::generate_random(&mut rng));
            let diff = StructDiff::diff(&first, &second);
            bytes += bincode::serialized_size(&diff).unwrap();
        }
        println!("StructDiff - small: {} bytes", bytes as f64 / 100.0)
    }

    fn size_large() {
        let mut bytes = 0_u64;
        let mut rng = WyRand::new();
        for _ in 0..100 {
            let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
            let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
            let diff = StructDiff::diff(&first, &second);
            bytes += bincode::serialized_size(&diff).unwrap();
        }
        println!("StructDiff - large: {} bytes", bytes as f64 / 100.0)
    }

    mod serde_diff_size {
        use bincode::Options;

        use super::*;

        pub fn size_basic() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random(&mut rng));
                let diff = std::hint::black_box(
                    options
                        .serialize(&serde_diff::Diff::serializable(&first, &second))
                        .unwrap(),
                );
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Serde-Diff - small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let diff = std::hint::black_box(
                    options
                        .serialize(&serde_diff::Diff::serializable(&first, &second))
                        .unwrap(),
                );
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Serde-Diff - small: {} bytes", bytes as f64 / 100.0)
        }
    }
}
