use generators::{fill, rand_string};
use nanorand::{Rng, WyRand};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use structdiff::{Difference, StructDiff};

pub trait RandValue {
    fn next() -> Self;
}

#[cfg(not(feature = "nanoserde"))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Clone, Difference, Default)]
pub struct Test {
    pub test1: i32,
    pub test2: String,
    pub test3: Vec<i32>,
    pub test4: f32,
    pub test5: Option<usize>,
}

#[cfg(not(feature = "nanoserde"))]
#[derive(Debug, PartialEq, Clone, Difference)]
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

#[cfg(not(feature = "nanoserde"))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Clone, Difference, Default)]
pub enum TestEnum {
    #[default]
    F0,
    F1(()),
    F2(String),
    F3 {
        field1: String,
        field2: (),
    },
    F4(Test),
}

#[cfg(not(feature = "nanoserde"))]
impl RandValue for Test {
    fn next() -> Self {
        let mut rng = WyRand::new();
        Test {
            test1: rng.generate(),
            test2: rand_string(&mut rng),
            test3: fill(&mut rng),
            test4: f32::from_bits(rng.generate::<u32>()),
            test5: match rng.generate::<bool>() {
                true => Some(rng.generate()),
                false => None,
            },
        }
    }
}

#[cfg(not(feature = "nanoserde"))]
impl RandValue for TestEnum {
    fn next() -> Self {
        let mut rng = WyRand::new();
        match rng.generate_range(0..5) {
            0 => Self::F0,
            1 => Self::F1(()),
            2 => Self::F2(rand_string(&mut rng)),
            3 => Self::F3 {
                field1: rand_string(&mut rng),
                field2: (),
            },
            _ => Self::F4(Test::next()),
        }
    }
}

mod generators {
    use nanorand::{Rng, WyRand};

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
