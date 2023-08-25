use std::collections::BTreeMap;

use generators::{fill, rand_bool, rand_string};
use nanorand::{Rng, WyRand};
#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use structdiff::{Difference, StructDiff};

pub trait RandValue
where
    Self: Sized,
{
    fn next() -> Self {
        let mut rng = WyRand::new();
        Self::next_seeded(&mut rng)
    }

    fn next_seeded(rng: &mut WyRand) -> Self;
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
#[derive(Debug, PartialEq, Clone, Difference, Default)]
#[difference(setters)]
pub struct Test {
    pub test1: i32,
    pub test2: String,
    pub test3: Vec<i32>,
    pub test4: f32,
    pub test5: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Difference)]
#[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
#[difference(setters)]
pub struct TestSkip<A>
where
    A: PartialEq,
{
    pub test1: A,
    pub test2: String,
    #[difference(skip)]
    pub test3skip: Vec<i32>,
    pub test4: f32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
#[derive(Debug, PartialEq, Clone, Difference, Default)]
pub enum TestEnum {
    #[default]
    F0,
    F1(bool),
    F2(String),
    F3 {
        field1: String,
        field2: bool,
    },
    F4(Test),
}

impl RandValue for Test {
    fn next_seeded(rng: &mut WyRand) -> Self {
        Test {
            test1: rng.generate(),
            test2: rand_string(rng),
            test3: fill(rng),
            test4: match f32::from_bits(rng.generate::<u32>()) {
                val if val.is_nan() => 0.0,
                val => val,
            },
            test5: match rng.generate::<bool>() {
                true => Some(rng.generate()),
                false => None,
            },
        }
    }
}

impl RandValue for TestEnum {
    fn next_seeded(rng: &mut WyRand) -> Self {
        match rng.generate_range(0..5) {
            0 => Self::F0,
            1 => Self::F1(rand_bool(rng)),
            2 => Self::F2(rand_string(rng)),
            3 => Self::F3 {
                field1: rand_string(rng),
                field2: rand_bool(rng),
            },
            _ => Self::F4(Test::next()),
        }
    }
}

#[derive(Difference, Default, PartialEq, Debug, Clone)]
#[difference(setters)]
pub struct TestSetters {
    #[difference(setter_name = "testing123", recurse)]
    pub f0: Test,
    pub f1: Test,
    pub f2: TestEnum,
    #[difference(recurse)]
    pub f3: Option<Test>,
    #[difference(collection_strategy = "unordered_array_like")]
    pub f4: Vec<i32>,
    #[difference(collection_strategy = "unordered_map_like", map_equality = "key_only")]
    pub f5: BTreeMap<i32, Test>,
    #[difference(
        collection_strategy = "unordered_map_like",
        map_equality = "key_and_value"
    )]
    pub f6: BTreeMap<i32, Test>,
}

impl RandValue for TestSetters {
    fn next_seeded(rng: &mut WyRand) -> Self {
        TestSetters {
            f0: Test::next(),
            f1: Test::next(),
            f2: TestEnum::next(),
            f3: if rng.generate::<bool>() {
                Some(Test::next_seeded(rng))
            } else {
                None
            },
            f4: generators::fill(rng),
            f5: generators::fill::<i32, Vec<i32>>(rng)
                .into_iter()
                .map(|x| (x, Test::next_seeded(rng)))
                .take(10)
                .collect(),
            f6: generators::fill::<i32, Vec<i32>>(rng)
                .into_iter()
                .map(|x| (x, Test::next_seeded(rng)))
                .take(10)
                .collect(),
        }
    }
}

mod generators {
    use nanorand::{Rng, WyRand};

    pub(super) fn rand_bool(rng: &mut WyRand) -> bool {
        let base = rng.generate::<u8>() as usize;
        if base % 2 == 0 {
            true
        } else {
            false
        }
    }

    pub(super) fn rand_string(rng: &mut WyRand) -> String {
        let base = vec![(); rng.generate::<u8>() as usize];
        base.into_iter()
            .map(|_| rng.generate::<u8>() as u32)
            .filter_map(char::from_u32)
            .collect::<String>()
    }

    pub(super) fn fill<V, T>(rng: &mut WyRand) -> T
    where
        V: nanorand::RandomGen<nanorand::WyRand, 8>,
        T: FromIterator<V>,
    {
        let base = vec![(); rng.generate::<u8>() as usize];
        base.into_iter().map(|_| rng.generate::<V>()).collect::<T>()
    }
}
