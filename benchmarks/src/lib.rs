use std::collections::{HashMap, HashSet};

use assert_unordered::assert_eq_unordered_sort;
use nanorand::{Rng, WyRand};
use structdiff::{Difference, StructDiff};

#[derive(Debug, Difference, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "compare", derive(diff::Diff))]
#[cfg_attr(feature = "compare", derive(serde_diff::SerdeDiff))]
pub struct TestBench {
    pub a: String,
    pub b: i32,
    #[difference(collection_strategy = "unordered_array_like")]
    #[cfg_attr(feature = "compare", serde_diff(opaque))]
    pub c: HashSet<String>,
    #[difference(collection_strategy = "unordered_array_like")]
    pub d: Vec<String>,
    #[difference(collection_strategy = "unordered_map_like", map_equality = "key_only")]
    pub e: HashMap<i32, String>,
    #[difference(
        collection_strategy = "unordered_map_like",
        map_equality = "key_and_value"
    )]
    pub f: HashMap<i32, String>,
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
    pub fn generate_random(rng: &mut WyRand) -> TestBench {
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
            e: (0..rng.generate::<u8>())
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
            f: (0..rng.generate::<u8>())
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
        }
    }

    pub fn generate_random_large(rng: &mut WyRand) -> TestBench {
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
            e: (0..rng.generate::<u16>())
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
            f: (0..rng.generate::<u16>())
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
        }
    }

    #[track_caller]
    pub fn assert_eq(self, right: TestBench, diff: &Vec<<TestBench as StructDiff>::Diff>) {
        assert_eq!(self.a, right.a, "{:?}", diff);
        assert_eq!(self.b, right.b, "{:?}", diff);
        assert_eq_unordered_sort!(self.c, right.c, "{:?}", diff);
        assert_eq_unordered_sort!(self.d, right.d, "{:?}", diff);
        assert_eq_unordered_sort!(
            self.e.iter().map(|x| x.0).collect::<Vec<_>>(),
            right.e.iter().map(|x| x.0).collect::<Vec<_>>(),
            "{:?}",
            diff
        );
        assert_eq_unordered_sort!(self.f, right.f, "{:?}", diff);
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn test_sizes() {
        size_basic();
        #[cfg(feature = "compare")]
        serde_diff_size::size_basic();
        size_large();
        #[cfg(feature = "compare")]
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

    #[cfg(feature = "compare")]
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
