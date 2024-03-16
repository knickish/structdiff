use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub(crate) enum OrderedArrayLikeChangeRef<'a, T> {
    Replace(&'a T, usize),
    Insert(&'a T, usize),
    Delete(usize, Option<usize>),
    #[allow(unused)]
    Swap(usize, usize),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum OrderedArrayLikeChangeOwned<T> {
    Replace(T, usize),
    Insert(T, usize),
    Delete(usize, Option<usize>),
    Swap(usize, usize),
}

impl<'a, T: Clone> From<OrderedArrayLikeChangeRef<'a, T>> for OrderedArrayLikeChangeOwned<T> {
    fn from(value: OrderedArrayLikeChangeRef<'a, T>) -> Self {
        match value {
            OrderedArrayLikeChangeRef::Replace(val, idx) => Self::Replace(val.to_owned(), idx),
            OrderedArrayLikeChangeRef::Insert(val, idx) => Self::Insert(val.to_owned(), idx),
            OrderedArrayLikeChangeRef::Delete(idx, range) => Self::Delete(idx, range),
            OrderedArrayLikeChangeRef::Swap(l, r) => Self::Swap(l, r),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ChangeInternal {
    NoOp(usize),
    Replace(usize),
    Insert(usize),
    Delete(usize),
}

impl ChangeInternal {
    fn cost(&self) -> usize {
        match self {
            ChangeInternal::NoOp(c) => *c,
            ChangeInternal::Replace(c) => *c,
            ChangeInternal::Insert(c) => *c,
            ChangeInternal::Delete(c) => *c,
        }
    }
}

impl<T> OrderedArrayLikeChangeOwned<T> {
    fn apply(self, container: &mut Vec<T>) {
        match self {
            OrderedArrayLikeChangeOwned::Replace(val, loc) => container[loc] = val,
            OrderedArrayLikeChangeOwned::Insert(val, loc) => container.insert(loc, val),
            OrderedArrayLikeChangeOwned::Delete(loc, None) => {
                container.remove(loc);
            }
            OrderedArrayLikeChangeOwned::Delete(l, Some(r)) => {
                container.drain(l..=r);
            }
            OrderedArrayLikeChangeOwned::Swap(l, r) => container.swap(l, r),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderedArrayLikeDiffOwned<T>(Vec<OrderedArrayLikeChangeOwned<T>>);

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OrderedArrayLikeDiffRef<'src, T>(Vec<OrderedArrayLikeChangeRef<'src, T>>);

impl<'src, T: Clone> From<OrderedArrayLikeDiffRef<'src, T>> for OrderedArrayLikeDiffOwned<T> {
    fn from(value: OrderedArrayLikeDiffRef<'src, T>) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

#[cfg(unused)]
fn print_table(table: &Vec<Vec<ChangeInternal>>) {
    for row in table {
        println!("{:?}", row)
    }
    println!("")
}

pub fn levenshtein<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: impl IntoIterator<Item = &'target T>,
    source: impl IntoIterator<Item = &'src T>,
) -> Option<OrderedArrayLikeDiffRef<'src, T>> {
    let target = target.into_iter().collect::<Vec<_>>();
    let source = source.into_iter().collect::<Vec<_>>();
    let mut table = vec![vec![ChangeInternal::NoOp(0); source.len() + 1]; target.len() + 1];

    for (i, entry) in table.iter_mut().enumerate().skip(1) {
        entry[0] = ChangeInternal::Insert(i);
    }

    for j in 0..=source.len() {
        table[0][j] = ChangeInternal::Delete(j)
    }

    // create cost table
    for target_index in 1..=target.len() {
        let target_entry = target[target_index - 1];
        for source_index in 1..=source.len() {
            let source_entry = source[source_index - 1];

            if target_entry == source_entry {
                table[target_index][source_index] =
                    ChangeInternal::NoOp(table[target_index - 1][source_index - 1].cost());
                // char matches, skip comparisons
                continue;
            }

            let insert = table[target_index - 1][source_index].cost();
            let delete = table[target_index][source_index - 1].cost();
            let replace = table[target_index - 1][source_index - 1].cost();
            let min = insert.min(delete).min(replace);

            if min == replace {
                table[target_index][source_index] = ChangeInternal::Replace(min + 1);
            } else if min == delete {
                table[target_index][source_index] = ChangeInternal::Delete(min + 1);
            } else {
                table[target_index][source_index] = ChangeInternal::Insert(min + 1);
            }
        }
    }

    let mut target_pos = target.len();
    let mut source_pos = source.len();
    let mut changelist = Vec::new();

    // collect required changes to make source into target
    while target_pos > 0 && source_pos > 0 {
        match &(table[target_pos][source_pos]) {
            ChangeInternal::NoOp(_) => {
                target_pos -= 1;
                source_pos -= 1;
            }
            ChangeInternal::Replace(_) => {
                changelist.push(OrderedArrayLikeChangeRef::Replace(
                    target[target_pos - 1],
                    source_pos - 1,
                ));
                target_pos -= 1;
                source_pos -= 1;
            }
            ChangeInternal::Insert(_) => {
                changelist.push(OrderedArrayLikeChangeRef::Insert(
                    target[target_pos - 1],
                    source_pos,
                ));
                target_pos -= 1;
            }
            ChangeInternal::Delete(_) => {
                changelist.push(OrderedArrayLikeChangeRef::Delete(source_pos - 1, None));
                source_pos -= 1;
            }
        }
        if changelist.len() == table[target.len()][source.len()].cost() {
            target_pos = 0;
            source_pos = 0;
            break;
        }
    }

    // target is longer than source, add the missing elements
    while target_pos > 0 {
        changelist.push(OrderedArrayLikeChangeRef::Insert(
            target[target_pos - 1],
            source_pos,
        ));
        target_pos -= 1;
    }

    // source is longer than target, remove the extra elements
    if source_pos > 0 {
        changelist.push(OrderedArrayLikeChangeRef::Delete(0, Some(source_pos - 1)));
    }

    match changelist.is_empty() {
        true => None,
        false => Some(OrderedArrayLikeDiffRef(changelist)),
    }
}

pub fn apply<T, L>(
    changes: impl Into<OrderedArrayLikeDiffOwned<T>>,
    existing: L,
) -> Box<dyn Iterator<Item = T>>
where
    T: Clone + 'static,
    L: IntoIterator<Item = T> + FromIterator<T>,
{
    let mut ret = existing.into_iter().collect::<Vec<_>>();

    for change in changes.into().0 {
        change.apply(&mut ret)
    }

    Box::new(ret.into_iter())
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::*;
    use nanoserde::{DeBin, SerBin};

    impl<T> OrderedArrayLikeChangeOwned<T> {
        #[inline]
        fn nanoserde_discriminant(&self) -> u8 {
            match self {
                OrderedArrayLikeChangeOwned::Replace(_, _) => 0,
                OrderedArrayLikeChangeOwned::Insert(_, _) => 1,
                OrderedArrayLikeChangeOwned::Delete(_, _) => 2,
                OrderedArrayLikeChangeOwned::Swap(_, _) => 3,
            }
        }
    }

    impl<T> OrderedArrayLikeChangeRef<'_, T> {
        #[inline]
        fn nanoserde_discriminant(&self) -> u8 {
            match self {
                OrderedArrayLikeChangeRef::Replace(_, _) => 0,
                OrderedArrayLikeChangeRef::Insert(_, _) => 1,
                OrderedArrayLikeChangeRef::Delete(_, _) => 2,
                OrderedArrayLikeChangeRef::Swap(_, _) => 3,
            }
        }
    }

    impl<T: SerBin> SerBin for OrderedArrayLikeChangeOwned<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                OrderedArrayLikeChangeOwned::Replace(val, idx) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    val.ser_bin(output);
                    idx.ser_bin(output);
                }
                OrderedArrayLikeChangeOwned::Insert(val, idx) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    val.ser_bin(output);
                    idx.ser_bin(output);
                }
                OrderedArrayLikeChangeOwned::Delete(idx, opt_start) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    idx.ser_bin(output);
                    opt_start.ser_bin(output);
                }
                OrderedArrayLikeChangeOwned::Swap(l, r) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    l.ser_bin(output);
                    r.ser_bin(output);
                }
            }
        }
    }

    impl<T: SerBin> SerBin for OrderedArrayLikeChangeRef<'_, T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                OrderedArrayLikeChangeRef::Replace(val, idx) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    val.ser_bin(output);
                    idx.ser_bin(output);
                }
                OrderedArrayLikeChangeRef::Insert(val, idx) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    val.ser_bin(output);
                    idx.ser_bin(output);
                }
                OrderedArrayLikeChangeRef::Delete(idx, opt_start) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    idx.ser_bin(output);
                    opt_start.ser_bin(output);
                }
                OrderedArrayLikeChangeRef::Swap(l, r) => {
                    self.nanoserde_discriminant().ser_bin(output);
                    l.ser_bin(output);
                    r.ser_bin(output);
                }
            }
        }
    }

    impl<T: DeBin> DeBin for OrderedArrayLikeChangeOwned<T> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            match <u8 as DeBin>::de_bin(offset, bytes)? {
                0 => {
                    let val = <T as DeBin>::de_bin(offset, bytes)?;
                    let idx = <usize as DeBin>::de_bin(offset, bytes)?;
                    Ok(OrderedArrayLikeChangeOwned::Replace(val, idx))
                }
                1 => {
                    let val = <T as DeBin>::de_bin(offset, bytes)?;
                    let idx = <usize as DeBin>::de_bin(offset, bytes)?;
                    Ok(OrderedArrayLikeChangeOwned::Insert(val, idx))
                }
                2 => {
                    let idx = <usize as DeBin>::de_bin(offset, bytes)?;
                    let opt_start = <Option<usize> as DeBin>::de_bin(offset, bytes)?;
                    Ok(OrderedArrayLikeChangeOwned::Delete(idx, opt_start))
                }
                3 => {
                    let l = <usize as DeBin>::de_bin(offset, bytes)?;
                    let r = <usize as DeBin>::de_bin(offset, bytes)?;
                    Ok(OrderedArrayLikeChangeOwned::Swap(l, r))
                }
                _ => Err(nanoserde::DeBinErr {
                    o: *offset - 1,
                    l: 1,
                    s: 1,
                }),
            }
        }
    }

    impl<T: SerBin> SerBin for OrderedArrayLikeDiffRef<'_, T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.0.ser_bin(output);
        }
    }

    impl<T: SerBin> SerBin for OrderedArrayLikeDiffOwned<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.0.ser_bin(output);
        }
    }

    impl<T: DeBin> DeBin for OrderedArrayLikeDiffOwned<T> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            let ret = <Vec<_> as DeBin>::de_bin(offset, bytes)?;
            Ok(Self(ret))
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::LinkedList;

    use super::*;
    use crate as structdiff;
    use nanorand::{Rng, WyRand};

    use structdiff::{Difference, StructDiff};

    #[test]
    fn test_string() {
        let s1 = String::from("tested");
        let s2 = String::from("testing");

        let s1_vec = s1.chars().collect::<Vec<_>>();
        let s2_vec = s2.chars().collect::<Vec<_>>();

        let Some(changes) = levenshtein(&s1_vec, &s2_vec) else {
            assert_eq!(&s1_vec, &s2_vec);
            return;
        };

        let changed = apply(changes, s2.chars().collect::<Vec<_>>())
            .into_iter()
            .collect::<String>();
        assert_eq!(s1, changed)
    }

    #[test]
    fn test_one_empty_string() {
        let s1: Vec<char> = "abc".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        let Some(changes) = levenshtein(&s1, &s2) else {
            assert_eq!(s1, s2);
            return;
        };

        assert_eq!(
            changes.0.len(),
            s1.len(),
            "Should require deletions for all characters in the non-empty string."
        );
    }

    #[test]
    fn test_empty_strings() {
        let s1: Vec<char> = "".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        let Some(changes) = levenshtein(&s1, &s2) else {
            assert_eq!(s1, s2);
            return;
        };

        assert!(
            changes.0.is_empty(),
            "No changes should be needed for two empty strings."
        );
    }

    #[test]
    fn test_identical_strings() {
        let s1: Vec<char> = "rust".chars().collect();
        let changes = levenshtein(&s1, &s1);
        assert!(
            changes.is_none(),
            "No changes should be needed for identical strings."
        );
    }

    #[test]
    fn test_random_strings() {
        let mut rng = WyRand::new();
        let charset = "abcdefghijklmnopqrstuvwxyz";
        let charset_len = charset.chars().count();

        for _ in 0..100 {
            // Generate and test 100 pairs of strings
            let s1_len = rng.generate_range(0..10); // Keep string lengths manageable
            let s2_len = rng.generate_range(0..10);

            let s1: String = (0..s1_len)
                .map(|_| {
                    charset
                        .chars()
                        .nth(rng.generate_range(0..charset_len))
                        .unwrap()
                })
                .collect();
            let s2: String = (0..s2_len)
                .map(|_| {
                    charset
                        .chars()
                        .nth(rng.generate_range(0..charset_len))
                        .unwrap()
                })
                .collect();

            let s1_vec: Vec<char> = s1.chars().collect();
            let s2_vec: Vec<char> = s2.chars().collect();

            let Some(changes) = levenshtein(&s1_vec, &s2_vec) else {
                assert_eq!(&s1_vec, &s2_vec);
                return;
            };

            let changed = apply(changes, s2_vec.clone())
                .into_iter()
                .collect::<Vec<char>>();
            assert_eq!(s1_vec, changed)
        }
    }

    #[test]
    fn test_random_f64_lists() {
        let mut rng = WyRand::new();

        for _ in 0..100 {
            // Generate and test 100 pairs of lists
            let list1_len = rng.generate_range(0..10);
            let list2_len = rng.generate_range(0..10);

            let list1: Vec<f64> = (0..list1_len).map(|_| rng.generate::<f64>()).collect();
            let list2: Vec<f64> = (0..list2_len).map(|_| rng.generate::<f64>()).collect();

            let Some(changes) = levenshtein(&list1, &list2) else {
                assert_eq!(&list1, &list2);
                return;
            };

            let changed = apply(changes, list2.clone()).collect::<Vec<_>>();
            assert_eq!(list1, changed)
        }
    }

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Default, Difference)]
        #[difference(setters)]
        struct TestCollection {
            #[difference(collection_strategy = "ordered_array_like")]
            test1: Vec<i32>,
            #[difference(collection_strategy = "ordered_array_like")]
            test2: LinkedList<i32>,
        }

        let first = TestCollection {
            test1: vec![10, 15, 20, 25, 30],
            test2: vec![10, 15, 17].into_iter().collect(),
        };

        let second = TestCollection {
            test1: Vec::default(),
            test2: vec![10, 15, 17, 19].into_iter().collect(),
        };

        let diffs = first.diff(&second).to_owned();

        type TestCollectionFields = <TestCollection as StructDiff>::Diff;

        if let TestCollectionFields::test1(OrderedArrayLikeDiffOwned(val)) = &diffs[0] {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test2(OrderedArrayLikeDiffOwned(val)) = &diffs[1] {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let diffed = first.apply(diffs);

        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
    }

    #[test]
    fn test_collection_strategies_ref() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        #[difference(setters)]
        struct TestCollection {
            #[difference(collection_strategy = "ordered_array_like")]
            test1: Vec<i32>,
            #[difference(collection_strategy = "ordered_array_like")]
            test2: LinkedList<i32>,
        }

        let first = TestCollection {
            test1: vec![10, 15, 20, 25, 30],
            test2: vec![10, 15, 17].into_iter().collect(),
        };

        let second = TestCollection {
            test1: Vec::default(),
            test2: vec![10, 15, 17, 19].into_iter().collect(),
        };

        let diffs = first.diff_ref(&second).to_owned();

        type TestCollectionFields<'target> = <TestCollection as StructDiff>::DiffRef<'target>;

        if let TestCollectionFields::test1(OrderedArrayLikeDiffRef(val)) = &diffs[0] {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test2(OrderedArrayLikeDiffRef(val)) = &diffs[1] {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let owned = diffs.into_iter().map(Into::into).collect();
        let diffed = first.apply(owned);

        assert_eq!(diffed.test1, second.test1);
        assert_eq!(diffed.test2, second.test2);
    }
}
