use std::collections::{HashSet, LinkedList};
use structdiff::{Difference, StructDiff};

#[test]
fn test_example() {
    #[derive(Debug, PartialEq, Clone, Difference)]
    struct Example {
        field1: f64,
        #[difference(skip)]
        field2: Vec<i32>,
        field3: String,
    }

    let first = Example {
        field1: 0.0,
        field2: Vec::new(),
        field3: String::from("Hello Diff"),
    };

    let second = Example {
        field1: 3.14,
        field2: vec![1],
        field3: String::from("Hello Diff"),
    };

    let diffs = first.diff(&second);
    // diffs is now a Vec of differences, with length
    // equal to number of changed/unskipped fields
    assert_eq!(diffs.len(), 1);

    let diffed = first.apply(diffs);
    // diffed is now equal to second, except for skipped field
    assert_eq!(diffed.field1, second.field1);
    assert_eq!(diffed.field3, second.field3);
    assert_ne!(diffed, second);
}

#[derive(Debug, PartialEq, Clone, Difference)]
pub struct Test {
    test1: i32,
    test2: String,
    test3: Vec<i32>,
    test4: f32,
}

// #[cfg(test)]
mod derive {
    use super::*;

    #[test]
    fn test_derive() {
        let first: Test = Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
        };

        let second = Test {
            test1: first.test1,
            test2: String::from("Hello Diff"),
            test3: vec![1],
            test4: 3.14,
        };

        let diffs = first.diff(&second);
        let diffed = first.apply(diffs);

        assert_eq!(diffed, second);
    }

    #[derive(Debug, PartialEq, Clone, Difference)]
    struct TestSkip {
        test1: i32,
        test2: String,
        #[difference(skip)]
        test3skip: Vec<i32>,
        test4: f32,
    }

    #[test]
    fn test_derive_with_skip() {
        let first = TestSkip {
            test1: 0,
            test2: String::new(),
            test3skip: Vec::new(),
            test4: 0.0,
        };

        let second = TestSkip {
            test1: first.test1,
            test2: String::from("Hello Diff"),
            test3skip: vec![1],
            test4: 3.14,
        };

        let diffs = first.diff(&second);
        let diffed = first.apply(diffs);

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3skip, second.test3skip);
        assert_eq!(diffed.test4, second.test4);
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
            };

            let second = Test {
                test1: first.test1,
                test2: String::from("Hello Diff"),
                test3: vec![1],
                test4: 3.14,
            };

            let diffs = first.diff(&second);
            let diffed = first.apply(diffs);

            assert_eq!(diffed, second);
        }
    }

    #[test]
    fn test_recurse() {
        #[derive(Debug, PartialEq, Clone, Difference)]
        struct TestRecurse {
            test1: i32,
            #[difference(recurse)]
            test2: Test,
        }

        let first = TestRecurse {
            test1: 0,
            test2: Test {
                test1: 0,
                test2: String::new(),
                test3: Vec::new(),
                test4: 0.0,
            },
        };

        let second = TestRecurse {
            test1: 1,
            test2: Test {
                test1: 2,
                test2: String::new(),
                test3: Vec::new(),
                test4: 3.14,
            },
        };

        let diffs = first.diff(&second);
        assert_eq!(diffs.len(), 2);

        type TestRecurseFields = <TestRecurse as StructDiff>::Diff;

        if let TestRecurseFields::test2(val) = &diffs[1] {
            dbg!(&val);
            assert_eq!(val.len(), 2);
        } else {
            panic!("Recursion failure");
        }

        let diffed = first.apply(diffs);

        assert_eq!(diffed, second);
    }

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        struct TestCollection {
            #[difference(collection_strategy = "unordered_hash")]
            test1: Vec<i32>,
            #[difference(collection_strategy = "unordered_hash")]
            test2: HashSet<i32>,
            #[difference(collection_strategy = "unordered_hash")]
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

        type TestCollectionFields = <TestCollection as StructDiff>::Diff;

        if let TestCollectionFields::test1(
            structdiff::collections::UnorderedCollectionDiff::Replace(val),
        ) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(
            structdiff::collections::UnorderedCollectionDiff::Modify(val),
        ) = &diffs[2]
        {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let diffed = first.apply(diffs);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
        assert_eq_unordered!(diffed.test3, second.test3);
    }
}

#[cfg(all(test, feature = "nanoserde"))]
mod nanoserde_serialize {
    use nanoserde::{DeBin, SerBin};
    use std::collections::{HashSet, LinkedList};

    use super::Test;
    use structdiff::{Difference, StructDiff};

    #[derive(Debug, PartialEq, Clone, Difference)]
    struct TestSkip {
        test1: i32,
        test2: String,
        #[difference(skip)]
        test3: Vec<i32>,
        test4: f32,
    }

    #[test]
    fn test_derive_with_skip() {
        let first = TestSkip {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
        };

        let second = TestSkip {
            test1: first.test1,
            test2: String::from("Hello Diff"),
            test3: vec![1],
            test4: 3.14,
        };

        let diffs = first.diff(&second);
        let ser = SerBin::serialize_bin(&diffs);
        let diffed = first.apply(DeBin::deserialize_bin(&ser).unwrap());

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3, second.test3);
        assert_eq!(diffed.test4, second.test4);
    }

    #[test]
    fn test_recurse() {
        #[derive(Debug, PartialEq, Clone, Difference)]
        struct TestRecurse {
            test1: i32,
            #[difference(recurse)]
            test2: Test,
        }

        let first = TestRecurse {
            test1: 0,
            test2: Test {
                test1: 0,
                test2: String::new(),
                test3: Vec::new(),
                test4: 0.0,
            },
        };

        let second = TestRecurse {
            test1: 1,
            test2: Test {
                test1: 2,
                test2: String::new(),
                test3: Vec::new(),
                test4: 3.14,
            },
        };

        let diffs = first.diff(&second);
        assert_eq!(diffs.len(), 2);

        type TestRecurseFields = <TestRecurse as StructDiff>::Diff;

        if let TestRecurseFields::test2(val) = &diffs[1] {
            dbg!(&val);
            assert_eq!(val.len(), 2);
        } else {
            panic!("Recursion failure");
        }

        let ser = SerBin::serialize_bin(&diffs);
        let diffed = first.apply(DeBin::deserialize_bin(&ser).unwrap());

        assert_eq!(diffed, second);
    }

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        struct TestCollection {
            #[difference(collection_strategy = "unordered_hash")]
            test1: Vec<i32>,
            #[difference(collection_strategy = "unordered_hash")]
            test2: HashSet<i32>,
            #[difference(collection_strategy = "unordered_hash")]
            test3: LinkedList<i32>,
        }

        let first = TestCollection {
            test1: vec![10],
            test3: vec![10, 15, 17].into_iter().collect(),
            ..Default::default()
        };

        let second = TestCollection {
            test1: Vec::default(),
            test2: vec![10].into_iter().collect(),
            test3: vec![10, 15, 17, 19].into_iter().collect(),
        };

        let diffs = first.diff(&second);

        type TestCollectionFields = <TestCollection as StructDiff>::Diff;

        if let TestCollectionFields::test1(
            structdiff::collections::UnorderedCollectionDiff::Replace(val),
        ) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(
            structdiff::collections::UnorderedCollectionDiff::Modify(val),
        ) = &diffs[2]
        {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let ser = SerBin::serialize_bin(&diffs);
        let diffed = first.apply(DeBin::deserialize_bin(&ser).unwrap());

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_serialize {
    use std::collections::{HashSet, LinkedList};

    use super::Test;
    use structdiff::{Difference, StructDiff};

    #[derive(Debug, PartialEq, Clone, Difference)]
    struct TestSkip {
        test1: i32,
        test2: String,
        #[difference(skip)]
        test3: Vec<i32>,
        test4: f32,
    }

    #[test]
    fn test_derive_with_skip() {
        let first = TestSkip {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
        };

        let second = TestSkip {
            test1: first.test1,
            test2: String::from("Hello Diff"),
            test3: vec![1],
            test4: 3.14,
        };

        let diffs = first.diff(&second);
        let ser = serde_json::to_string(&diffs).unwrap();
        let diffed = first.apply(serde_json::from_str(&ser).unwrap());

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3, second.test3);
        assert_eq!(diffed.test4, second.test4);
    }

    #[test]
    fn test_recurse() {
        #[derive(Debug, PartialEq, Clone, Difference)]
        struct TestRecurse {
            test1: i32,
            #[difference(recurse)]
            test2: Test,
        }

        let first = TestRecurse {
            test1: 0,
            test2: Test {
                test1: 0,
                test2: String::new(),
                test3: Vec::new(),
                test4: 0.0,
            },
        };

        let second = TestRecurse {
            test1: 1,
            test2: Test {
                test1: 2,
                test2: String::new(),
                test3: Vec::new(),
                test4: 3.14,
            },
        };

        let diffs = first.diff(&second);
        assert_eq!(diffs.len(), 2);

        type TestRecurseFields = <TestRecurse as StructDiff>::Diff;

        if let TestRecurseFields::test2(val) = &diffs[1] {
            dbg!(&val);
            assert_eq!(val.len(), 2);
        } else {
            panic!("Recursion failure");
        }

        let diffs = first.diff(&second);
        let ser = serde_json::to_string(&diffs).unwrap();
        let diffed = first.apply(serde_json::from_str(&ser).unwrap());

        assert_eq!(diffed, second);
    }

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        struct TestCollection {
            #[difference(collection_strategy = "unordered_hash")]
            test1: Vec<i32>,
            #[difference(collection_strategy = "unordered_hash")]
            test2: HashSet<i32>,
            #[difference(collection_strategy = "unordered_hash")]
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

        type TestCollectionFields = <TestCollection as StructDiff>::Diff;

        if let TestCollectionFields::test1(
            structdiff::collections::UnorderedCollectionDiff::Replace(val),
        ) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(
            structdiff::collections::UnorderedCollectionDiff::Modify(val),
        ) = &diffs[2]
        {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let ser = serde_json::to_string(&diffs).unwrap();
        let diffed = first.apply(serde_json::from_str(&ser).unwrap());

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
        assert_eq_unordered!(diffed.test3, second.test3);
    }
}
