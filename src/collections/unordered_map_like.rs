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
    InsertSingle((K, V)),
    RemoveSingle((K, V)),
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

fn collect_into_key_eq_map<
    'a,
    K: Hash + PartialEq + Eq + 'a,
    V: 'a,
    B: Iterator<Item = (&'a K, &'a V)>,
>(
    list: B,
) -> HashMap<&'a K, (&'a V, usize)> {
    let mut map: HashMap<&K, (&V, usize)> = HashMap::new();
    for (key, value) in list {
        match map.get_mut(&key) {
            Some((_, count)) => *count += 1,
            None => {
                map.insert(key, (value, 1_usize));
            }
        }
    }
    map
}

fn collect_into_key_value_eq_map<
    'a,
    K: Hash + PartialEq + Eq + 'a,
    V: PartialEq + 'a,
    B: Iterator<Item = (&'a K, &'a V)>,
>(
    list: B,
) -> HashMap<&'a K, (&'a V, usize)> {
    let mut map: HashMap<&K, (&V, usize)> = HashMap::new();
    for (key, value) in list {
        match map.get_mut(&key) {
            Some((ref current_val, count)) => match current_val == &value {
                true => *count += 1,
                false => {
                    map.insert(key, (value, 1_usize));
                }
            },
            None => {
                map.insert(key, (value, 1_usize));
            }
        }
    }
    map
}

enum Operation {
    Insert,
    Remove,
}

impl<K, V> UnorderedMapLikeChange<K, V> {
    fn new(item: (K, V), count: usize, insert_or_remove: Operation) -> Self {
        debug_assert_ne!(count, 0);
        match (insert_or_remove, count) {
            (Operation::Insert, 1) => UnorderedMapLikeChange::InsertSingle((item.0, item.1)),
            (Operation::Insert, val) if val <= u8::MAX as usize => {
                UnorderedMapLikeChange::InsertFew(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val as u8,
                })
            }
            (Operation::Insert, val) if val > u8::MAX as usize => {
                UnorderedMapLikeChange::InsertMany(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val,
                })
            }
            (Operation::Remove, 1) => UnorderedMapLikeChange::RemoveSingle((item.0, item.1)),
            (Operation::Remove, val) if val <= u8::MAX as usize => {
                UnorderedMapLikeChange::RemoveFew(UnorderedMapLikeChangeSpec {
                    key: item.0,
                    value: item.1,
                    count: val as u8,
                })
            }
            (Operation::Remove, val) if val > u8::MAX as usize => {
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
    #[cfg(feature = "nanoserde")] K: Hash + Clone + PartialEq + Eq + SerBin + DeBin + std::fmt::Debug + 'a,
    #[cfg(not(feature = "nanoserde"))] K: Hash + Clone + PartialEq + Eq + 'a,
    V: Clone + PartialEq + std::fmt::Debug + 'a,
    B: Iterator<Item = (&'a K, &'a V)>,
>(
    previous: B,
    current: B,
    key_only: bool,
) -> Option<UnorderedMapLikeDiff<K, V>> {
    let (mut previous, current) = if key_only {
        (
            collect_into_key_eq_map(previous),
            collect_into_key_eq_map(current),
        )
    } else {
        (
            collect_into_key_value_eq_map(previous),
            collect_into_key_value_eq_map(current),
        )
    };

    if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
        return Some(UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Replace(
            current
                .into_iter()
                .flat_map(|(k, (v, count))| std::iter::repeat((k.clone(), v.clone())).take(count))
                .collect(),
        )));
    }

    let mut ret: Vec<UnorderedMapLikeChange<K, V>> = vec![];

    for (&k, &(v, current_count)) in current.iter() {
        match previous.remove(&k) {
            Some((prev_val, prev_count)) if prev_val == v => {
                match (current_count as i128) - (prev_count as i128) {
                    add if add > 1 => ret.push(UnorderedMapLikeChange::new(
                        (k.clone(), v.clone()),
                        add as usize,
                        Operation::Insert,
                    )),
                    add if add == 1 => ret.push(UnorderedMapLikeChange::new(
                        (k.clone(), v.clone()),
                        add as usize,
                        Operation::Insert,
                    )),
                    sub if sub < 0 => ret.push(UnorderedMapLikeChange::new(
                        (k.clone(), v.clone()),
                        -sub as usize,
                        Operation::Remove,
                    )),
                    sub if sub == -1 => ret.push(UnorderedMapLikeChange::new(
                        (k.clone(), v.clone()),
                        -sub as usize,
                        Operation::Remove,
                    )),
                    _ => (),
                }
            }
            Some((prev_val, prev_count)) if prev_val != v => {
                ret.push(UnorderedMapLikeChange::new(
                    (k.clone(), prev_val.clone()),
                    prev_count,
                    Operation::Remove,
                ));
                ret.push(UnorderedMapLikeChange::new(
                    (k.clone(), v.clone()),
                    current_count,
                    Operation::Insert,
                ));
            }
            Some(_) => unreachable!(),
            None => ret.push(UnorderedMapLikeChange::new(
                (k.clone(), v.clone()),
                current_count,
                Operation::Insert,
            )),
        }
    }

    for (k, (v, count)) in previous.into_iter() {
        ret.push(UnorderedMapLikeChange::new(
            (k.clone(), v.clone()),
            count,
            Operation::Remove,
        ))
    }

    match ret.is_empty() {
        true => None,
        false => Some(UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Modify(
            ret,
        ))),
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
) -> Box<dyn Iterator<Item = (K, V)>> {
    let diffs = match diffs {
        UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Replace(replacement)) => {
            return Box::new(replacement.into_iter());
        }
        UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Modify(diffs)) => diffs,
    };

    let (insertions, removals): (Vec<_>, Vec<_>) = diffs.into_iter().partition(|x| match &x {
        UnorderedMapLikeChange::InsertMany(_)
        | UnorderedMapLikeChange::InsertFew(_)
        | UnorderedMapLikeChange::InsertSingle(_) => true,
        UnorderedMapLikeChange::RemoveMany(_)
        | UnorderedMapLikeChange::RemoveFew(_)
        | UnorderedMapLikeChange::RemoveSingle(_) => false,
    });
    let holder: Vec<_> = list.into_iter().collect();
    // let ref_holder: Vec<_> = holder.iter().map(|(k, v)| (k, v)).collect();
    let mut list_hash = collect_into_key_eq_map(holder.iter().map(|t| (&t.0, &t.1)));

    for remove in removals {
        match remove {
            UnorderedMapLikeChange::RemoveMany(UnorderedMapLikeChangeSpec {
                count, key, ..
            }) => match list_hash.get_mut(&key) {
                Some(val) if val.1 > count => {
                    val.1 -= count;
                }
                Some(val) if val.1 <= count => {
                    list_hash.remove(&key);
                }
                _ => (),
            },
            UnorderedMapLikeChange::RemoveFew(UnorderedMapLikeChangeSpec {
                count, key, ..
            }) => match list_hash.get_mut(&key) {
                Some(val) if val.1 > count as usize => {
                    val.1 -= count as usize;
                }
                Some(val) if val.1 <= count as usize => {
                    list_hash.remove(&key);
                }
                _ => (),
            },
            UnorderedMapLikeChange::RemoveSingle((key, ..)) => match list_hash.get_mut(&key) {
                Some(val) if val.1 > 1 => {
                    val.1 -= 1;
                }
                Some(val) if val.1 <= 1 => {
                    list_hash.remove(&key);
                }
                _ => (),
            },
            _ => unreachable!(),
        }
    }

    for insertion in insertions.iter() {
        match insertion {
            UnorderedMapLikeChange::InsertMany(UnorderedMapLikeChangeSpec {
                count,
                key,
                value,
            }) => match list_hash.get_mut(&key) {
                Some(val) => {
                    val.1 += count;
                }
                None => {
                    list_hash.insert(key, (value, *count));
                }
            },
            UnorderedMapLikeChange::InsertFew(UnorderedMapLikeChangeSpec { count, key, value }) => {
                match list_hash.get_mut(&key) {
                    Some(val) => {
                        val.1 += *count as usize;
                    }
                    None => {
                        list_hash.insert(key, (value, *count as usize));
                    }
                }
            }
            UnorderedMapLikeChange::InsertSingle((key, value)) => match list_hash.get_mut(&key) {
                Some(val) => {
                    val.1 += 1;
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

    Box::new(
        list_hash
            .into_iter()
            .flat_map(|(k, (v, count))| std::iter::repeat((k.clone(), v.clone())).take(count))
            .collect::<Vec<_>>()
            .into_iter(),
    )
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::{
        DeBin, SerBin, UnorderedMapLikeChange, UnorderedMapLikeChangeSpec, UnorderedMapLikeDiff,
        UnorderedMapLikeDiffInternal,
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
                4_u8 => UnorderedMapLikeChange::InsertSingle(DeBin::de_bin(offset, bytes)?),
                5_u8 => UnorderedMapLikeChange::RemoveSingle(DeBin::de_bin(offset, bytes)?),
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
                0_u8 => UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Replace(DeBin::de_bin(
                    offset, bytes,
                )?)),
                1_u8 => UnorderedMapLikeDiff(UnorderedMapLikeDiffInternal::Modify(DeBin::de_bin(
                    offset, bytes,
                )?)),
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

#[cfg(not(feature = "nanoserde"))]
#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, HashMap};

    use super::{UnorderedMapLikeDiff, UnorderedMapLikeDiffInternal};
    use crate::{Difference, StructDiff};

    use crate as structdiff;

    #[test]
    fn test_key_only() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        #[difference(setters)]
        struct TestCollection {
            #[difference(collection_strategy = "unordered_map_like")]
            test1: HashMap<i32, i32>,
            #[difference(collection_strategy = "unordered_map_like")]
            test2: BTreeMap<i32, i32>,
            #[difference(collection_strategy = "unordered_map_like")]
            test3: HashMap<i32, i32>,
            #[difference(collection_strategy = "unordered_map_like")]
            test4: BTreeMap<i32, i32>,
        }

        let first = TestCollection {
            test1: vec![(10, 0), (15, 2), (20, 0), (25, 0), (30, 15)]
                .into_iter()
                .collect(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 0)]
                .into_iter()
                .collect(),
            test3: vec![(10, 0), (15, 2), (20, 0), (25, 0), (30, 15)]
                .into_iter()
                .collect(),
            test4: vec![(10, 0), (15, 2), (20, 0), (25, 0)]
                .into_iter()
                .collect(),
        };

        let second = TestCollection {
            test1: Default::default(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 0), (10, 0)]
                .into_iter()
                .collect(),
            test3: vec![(10, 0), (15, 2), (20, 0), (25, 0)]
                .into_iter()
                .collect(),
            test4: vec![(10, 0), (15, 2), (20, 0), (25, 0), (15, 2)]
                .into_iter()
                .collect(), // add duplicated field
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

        let diffed = first.apply(diffs);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
        assert_eq_unordered!(diffed.test3, second.test3);
        assert_eq_unordered!(diffed.test4, second.test4);
    }

    #[test]
    fn test_key_value() {
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        #[difference(setters)]
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

        let diffed = first.apply(diffs);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
    }
}
