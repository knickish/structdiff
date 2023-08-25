#![allow(unused_imports)]

mod derives;
mod enums;
mod types;
use assert_unordered::{assert_eq_unordered, assert_eq_unordered_sort};
pub use types::{RandValue, Test, TestEnum, TestSkip};

use std::hash::Hash;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList},
    fmt::Debug,
    num::Wrapping,
};
use structdiff::{Difference, StructDiff};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};

#[test]
/// This should match the code used in README.md
fn test_example() {
    #[derive(Debug, PartialEq, Clone, Difference)]
    struct Example {
        field1: f64,
        #[difference(skip)]
        field2: Vec<i32>,
        #[difference(collection_strategy = "unordered_array_like")]
        field3: BTreeSet<usize>,
    }

    let first = Example {
        field1: 0.0,
        field2: vec![],
        field3: vec![1, 2, 3].into_iter().collect(),
    };

    let second = Example {
        field1: 3.14,
        field2: vec![1],
        field3: vec![2, 3, 4].into_iter().collect(),
    };

    let diffs = first.diff(&second);
    // diffs is now a Vec of differences, with length
    // equal to number of changed/unskipped fields
    assert_eq!(diffs.len(), 2);

    let diffed = first.apply(diffs);
    // diffed is now equal to second, except for skipped field
    assert_eq!(diffed.field1, second.field1);
    assert_eq!(&diffed.field3, &second.field3);
    assert_ne!(diffed, second);
}

#[test]
fn test_derive() {
    let first: Test = Test {
        test1: 0,
        test2: String::new(),
        test3: Vec::new(),
        test4: 0.0,
        test5: None,
    };

    let second = Test {
        test1: first.test1,
        test2: String::from("Hello Diff"),
        test3: vec![1],
        test4: 3.14,
        test5: Some(12),
    };

    let diffs = first.diff(&second);
    let diffed = first.apply(diffs);

    assert_eq!(diffed, second);
}

#[test]
fn test_derive_with_skip() {
    let first: TestSkip<i32> = TestSkip {
        test1: 0,
        test2: String::new(),
        test3skip: Vec::new(),
        test4: 0.0,
    };

    let second: TestSkip<i32> = TestSkip {
        test1: first.test1,
        test2: String::from("Hello Diff"),
        test3skip: vec![1],
        test4: 3.14,
    };

    let diffs = first.diff(&second);

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        assert_eq!(diffed_serde.test1, second.test1);
        assert_eq!(diffed_serde.test2, second.test2);
        assert_ne!(diffed_serde.test3skip, second.test3skip);
        assert_eq!(diffed_serde.test4, second.test4);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_serde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        assert_eq!(diffed_serde.test1, second.test1);
        assert_eq!(diffed_serde.test2, second.test2);
        assert_ne!(diffed_serde.test3skip, second.test3skip);
        assert_eq!(diffed_serde.test4, second.test4);
    }

    let diffed = first.apply(diffs);

    //check that all except the skipped are changed
    assert_eq!(diffed.test1, second.test1);
    assert_eq!(diffed.test2, second.test2);
    assert_ne!(diffed.test3skip, second.test3skip);
    assert_eq!(diffed.test4, second.test4);
}

#[derive(Debug, PartialEq, Clone, Difference)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
#[difference(setters)]
struct TestGenerics<A, B, C, RS: Eq + Hash> {
    test1: A,
    test2: B,
    test3: C,
    test4: HashMap<RS, A>,
}

#[test]
fn test_generics() {
    type TestType = TestGenerics<i32, usize, Option<bool>, String>;
    let first: TestType = TestGenerics {
        test1: 0,
        test2: 42,
        test3: Some(true),
        test4: [(String::from("test123"), 1)].into_iter().collect(),
    };

    let second: TestType = TestGenerics {
        test1: 0,
        test2: 42,
        test3: None,
        test4: [(String::from("test1234"), 2)].into_iter().collect(),
    };

    let diffs = first.diff(&second);

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        assert_eq!(diffed_serde, second);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_serde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        assert_eq!(&diffed_serde, &second);
    }

    let diffed = first.apply(diffs);

    //check that all except the skipped are changed
    assert_eq!(diffed.test1, second.test1);
    assert_eq!(diffed.test2, second.test2);
    assert_eq!(diffed.test3, second.test3);
    assert_eq!(diffed.test4, second.test4);
}

#[derive(Debug, PartialEq, Clone, Difference)]
#[difference(setters)]
struct TestGenericsSkip<A, B, C, RS: Eq + Hash> {
    test1: A,
    test2: B,
    test3: C,
    #[difference(skip)]
    test4: HashMap<RS, A>,
    test5: HashMap<RS, A>,
}

#[test]
fn test_generics_skip() {
    let first: TestGenericsSkip<i32, usize, Option<bool>, String> = TestGenericsSkip {
        test1: 0,
        test2: 42,
        test3: Some(true),
        test4: [(String::from("test123"), 1)].into_iter().collect(),
        test5: [(String::from("test123"), 1)].into_iter().collect(),
    };

    let second: TestGenericsSkip<i32, usize, Option<bool>, String> = TestGenericsSkip {
        test1: 0,
        test2: 42,
        test3: None,
        test4: [(String::from("test1234"), 2)].into_iter().collect(),
        test5: [(String::from("test1234"), 2)].into_iter().collect(),
    };

    let diffs = first.diff(&second);

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        assert_eq!(diffed_serde.test1, second.test1);
        assert_eq!(diffed_serde.test2, second.test2);
        assert_eq!(diffed_serde.test3, second.test3);
        assert_ne!(diffed_serde.test4, second.test4);
        assert_eq!(diffed_serde.test5, second.test5);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_serde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        assert_eq!(diffed_serde.test1, second.test1);
        assert_eq!(diffed_serde.test2, second.test2);
        assert_eq!(diffed_serde.test3, second.test3);
        assert_ne!(diffed_serde.test4, second.test4);
        assert_eq!(diffed_serde.test5, second.test5);
    }

    let diffed = first.apply(diffs);

    //check that all except the skipped are changed
    assert_eq!(diffed.test1, second.test1);
    assert_eq!(diffed.test2, second.test2);
    assert_eq!(diffed.test3, second.test3);
    assert_ne!(diffed.test4, second.test4);
    assert_eq!(diffed.test5, second.test5);
}

#[test]
fn test_enums() {
    let mut follower = TestEnum::next();
    let mut leader: TestEnum;
    for _ in 0..100 {
        leader = TestEnum::next();
        let diff = follower.diff(&leader);
        follower.apply_mut(diff);
        assert_eq!(leader, follower)
    }
}

mod derive_inner {
    use super::{StructDiff, Test};
    //tests that the associated type does not need to be exported manually

    #[test]
    fn test_derive_inner() {
        let first: Test = Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
            test5: None,
        };

        let second = Test {
            test1: first.test1,
            test2: String::from("Hello Diff"),
            test3: vec![1],
            test4: 3.14,
            test5: Some(13),
        };

        let diffs = first.diff(&second);
        let diffed = first.apply(diffs);

        assert_eq!(diffed, second);
    }
}

#[test]
fn test_recurse() {
    #[derive(Debug, PartialEq, Clone, Difference)]
    #[difference(setters)]
    struct TestRecurse {
        test1: i32,
        #[difference(recurse)]
        test2: Test,
        #[difference(recurse)]
        test3: Option<Test>,
        #[difference(recurse)]
        test4: Option<Test>,
        #[difference(recurse)]
        test5: Option<Test>,
    }

    let first = TestRecurse {
        test1: 0,
        test2: Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
            test5: Some(14),
        },
        test3: None,
        test4: Some(Test::default()),
        test5: Some(Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
            test5: Some(14),
        }),
    };

    let second = TestRecurse {
        test1: 1,
        test2: Test {
            test1: 2,
            test2: String::new(),
            test3: Vec::new(),
            test4: 3.14,
            test5: None,
        },
        test3: Some(Test::default()),
        test4: Some(Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
            test5: Some(14),
        }),
        test5: None,
    };

    let diffs = first.diff(&second);
    assert_eq!(diffs.len(), 5);

    type TestRecurseFields = <TestRecurse as StructDiff>::Diff;

    if let TestRecurseFields::test2(val) = &diffs[1] {
        assert_eq!(val.len(), 3);
    } else {
        panic!("Recursion failure");
    }

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        assert_eq!(diffed_serde, second);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_serde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        assert_eq!(&diffed_serde, &second);
    }

    let diffed = first.apply(diffs);

    assert_eq!(diffed, second);
}

#[test]
fn test_collection_strategies() {
    #[derive(Debug, PartialEq, Clone, Difference, Default)]
    struct TestCollection {
        #[difference(collection_strategy = "unordered_array_like")]
        test1: Vec<i32>,
        #[difference(collection_strategy = "unordered_array_like")]
        test2: HashSet<i32>,
        #[difference(collection_strategy = "unordered_array_like")]
        test3: LinkedList<i32>,
    }

    let first = TestCollection {
        test1: vec![10, 15, 20, 25, 30],
        test3: vec![10, 15, 17].into_iter().collect(),
        ..Default::default()
    };

    let second = TestCollection {
        test1: Vec::default(),
        test2: vec![10].into_iter().collect(),
        test3: vec![10, 15, 17, 19].into_iter().collect(),
    };

    let diffs = first.diff(&second);

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(&diffed_serde.test1, &second.test1);
        assert_eq_unordered!(&diffed_serde.test2, &second.test2);
        assert_eq_unordered!(&diffed_serde.test3, &second.test3);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_nserde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(&diffed_nserde.test1, &second.test1);
        assert_eq_unordered!(&diffed_nserde.test2, &second.test2);
        assert_eq_unordered!(&diffed_nserde.test3, &second.test3);
    }

    let diffed = first.apply(diffs);

    use assert_unordered::assert_eq_unordered;
    assert_eq_unordered!(diffed.test1, second.test1);
    assert_eq_unordered!(diffed.test2, second.test2);
    assert_eq_unordered!(diffed.test3, second.test3);
}

#[test]
fn test_key_value() {
    #[derive(Debug, PartialEq, Clone, Difference, Default)]
    struct TestCollection {
        #[difference(
            collection_strategy = "unordered_map_like",
            map_equality = "key_and_value"
        )]
        test1: HashMap<i32, i32>,
    }

    let first = TestCollection {
        test1: vec![(10, 0), (15, 2), (20, 0), (25, 0), (30, 15)]
            .into_iter()
            .collect(),
    };

    let second = TestCollection {
        test1: vec![(10, 21), (15, 2), (20, 0), (25, 0), (30, 15)]
            .into_iter()
            .collect(),
    };

    let diffs = first.diff(&second);

    #[cfg(feature = "serde")]
    {
        let ser_diff = bincode::serialize(&diffs).unwrap();
        let deser_diff = bincode::deserialize(&ser_diff).unwrap();
        let diffed_serde = first.clone().apply(deser_diff);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(&diffed_serde.test1, &second.test1);
    }

    #[cfg(feature = "nanoserde")]
    {
        let ser = SerBin::serialize_bin(&diffs);
        let diffed_serde = first.clone().apply(DeBin::deserialize_bin(&ser).unwrap());

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(&diffed_serde.test1, &second.test1);
    }

    let diffed = first.apply(diffs);

    use assert_unordered::assert_eq_unordered;
    assert_eq_unordered!(diffed.test1, second.test1);
}

#[cfg(feature = "generated_setters")]
#[test]
fn test_setters() {
    use types::TestSetters;
    let mut base = TestSetters::default();
    let mut end = TestSetters::default();
    let mut partial_diffs = vec![];
    let mut full_diffs = vec![];

    for _ in 0..100 {
        end = TestSetters::next();
        partial_diffs.extend(base.testing123(end.f0.clone()));
        partial_diffs.extend(base.set_f1_with_diff(end.f1.clone()));
        partial_diffs.extend(base.set_f2_with_diff(end.f2.clone()));
        partial_diffs.extend(base.set_f3_with_diff(end.f3.clone()));
        partial_diffs.extend(base.set_f4_with_diff(end.f4.clone()));
        partial_diffs.extend(base.set_f5_with_diff(end.f5.clone()));
        partial_diffs.extend(base.set_f6_with_diff(end.f6.clone()));
        let tmp = base.apply_ref(partial_diffs.clone());
        assert_eq!(&tmp.f0, &end.f0);
        assert_eq!(&tmp.f1, &end.f1);
        assert_eq!(&tmp.f2, &end.f2);
        assert_eq!(&tmp.f3, &end.f3);
        assert_eq_unordered!(&tmp.f4, &end.f4);
        assert_eq_unordered!(&tmp.f5, &end.f5);
        assert_eq_unordered!(&tmp.f6, &end.f6);
        base = end.clone();
        full_diffs.extend(std::mem::take(&mut partial_diffs));
    }

    let modified = TestSetters::default().apply(full_diffs);
    assert_eq!(modified.f0, end.f0);
    assert_eq!(modified.f1, end.f1);
    assert_eq!(modified.f2, end.f2);
    assert_eq!(modified.f3, end.f3);
    assert_eq_unordered_sort!(modified.f4, end.f4);
    assert_eq_unordered!(modified.f5, end.f5);
    assert_eq_unordered!(modified.f6, end.f6);
}
