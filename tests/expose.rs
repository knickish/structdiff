#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::fmt::Debug;
use structdiff::{Difference, StructDiff};

#[test]
fn test_expose() {
    #[derive(Debug, PartialEq, Clone, Difference)]
    #[difference(expose)]
    struct Example {
        field1: f64,
    }

    let first = Example { field1: 0.0 };

    let second = Example { field1: PI };

    for diff in first.diff(&second) {
        match diff {
            ExampleStructDiffEnum::field1(v) => {
                dbg!(&v);
            }
        }
    }

    for diff in first.diff_ref(&second) {
        match diff {
            ExampleStructDiffEnumRef::field1(v) => {
                dbg!(&v);
            }
        }
    }
}

#[test]
fn test_expose_rename() {
    #[derive(Debug, PartialEq, Clone, Difference)]
    #[difference(expose = "Cheese")]
    struct Example {
        field1: f64,
    }

    let first = Example { field1: 0.0 };

    let second = Example { field1: PI };

    for diff in first.diff(&second) {
        match diff {
            Cheese::field1(_v) => {}
        }
    }

    for diff in first.diff_ref(&second) {
        match diff {
            CheeseRef::field1(_v) => {}
        }
    }
}

#[test]
fn test_expose_enum() {
    #[derive(Debug, Clone, PartialEq, Difference)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
    #[difference(expose)]
    pub enum Test {
        A,
        B(u32),
    }

    let first = Test::A;
    let second = Test::B(1);

    for diff in first.diff(&second) {
        match diff {
            TestStructDiffEnum::Replace(_) => {}
        }
    }
}
