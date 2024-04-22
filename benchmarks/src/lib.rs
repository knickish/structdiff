use std::collections::{HashMap, HashSet};

use assert_unordered::assert_eq_unordered_sort;
use nanorand::{Rng, WyRand};
use structdiff::{Difference, StructDiff};

pub mod basic;
pub mod large;

#[derive(Debug, Difference, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "compare", derive(diff::Diff))]
#[cfg_attr(feature = "compare", diff(attr(
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
)))]
#[cfg_attr(feature = "compare", derive(serde_diff::SerdeDiff))]
pub struct TestBench {
    pub a: String,
    pub b: i32,
    #[difference(collection_strategy = "unordered_array_like")]
    #[cfg_attr(feature = "compare", serde_diff(opaque))]
    pub c: HashSet<String>,
    #[difference(collection_strategy = "ordered_array_like")]
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
    let base = vec![(); rng.generate_range::<u8, _>(5..15) as usize];
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
            c: (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            d: (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            e: (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
            f: (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
        }
    }

    pub fn generate_random_large(rng: &mut WyRand) -> TestBench {
        TestBench {
            a: rand_string_large(rng),
            b: rng.generate::<i32>(),
            c: (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            d: (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .into_iter()
                .collect(),
            e: (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
            f: (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| (rng.generate::<i32>(), rand_string(rng)))
                .into_iter()
                .collect(),
        }
    }

    pub fn random_mutate(self, rng: &mut WyRand) -> Self {
        match rng.generate_range(0..6) {
            0 => Self {
                a: rand_string(rng),
                ..self
            },
            1 => Self {
                b: rng.generate::<i32>(),
                ..self
            },
            2 => Self {
                c: self
                    .c
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 30_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            rand_string(rng)
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u8, _>(0..(u8::MAX / 4)))
                            .map(|_| rand_string(rng)),
                    )
                    .collect(),
                ..self
            },
            3 => Self {
                d: self
                    .d
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 30_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            rand_string(rng)
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u8, _>(0..(u8::MAX / 4)))
                            .map(|_| rand_string(rng)),
                    )
                    .collect(),
                ..self
            },
            4 => Self {
                e: self
                    .e
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 25_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            (rng.generate::<i32>(), rand_string(rng))
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u8, _>(0..(u8::MAX / 4)))
                            .map(|_| (rng.generate::<i32>(), rand_string(rng))),
                    )
                    .collect(),
                ..self
            },
            5 => Self {
                f: self
                    .f
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 25_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            (rng.generate::<i32>(), rand_string(rng))
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u8, _>(0..(u8::MAX / 4)))
                            .map(|_| (rng.generate::<i32>(), rand_string(rng))),
                    )
                    .collect(),
                ..self
            },
            _ => self,
        }
    }

    pub fn random_mutate_large(self, rng: &mut WyRand) -> Self {
        match rng.generate_range(0..6) {
            0 => Self {
                a: rand_string_large(rng),
                ..self
            },
            1 => Self {
                b: rng.generate::<i32>(),
                ..self
            },
            2 => Self {
                c: self
                    .c
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 30_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            rand_string(rng)
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                            .map(|_| rand_string(rng)),
                    )
                    .collect(),
                ..self
            },
            3 => Self {
                d: self
                    .d
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 30_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            rand_string(rng)
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                            .map(|_| rand_string(rng)),
                    )
                    .collect(),
                ..self
            },
            4 => Self {
                e: self
                    .e
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 25_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            (rng.generate::<i32>(), rand_string(rng))
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                            .map(|_| (rng.generate::<i32>(), rand_string(rng))),
                    )
                    .collect(),
                ..self
            },
            5 => Self {
                f: self
                    .f
                    .into_iter()
                    .filter(|_| rng.generate_range(0..100) < 25_u8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if rng.generate_range(0..100) < 25_u8 {
                            (rng.generate::<i32>(), rand_string(rng))
                        } else {
                            v
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                            .map(|_| (rng.generate::<i32>(), rand_string(rng))),
                    )
                    .collect(),
                ..self
            },
            _ => self,
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
    use bincode::Options;

    use super::*;

    #[test]
    fn test_sizes_basic() {
        structdiff_size::size_basic();
        #[cfg(feature = "compare")]
        {
            serde_diff_size::size_basic();
            diff_struct_size::size_basic();
        }
    }

    #[ignore]
    #[test]
    fn test_sizes_large() {
        structdiff_size::size_large();
        #[cfg(feature = "compare")]
        {
            serde_diff_size::size_large();
            diff_struct_size::size_large();
        }
    }

    #[test]
    fn test_sizes_basic_mut() {
        structdiff_size_mut::size_basic_mut();
        #[cfg(feature = "compare")]
        {
            serde_diff_size_mut::size_basic_mut();
            diff_struct_size_mut::size_basic_mut();
        }
    }

    #[ignore]
    #[test]
    fn test_sizes_large_mut() {
        structdiff_size_mut::size_large_mut();
        #[cfg(feature = "compare")]
        {
            serde_diff_size_mut::size_large_mut();
            diff_struct_size_mut::size_large_mut();
        }
    }

    mod structdiff_size {
        use super::*;

        pub fn size_basic() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _i in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random(&mut rng));
                let diff = StructDiff::diff(&first, &second);
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("StructDiff - small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _i in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                bytes += bincode::serialized_size(&StructDiff::diff(&first, &second)).unwrap();
            }
            println!("StructDiff - large: {} bytes", bytes as f64 / 100.0)
        }
    }

    #[cfg(feature = "compare")]
    mod diff_struct_size {
        use diff::Diff;

        use super::*;

        pub fn size_basic() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _i in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random(&mut rng));
                let diff = Diff::diff(&first, &second);
                bytes += bincode::serialized_size(&diff).unwrap();
            }

            println!("Diff-Struct - small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                bytes += bincode::serialized_size(&Diff::diff(&first, &second)).unwrap();
            }
            println!("Diff-Struct - large: {} bytes", bytes as f64 / 100.0)
        }
    }

    #[cfg(feature = "compare")]
    mod serde_diff_size {
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
            println!("Serde-Diff - large: {} bytes", bytes as f64 / 100.0)
        }
    }

    mod structdiff_size_mut {
        use super::*;

        pub fn size_basic_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate(&mut rng));
                let diff = StructDiff::diff(&first, &second);

                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("StructDiff - mut small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate_large(&mut rng));
                let diff = StructDiff::diff(&first, &second);
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("StructDiff - mut large: {} bytes", bytes as f64 / 100.0)
        }
    }

    #[cfg(feature = "compare")]
    mod diff_struct_size_mut {
        use diff::Diff;

        use super::*;

        pub fn size_basic_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate(&mut rng));
                let diff =
                    std::hint::black_box(options.serialize(&Diff::diff(&first, &second)).unwrap());

                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Diff-Struct - mut small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate_large(&mut rng));
                let diff =
                    std::hint::black_box(options.serialize(&Diff::diff(&first, &second)).unwrap());
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Diff-Struct - mut large: {} bytes", bytes as f64 / 100.0)
        }
    }

    #[cfg(feature = "compare")]
    mod serde_diff_size_mut {
        use bincode::Options;

        use super::*;

        pub fn size_basic_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate(&mut rng));
                let diff = std::hint::black_box(
                    options
                        .serialize(&serde_diff::Diff::serializable(&first, &second))
                        .unwrap(),
                );

                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Serde-Diff - mut small: {} bytes", bytes as f64 / 100.0)
        }

        pub fn size_large_mut() {
            let mut bytes = 0_u64;
            let mut rng = WyRand::new();
            let options = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();
            for _ in 0..100 {
                let first = std::hint::black_box(TestBench::generate_random_large(&mut rng));
                let second = std::hint::black_box(first.clone().random_mutate_large(&mut rng));
                let diff = std::hint::black_box(
                    options
                        .serialize(&serde_diff::Diff::serializable(&first, &second))
                        .unwrap(),
                );
                bytes += bincode::serialized_size(&diff).unwrap();
            }
            println!("Serde-Diff - mut large: {} bytes", bytes as f64 / 100.0)
        }
    }
}
