#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "rustc_hash"))]
type HashMap<K, V> = std::collections::HashMap<K, V>;
#[cfg(feature = "rustc_hash")]
type HashMap<K, V> =
    std::collections::HashMap<K, V, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

use std::hash::Hash;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) struct UnorderedArrayLikeChangeSpec<T, S> {
    item: T,
    count: S,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedArrayLikeChange<T> {
    InsertMany(UnorderedArrayLikeChangeSpec<T, usize>),
    RemoveMany(UnorderedArrayLikeChangeSpec<T, usize>),
    InsertFew(UnorderedArrayLikeChangeSpec<T, u8>),
    RemoveFew(UnorderedArrayLikeChangeSpec<T, u8>),
    InsertSingle(T),
    RemoveSingle(T),
}

impl<'a, T: Clone + 'a> From<UnorderedArrayLikeChange<&'a T>> for UnorderedArrayLikeChange<T> {
    fn from(value: UnorderedArrayLikeChange<&'a T>) -> Self {
        match value {
            UnorderedArrayLikeChange::InsertMany(UnorderedArrayLikeChangeSpec { item, count }) => {
                UnorderedArrayLikeChange::InsertMany(UnorderedArrayLikeChangeSpec {
                    item: item.clone(),
                    count,
                })
            }
            UnorderedArrayLikeChange::RemoveMany(UnorderedArrayLikeChangeSpec { item, count }) => {
                UnorderedArrayLikeChange::RemoveMany(UnorderedArrayLikeChangeSpec {
                    item: item.clone(),
                    count,
                })
            }
            UnorderedArrayLikeChange::InsertFew(UnorderedArrayLikeChangeSpec { item, count }) => {
                UnorderedArrayLikeChange::InsertFew(UnorderedArrayLikeChangeSpec {
                    item: item.clone(),
                    count,
                })
            }
            UnorderedArrayLikeChange::RemoveFew(UnorderedArrayLikeChangeSpec { item, count }) => {
                UnorderedArrayLikeChange::RemoveFew(UnorderedArrayLikeChangeSpec {
                    item: item.clone(),
                    count,
                })
            }
            UnorderedArrayLikeChange::InsertSingle(v) => {
                UnorderedArrayLikeChange::InsertSingle(v.clone())
            }
            UnorderedArrayLikeChange::RemoveSingle(v) => {
                UnorderedArrayLikeChange::RemoveSingle(v.clone())
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedArrayLikeDiffInternal<T> {
    Replace(Vec<T>),
    Modify(Vec<UnorderedArrayLikeChange<T>>),
}

#[repr(transparent)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnorderedArrayLikeDiff<T>(UnorderedArrayLikeDiffInternal<T>);

impl<'a, T: Clone + 'a> From<UnorderedArrayLikeDiff<&'a T>> for UnorderedArrayLikeDiff<T> {
    fn from(value: UnorderedArrayLikeDiff<&'a T>) -> Self {
        let new_inner = match value.0 {
            UnorderedArrayLikeDiffInternal::Replace(replace) => {
                UnorderedArrayLikeDiffInternal::Replace(replace.into_iter().cloned().collect())
            }
            UnorderedArrayLikeDiffInternal::Modify(modify) => {
                UnorderedArrayLikeDiffInternal::Modify(modify.into_iter().map(Into::into).collect())
            }
        };
        Self(new_inner)
    }
}

fn collect_into_map<'a, T: Hash + PartialEq + Eq + 'a, B: Iterator<Item = T>>(
    list: B,
) -> HashMap<T, usize> {
    let mut map: HashMap<T, usize> = HashMap::default();
    map.reserve(list.size_hint().1.unwrap_or_default());

    for item in list {
        match map.get_mut(&item) {
            Some(count) => *count += 1,
            None => {
                map.insert(item, 1_usize);
            }
        }
    }
    map
}

enum InsertOrRemove {
    Insert,
    Remove,
}

impl<T> UnorderedArrayLikeChange<T> {
    fn new(item: T, count: usize, insert_or_remove: InsertOrRemove) -> Self {
        #[cfg(feature = "debug_asserts")]
        debug_assert_ne!(count, 0);
        match (insert_or_remove, count) {
            (InsertOrRemove::Insert, 1) => UnorderedArrayLikeChange::InsertSingle(item),
            (InsertOrRemove::Insert, val) if val <= u8::MAX as usize => {
                UnorderedArrayLikeChange::InsertFew(UnorderedArrayLikeChangeSpec {
                    item,
                    count: val as u8,
                })
            }
            (InsertOrRemove::Insert, val) if val > u8::MAX as usize => {
                UnorderedArrayLikeChange::InsertMany(UnorderedArrayLikeChangeSpec {
                    item,
                    count: val,
                })
            }
            (InsertOrRemove::Remove, 1) => UnorderedArrayLikeChange::RemoveSingle(item),
            (InsertOrRemove::Remove, val) if val <= u8::MAX as usize => {
                UnorderedArrayLikeChange::RemoveFew(UnorderedArrayLikeChangeSpec {
                    item,
                    count: val as u8,
                })
            }
            (InsertOrRemove::Remove, val) if val > u8::MAX as usize => {
                UnorderedArrayLikeChange::RemoveMany(UnorderedArrayLikeChangeSpec {
                    item,
                    count: val,
                })
            }
            (_, _) => unreachable!(),
        }
    }
}

pub fn unordered_hashcmp<
    'a,
    #[cfg(feature = "nanoserde")] T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'a,
    #[cfg(not(feature = "nanoserde"))] T: Hash + Clone + PartialEq + Eq + 'a,
    B: Iterator<Item = &'a T>,
>(
    previous: B,
    current: B,
) -> Option<UnorderedArrayLikeDiff<&'a T>> {
    let mut previous = collect_into_map(previous);
    let current = collect_into_map(current);

    if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
        return Some(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Replace(
                current
                    .into_iter()
                    .flat_map(|(k, v)| std::iter::repeat_n(k, v))
                    .collect(),
            ),
        ));
    }

    let mut ret: Vec<UnorderedArrayLikeChange<&T>> =
        Vec::with_capacity((previous.len() + current.len()) >> 1);

    for (k, current_count) in current.iter() {
        match previous.remove(k) {
            Some(prev_count) => match (*current_count as i128) - (prev_count as i128) {
                add if add > 1 => ret.push(UnorderedArrayLikeChange::new(
                    k,
                    add as usize,
                    InsertOrRemove::Insert,
                )),
                add if add == 1 => ret.push(UnorderedArrayLikeChange::new(
                    k,
                    add as usize,
                    InsertOrRemove::Insert,
                )),
                sub if sub < 0 => ret.push(UnorderedArrayLikeChange::new(
                    k,
                    -sub as usize,
                    InsertOrRemove::Remove,
                )),
                sub if sub == -1 => ret.push(UnorderedArrayLikeChange::new(
                    k,
                    -sub as usize,
                    InsertOrRemove::Remove,
                )),
                _ => (),
            },
            None => ret.push(UnorderedArrayLikeChange::new(
                k,
                *current_count,
                InsertOrRemove::Insert,
            )),
        }
    }

    for (k, v) in previous.into_iter() {
        ret.push(UnorderedArrayLikeChange::new(k, v, InsertOrRemove::Remove))
    }

    ret.shrink_to_fit();

    match ret.is_empty() {
        true => None,
        false => Some(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Modify(ret),
        )),
    }
}

pub fn apply_unordered_hashdiffs<
    #[cfg(feature = "nanoserde")] T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'static,
    #[cfg(not(feature = "nanoserde"))] T: Hash + Clone + PartialEq + Eq + 'static,
    B: IntoIterator<Item = T>,
>(
    list: B,
    diffs: UnorderedArrayLikeDiff<T>,
) -> Box<dyn Iterator<Item = T>>
where
    <B as IntoIterator>::IntoIter: ExactSizeIterator,
{
    let diffs = match diffs {
        UnorderedArrayLikeDiff(UnorderedArrayLikeDiffInternal::Replace(replacement)) => {
            return Box::new(replacement.into_iter());
        }
        UnorderedArrayLikeDiff(UnorderedArrayLikeDiffInternal::Modify(diffs)) => diffs,
    };

    let (insertions, removals): (
        Vec<UnorderedArrayLikeChange<T>>,
        Vec<UnorderedArrayLikeChange<T>>,
    ) = diffs.into_iter().partition(|x| match &x {
        UnorderedArrayLikeChange::InsertMany(_)
        | UnorderedArrayLikeChange::InsertFew(_)
        | UnorderedArrayLikeChange::InsertSingle(_) => true,
        UnorderedArrayLikeChange::RemoveMany(_)
        | UnorderedArrayLikeChange::RemoveFew(_)
        | UnorderedArrayLikeChange::RemoveSingle(_) => false,
    });
    let mut list_hash = collect_into_map(list.into_iter());

    for remove in removals {
        match remove {
            UnorderedArrayLikeChange::RemoveMany(UnorderedArrayLikeChangeSpec { item, count }) => {
                match list_hash.get_mut(&item) {
                    Some(val) if *val > count => {
                        *val -= count;
                    }
                    Some(val) if *val <= count => {
                        list_hash.remove(&item);
                    }
                    _ => (),
                }
            }
            UnorderedArrayLikeChange::RemoveFew(UnorderedArrayLikeChangeSpec { item, count }) => {
                match list_hash.get_mut(&item) {
                    Some(val) if *val > count as usize => {
                        *val -= count as usize;
                    }
                    Some(val) if *val <= count as usize => {
                        list_hash.remove(&item);
                    }
                    _ => (),
                }
            }
            UnorderedArrayLikeChange::RemoveSingle(item) => match list_hash.get_mut(&item) {
                Some(val) if *val > 1 => {
                    *val -= 1;
                }
                Some(val) if *val <= 1 => {
                    list_hash.remove(&item);
                }
                _ => (),
            },
            _ => {
                #[cfg(all(debug_assertions, feature = "debug_asserts"))]
                panic!("Sorting failure")
            }
        }
    }

    for insertion in insertions.into_iter() {
        match insertion {
            UnorderedArrayLikeChange::InsertMany(UnorderedArrayLikeChangeSpec { item, count }) => {
                match list_hash.get_mut(&item) {
                    Some(val) => {
                        *val += count;
                    }
                    None => {
                        list_hash.insert(item, count);
                    }
                }
            }
            UnorderedArrayLikeChange::InsertFew(UnorderedArrayLikeChangeSpec { item, count }) => {
                match list_hash.get_mut(&item) {
                    Some(val) => {
                        *val += count as usize;
                    }
                    None => {
                        list_hash.insert(item, count as usize);
                    }
                }
            }
            UnorderedArrayLikeChange::InsertSingle(item) => match list_hash.get_mut(&item) {
                Some(val) => {
                    *val += 1;
                }
                None => {
                    list_hash.insert(item, 1);
                }
            },
            _ => {
                #[cfg(all(debug_assertions, feature = "debug_asserts"))]
                panic!("Sorting failure")
            }
        }
    }

    Box::new(
        list_hash
            .into_iter()
            .flat_map(|(k, v)| std::iter::repeat_n(k, v)),
    )
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::{
        DeBin, SerBin, UnorderedArrayLikeChange, UnorderedArrayLikeChangeSpec,
        UnorderedArrayLikeDiff, UnorderedArrayLikeDiffInternal,
    };

    impl<T: SerBin + DeBin> SerBin for UnorderedArrayLikeChangeSpec<T, usize> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin + DeBin> SerBin for &UnorderedArrayLikeChangeSpec<&T, usize> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin + DeBin> SerBin for UnorderedArrayLikeChangeSpec<T, u8> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin + DeBin> SerBin for &UnorderedArrayLikeChangeSpec<&T, u8> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for UnorderedArrayLikeChange<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                Self::InsertMany(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::RemoveMany(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::InsertFew(val) => {
                    2_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::RemoveFew(val) => {
                    3_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::InsertSingle(val) => {
                    4_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::RemoveSingle(val) => {
                    5_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for &UnorderedArrayLikeChange<&T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                UnorderedArrayLikeChange::InsertMany(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeChange::RemoveMany(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeChange::InsertFew(val) => {
                    2_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeChange::RemoveFew(val) => {
                    3_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeChange::InsertSingle(val) => {
                    4_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeChange::RemoveSingle(val) => {
                    5_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for UnorderedArrayLikeDiff<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match &self.0 {
                UnorderedArrayLikeDiffInternal::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedArrayLikeDiffInternal::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for &UnorderedArrayLikeDiff<&T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match &self.0 {
                UnorderedArrayLikeDiffInternal::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.len().ser_bin(output);
                    for entry in val {
                        entry.ser_bin(output);
                    }
                }
                UnorderedArrayLikeDiffInternal::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.len().ser_bin(output);
                    for entry in val {
                        entry.ser_bin(output);
                    }
                }
            }
        }
    }

    impl<T: DeBin + SerBin> DeBin for UnorderedArrayLikeChangeSpec<T, usize> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                item: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<T: DeBin + SerBin> DeBin for UnorderedArrayLikeChangeSpec<T, u8> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                item: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<T: DeBin + PartialEq + Clone + SerBin> DeBin for UnorderedArrayLikeChange<T> {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedArrayLikeChange<T>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedArrayLikeChange::InsertMany(DeBin::de_bin(offset, bytes)?),
                1_u8 => UnorderedArrayLikeChange::RemoveMany(DeBin::de_bin(offset, bytes)?),
                2_u8 => UnorderedArrayLikeChange::InsertFew(DeBin::de_bin(offset, bytes)?),
                3_u8 => UnorderedArrayLikeChange::RemoveFew(DeBin::de_bin(offset, bytes)?),
                4_u8 => UnorderedArrayLikeChange::InsertSingle(DeBin::de_bin(offset, bytes)?),
                5_u8 => UnorderedArrayLikeChange::RemoveSingle(DeBin::de_bin(offset, bytes)?),
                _ => {
                    return core::result::Result::Err(nanoserde::DeBinErr {
                        o: *offset,
                        l: 0,
                        s: bytes.len(),
                    })
                }
            })
        }
    }

    impl<T: DeBin + PartialEq + Clone + SerBin> DeBin for UnorderedArrayLikeDiff<T> {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedArrayLikeDiff<T>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => {
                    let len: usize = DeBin::de_bin(offset, bytes)?;
                    let mut contents: Vec<T> = Vec::new();
                    for _ in 0..len {
                        let content = DeBin::de_bin(offset, bytes)?;
                        contents.push(content);
                    }
                    UnorderedArrayLikeDiff(UnorderedArrayLikeDiffInternal::Replace(contents))
                }
                1_u8 => {
                    let len: usize = DeBin::de_bin(offset, bytes)?;
                    let mut contents: Vec<UnorderedArrayLikeChange<T>> = Vec::new();
                    for _ in 0..len {
                        let content = DeBin::de_bin(offset, bytes)?;
                        contents.push(content);
                    }
                    UnorderedArrayLikeDiff(UnorderedArrayLikeDiffInternal::Modify(contents))
                }
                _ => {
                    return core::result::Result::Err(nanoserde::DeBinErr {
                        o: *offset,
                        l: 0,
                        s: bytes.len(),
                    })
                }
            })
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::{HashSet, LinkedList};

    use super::{UnorderedArrayLikeDiff, UnorderedArrayLikeDiffInternal};
    use crate::{Difference, StructDiff};

    use crate as structdiff;

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        // #[derive(Debug, PartialEq, Clone, Default)]
        // #[difference(setters)]
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

        let diffs = first.diff(&second).to_owned();

        type TestCollectionFields = <TestCollection as StructDiff>::Diff;

        if let TestCollectionFields::test1(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Replace(val),
        )) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Modify(val),
        )) = &diffs[2]
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

    #[test]
    fn test_collection_strategies_ref() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        // #[derive(Debug, PartialEq, Clone, Default)]
        #[difference(setters)]
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

        let diffs = first.diff_ref(&second).to_owned();

        type TestCollectionFields<'target> = <TestCollection as StructDiff>::DiffRef<'target>;

        if let TestCollectionFields::test1(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Replace(val),
        )) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(UnorderedArrayLikeDiff(
            UnorderedArrayLikeDiffInternal::Modify(val),
        )) = &diffs[2]
        {
            assert_eq!(val.len(), 1);
        } else {
            panic!("Collection strategy failure");
        }

        let owned = diffs.into_iter().map(Into::into).collect();
        let diffed = first.apply(owned);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
        assert_eq_unordered!(diffed.test3, second.test3);
    }
}
