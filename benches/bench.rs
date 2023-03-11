#![feature(test)]

#[cfg(feature = "bench")] 
mod bench {


extern crate test;
use std::collections::HashSet;

use test::Bencher;


use nanorand::{Rng, WyRand};
use structdiff::{Difference, StructDiff};
use diff::Diff;

#[derive(Debug, Difference, PartialEq, Clone, Diff, serde::Serialize)]
struct TestBench {
    a: String,
    b: i32,
    #[difference(collection_strategy = "unordered_array_like")]
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
            c: (0..rng.generate::<u8>()).map(|_| rand_string(rng) )
                .into_iter()
                .collect(),
            d: (0..rng.generate::<u8>()).map(|_| rand_string(rng) )
                .into_iter()
                .collect(),
        }
    }

    fn generate_random_large(rng: &mut WyRand) -> TestBench {
        TestBench {
            a: rand_string_large(rng),
            b: rng.generate::<i32>(),
            c: (0..rng.generate::<u16>()).map(|_| rand_string(rng) )
                .into_iter()
                .collect(),
            d: (0..rng.generate::<u16>()).map(|_| rand_string(rng) )
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
        let diff = StructDiff::diff(&first,&second);
        std::hint::black_box(first.apply_ref(diff))
    });
}

#[bench]
fn bench_large(b: &mut Bencher) {
    let mut rng = WyRand::new();
    let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
    b.iter(|| {
        let diff = StructDiff::diff(&first,&second);
        std::hint::black_box(first.apply_ref(diff))
    });
}

mod competition {
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

}