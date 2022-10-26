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

    let diffs = second.diff(&first);
    // diffs is now a Vec of differences, with length
    // equal to number of changed/unskipped fields
    assert_eq!(diffs.len(), 1);

    let diffed = first.apply(diffs);
    // diffed is now equal to second, except for skipped field
    assert_eq!(diffed.field1, second.field1);
    assert_eq!(diffed.field3, second.field3);
    assert_ne!(diffed, second);
}

mod basic {
    use super::*;
    #[derive(Debug, PartialEq, Clone)]
    struct Test {
        test1: i32,
        test2: String,
        test3: Vec<i32>,
        test4: f32,
    }

    #[allow(non_camel_case_types)]
    pub enum __TestStructDiffEnum {
        test1(i32),
        test2(String),
        test3(Vec<i32>),
        test4(f32),
    }

    impl StructDiff for Test {
        type Diff = __TestStructDiffEnum;

        fn diff(&self, prev: &Self) -> Vec<Self::Diff> {
            let mut diffs = vec![];
            if self.test1 != prev.test1 {
                diffs.push(Self::Diff::test1(self.test1.clone()))
            };
            if self.test2 != prev.test2 {
                diffs.push(Self::Diff::test2(self.test2.clone()))
            };
            if self.test3 != prev.test3 {
                diffs.push(Self::Diff::test3(self.test3.clone()))
            };
            if self.test4 != prev.test4 {
                diffs.push(Self::Diff::test4(self.test4.clone()))
            };
            diffs
        }

        #[inline(always)]
        fn apply_single(&mut self, diff: Self::Diff) {
            match diff {
                __TestStructDiffEnum::test1(__1) => self.test1 = __1,
                __TestStructDiffEnum::test2(__2) => self.test2 = __2,
                __TestStructDiffEnum::test3(__3) => self.test3 = __3,
                __TestStructDiffEnum::test4(__4) => self.test4 = __4,
            }
        }
    }

    #[test]
    fn test_basic() {
        let first: Test = Test {
            test1: 0,
            test2: String::new(),
            test3: Vec::new(),
            test4: 0.0,
        };

        let second = Test {
            test1: 1,
            test2: String::from("Hello Diff"),
            test3: vec![1],
            test4: 3.14,
        };

        let diffs = second.diff(&first);

        let diffed = first.apply(diffs);

        assert_eq!(diffed, second);
    }
}

mod derive {
    use super::*;

    #[derive(Debug, PartialEq, Clone, Difference)]
    struct Test {
        test1: i32,
        test2: String,
        test3: Vec<i32>,
        test4: f32,
    }

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

        let diffs = second.diff(&first);
        let diffed = first.apply(diffs);

        assert_eq!(diffed, second);
    }

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

        let diffs = second.diff(&first);
        let diffed = first.apply(diffs);

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3, second.test3);
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

            let diffs = second.diff(&first);
            let diffed = first.apply(diffs);

            assert_eq!(diffed, second);
        }
    }
}

#[cfg(all(test, feature = "nanoserde"))]
mod nanoserde_serialize {
    use nanoserde::{DeBin, SerBin};
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

        let diffs = second.diff(&first);
        let ser = SerBin::serialize_bin(&diffs);
        let diffed = first.apply(DeBin::deserialize_bin(&ser).unwrap());

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3, second.test3);
        assert_eq!(diffed.test4, second.test4);
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_serialize {
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

        let diffs = second.diff(&first);
        let ser = serde_json::to_string(&diffs).unwrap();
        let diffed = first.apply(serde_json::from_str(&ser).unwrap());

        //check that all except the skipped are changed
        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
        assert_ne!(diffed.test3, second.test3);
        assert_eq!(diffed.test4, second.test4);
    }
}
