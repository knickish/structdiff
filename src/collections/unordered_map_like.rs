#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) struct UnorderedMapLikeChangeSpec<K, V, S> {
    key: K,
    value: V,
    count: S,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedMapLikeChange<K, V> {
    InsertMany(UnorderedMapLikeChangeSpec<K, V, usize>),
    RemoveMany(UnorderedMapLikeChangeSpec<K, V, usize>),
    InsertFew(UnorderedMapLikeChangeSpec<K, V, u8>),
    RemoveFew(UnorderedMapLikeChangeSpec<K, V, u8>),
    InsertSingle(K, V),
    RemoveSingle(K, V),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedMapLikeDiffInternal<K, V> {
    Replace(Vec<(K, V)>),
    Modify(Vec<UnorderedMapLikeChange<K, V>>),
}

#[repr(transparent)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnorderedMapLikeDiff<K, V>(UnorderedMapLikeDiffInternal<K, V>);

fn collect_into_map<'a, K: Hash + PartialEq + Eq + 'a, V, B: Iterator<Item = (K, V)>>(
    list: B,
) -> HashMap<K, Vec<(V, usize)>> {
    let mut map: HashMap<K, Vec<(V, usize)>> = HashMap::new();
    for (key, value) in list {
        match map.get_mut(&key) {
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

impl<K, V> UnorderedMapLikeChange<K, V> {
    fn new(item: (K, V), count: usize, insert_or_remove: InsertOrRemove) -> Self {
        debug_assert_ne!(count, 0);
        match (insert_or_remove, count) {
            (InsertOrRemove::Insert, 1) => UnorderedMapLikeChange::InsertSingle(item.0, item.1),
            (InsertOrRemove::Insert, val) if val <= u8::MAX as usize => {
                UnorderedMapLikeChange::InsertFew(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val as u8,
                })
            }
            (InsertOrRemove::Insert, val) if val > u8::MAX as usize => {
                UnorderedMapLikeChange::InsertMany(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val,
                })
            }
            (InsertOrRemove::Remove, 1) => UnorderedMapLikeChange::RemoveSingle(item.0, item.1),
            (InsertOrRemove::Remove, val) if val <= u8::MAX as usize => {
                UnorderedMapLikeChange::RemoveFew(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val as u8,
                })
            }
            (InsertOrRemove::Remove, val) if val > u8::MAX as usize => {
                UnorderedMapLikeChange::RemoveMany(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val,
                })
            }
            (_, _) => unreachable!(),
        }
    }
}

pub fn unordered_hashcmp<
    'a,
    #[cfg(feature = "nanoserde")] K: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'a,
    #[cfg(not(feature = "nanoserde"))] K: Hash + Clone + PartialEq + Eq + 'a,
    V: Clone + 'a,
    B: Iterator<Item = &'a (K, V)>,
>(
    previous: B,
    current: B,
) -> Option<UnorderedMapLikeDiff<K, V>> {
    let mut previous = collect_into_map(previous);
    let current = collect_into_map(current);

    if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
        return Some(UnorderedMapLikeDiff(
            UnorderedMapLikeDiffInternal::Replace(
                current
                    .into_iter()
                    .flat_map(|(k, v)| std::iter::repeat(k.clone()).take(v))
                    .collect(),
            ),
        ));
    }

    let mut ret: Vec<UnorderedMapLikeChange<K, V>> = vec![];

    for (&k, current_count) in current.iter() {
        match previous.remove(&k) {
            Some(prev_count) => match (*current_count as i128) - (prev_count as i128) {
                add if add > 1 => ret.push(UnorderedMapLikeChange::new(
                    k.clone(),
                    add as usize,
                    InsertOrRemove::Insert,
                )),
                add if add == 1 => ret.push(UnorderedMapLikeChange::new(
                    k.clone(),
                    add as usize,
                    InsertOrRemove::Insert,
                )),
                sub if sub < 0 => ret.push(UnorderedMapLikeChange::new(
                    k.clone(),
                    -sub as usize,
                    InsertOrRemove::Remove,
                )),
                sub if sub == -1 => ret.push(UnorderedMapLikeChange::new(
                    k.clone(),
                    -sub as usize,
                    InsertOrRemove::Remove,
                )),
                _ => (),
            },
            None => ret.push(UnorderedMapLikeChange::new(
                k.clone(),
                *current_count,
                InsertOrRemove::Insert,
            )),
        }
    }

    for (k, v) in previous.into_iter() {
        ret.push(UnorderedMapLikeChange::new(
            k.clone(),
            v,
            InsertOrRemove::Remove,
        ))
    }

    match ret.is_empty() {
        true => None,
        false => Some(UnorderedMapLikeDiff(
            UnorderedMapLikeDiffInternal::Modify(ret),
        )),
    }
}

pub fn apply_unordered_hashdiffs<
    #[cfg(feature = "nanoserde")] K: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'static,
    #[cfg(not(feature = "nanoserde"))] K: Hash + Clone + PartialEq + Eq + 'static,
    V: Clone + 'static,
    B: IntoIterator<Item = (K, V)>,
>(
    list: B,
    diffs: UnorderedMapLikeDiff<K, V>,
) -> impl Iterator<Item = (K, V)> {
    let diffs = match diffs {
        UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Replace(replacement)) => {
            return replacement.into_iter();
        }
        UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Modify(diffs)) => diffs,
    };

    let (insertions, removals): (
        Vec<UnorderedMapLikeChange<K, V>>,
        Vec<UnorderedMapLikeChange<K, V>>,
    ) = diffs.into_iter().partition(|x| match &x {
        UnorderedMapLikeChange::InsertMany(_)
        | UnorderedMapLikeChange::InsertFew(_)
        | UnorderedMapLikeChange::InsertSingle(_) => true,
        UnorderedMapLikeChange::RemoveMany(_)
        | UnorderedMapLikeChange::RemoveFew(_)
        | UnorderedMapLikeChange::RemoveSingle(_) => false,
    });
    let mut list_hash = collect_into_map(list.into_iter());

    for remove in removals {
        match remove {
            UnorderedMapLikeChange::RemoveMany(UnorderedMapLikeChangeSpec {count, key, value }) => {
                match list_hash.get_mut(&key) {
                    Some(val) if *val.1 > count => {
                        *val -= count;
                    }
                    Some(val) if *val.1 <= count => {
                        list_hash.remove(&key);
                    }
                    _ => (),
                }
            }
            UnorderedMapLikeChange::RemoveFew(UnorderedMapLikeChangeSpec {count, key, value }) => {
                match list_hash.get_mut(&key) {
                    Some(val) if *val.1 > count as usize => {
                        *val -= count as usize;
                    }
                    Some(val) if *val.1 <= count as usize => {
                        list_hash.remove(&key);
                    }
                    _ => (),
                }
            }
            UnorderedMapLikeChange::RemoveSingle(key, value) => match list_hash.get_mut(&key) {
                Some(val) if *val.1 > 1 => {
                    *val -= 1;
                }
                Some(val) if *val.1 <= 1 => {
                    list_hash.remove(&key);
                }
                _ => (),
            },
            _ => {
                #[cfg(debug_assertions)]
                panic!("Sorting failure")
            }
        }
    }

    for insertion in insertions.into_iter() {
        match insertion {
            UnorderedMapLikeChange::InsertMany(UnorderedMapLikeChangeSpec {count, key, value }) => {
                match list_hash.get_mut(&key) {
                    Some(val) => {
                        *val.1 += count;
                    }
                    None => {
                        list_hash.insert(&key, (value, count));
                    }
                }
            }
            UnorderedMapLikeChange::InsertFew(UnorderedMapLikeChangeSpec {count, key, value }) => {
                match list_hash.get_mut(&key) {
                    Some(val) => {
                        *val.1 += count as usize;
                    }
                    None => {
                        list_hash.insert(&key, (value, count));
                    }
                }
            }
            UnorderedMapLikeChange::InsertSingle(key, value) => match list_hash.get_mut(&key) {
                Some(val) => {
                    *val.1 += 1;
                }
                None => {
                    list_hash.insert(key, (value, 1));
                }
            },
            _ => {
                #[cfg(debug_assertions)]
                panic!("Sorting failure")
            }
        }
    }

    list_hash
        .into_iter()
        .flat_map(|(k, v)| std::iter::repeat(k).take(v))
        .collect::<Vec<T>>()
        .into_iter()
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::{
        DeBin, SerBin, UnorderedMapLikeChange, UnorderedMapLikeChangeSpec,
        UnorderedMapLikeDiff, UnorderedMapLikeDiffInternal,
    };

    impl<K: SerBin + DeBin, V: SerBin + DeBin> SerBin for UnorderedMapLikeChangeSpec<K, V, usize> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.key.ser_bin(output);
            self.value.ser_bin(output);
            self.count.ser_bin(output);
        }
    }

    impl<K: SerBin + DeBin, V: SerBin + DeBin> SerBin for UnorderedMapLikeChangeSpec<K, V, u8> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.key.ser_bin(output);
            self.value.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<K, V> SerBin for UnorderedMapLikeChange<K, V>
    where 
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin,
    {
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

    impl<K, V> SerBin for UnorderedMapLikeDiff<K, V>
    where 
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin,
    {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match &self.0 {
                UnorderedMapLikeDiffInternal::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedMapLikeDiffInternal::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<K, V> DeBin for UnorderedMapLikeChangeSpec<K, V, usize>
    where 
        K: DeBin,
        V: DeBin,
 {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                key: DeBin::de_bin(offset, bytes)?,
                value: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<K, V> DeBin for UnorderedMapLikeChangeSpec<K, V, u8>
    where 
        K: DeBin,
        V: DeBin,
    {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                key: DeBin::de_bin(offset, bytes)?,
                value: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<K, V> DeBin for UnorderedMapLikeChange<K, V>
    where 
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin,
    {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedMapLikeChange<K, V>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedMapLikeChange::InsertMany(DeBin::de_bin(offset, bytes)?),
                1_u8 => UnorderedMapLikeChange::RemoveMany(DeBin::de_bin(offset, bytes)?),
                2_u8 => UnorderedMapLikeChange::InsertFew(DeBin::de_bin(offset, bytes)?),
                3_u8 => UnorderedMapLikeChange::RemoveFew(DeBin::de_bin(offset, bytes)?),
                4_u8 => UnorderedMapLikeChange::InsertSingle(DeBin::de_bin(offset, bytes)?, DeBin::de_bin(offset, bytes)?),
                5_u8 => UnorderedMapLikeChange::RemoveSingle(DeBin::de_bin(offset, bytes)?, DeBin::de_bin(offset, bytes)?),
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

    impl<K, V> DeBin for UnorderedMapLikeDiff<K, V>
    where 
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin,
    {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedMapLikeDiff<K, V>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Replace(
                    DeBin::de_bin(offset, bytes)?,
                )),
                1_u8 => UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Modify(
                    DeBin::de_bin(offset, bytes)?,
                )),
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

    use super::{UnorderedMapLikeDiff, UnorderedMapLikeDiffInternal};
    use crate::{Difference, StructDiff};

    use crate as structdiff;

    #[test]
    fn test_collection_strategies() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        struct TestCollection {
            #[difference(collection)]
            test1: Vec<i32>,
            #[difference(collection)]
            test2: HashSet<i32>,
            #[difference(collection)]
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

        if let TestCollectionFields::test1(UnorderedMapLikeDiff(
            UnorderedMapLikeDiffInternal::Replace(val),
        )) = &diffs[0]
        {
            assert_eq!(val.len(), 0);
        } else {
            panic!("Collection strategy failure");
        }

        if let TestCollectionFields::test3(UnorderedMapLikeDiff(
            UnorderedMapLikeDiffInternal::Modify(val),
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
}
