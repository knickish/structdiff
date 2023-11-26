#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "debug_diffs")]
use std::fmt::Debug;

use std::{collections::HashMap, hash::Hash, marker::PhantomData};

use crate::StructDiff;

#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub(crate) enum UnorderedMapLikeRecursiveChangeRef<'a, K: Clone, V: StructDiff + Clone> {
    Insert((&'a K, &'a V)),
    Remove(&'a K),
    Change((&'a K, Vec<V::DiffRef<'a>>)),
}

#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub(crate) enum UnorderedMapLikeRecursiveDiffInternalRef<'a, K: Clone, V: StructDiff + Clone> {
    Replace(Vec<(&'a K, &'a V)>),
    Modify(Vec<UnorderedMapLikeRecursiveChangeRef<'a, K, V>>),
}

/// Used internally by StructDiff to track recursive changes to a map-like collection
#[repr(transparent)]
#[derive(Clone)]
#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct UnorderedMapLikeRecursiveDiffRef<'a, K: Clone, V: StructDiff + Clone>(
    UnorderedMapLikeRecursiveDiffInternalRef<'a, K, V>,
);

#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedMapLikeRecursiveChangeOwned<K: Clone, V: StructDiff> {
    Insert((K, V)),
    Remove(K),
    Change((K, Vec<V::Diff>)),
}

#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum UnorderedMapLikeRecursiveDiffInternalOwned<K: Clone, V: StructDiff> {
    Replace(Vec<(K, V)>),
    Modify(Vec<UnorderedMapLikeRecursiveChangeOwned<K, V>>),
}

/// Used internally by StructDiff to track recursive changes to a map-like collection
#[repr(transparent)]
#[derive(Clone)]
#[cfg_attr(feature = "debug_diffs", derive(Debug))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnorderedMapLikeRecursiveDiffOwned<K: Clone, V: StructDiff + Clone>(
    UnorderedMapLikeRecursiveDiffInternalOwned<K, V>,
);

impl<'a, K: Clone, V: StructDiff + Clone> From<UnorderedMapLikeRecursiveDiffRef<'a, K, V>>
    for UnorderedMapLikeRecursiveDiffOwned<K, V>
{
    fn from(value: UnorderedMapLikeRecursiveDiffRef<'a, K, V>) -> Self {
        let new_inner: UnorderedMapLikeRecursiveDiffInternalOwned<K, V> = match value.0 {
            UnorderedMapLikeRecursiveDiffInternalRef::Replace(vals) => {
                UnorderedMapLikeRecursiveDiffInternalOwned::Replace(
                    vals.into_iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
            }
            UnorderedMapLikeRecursiveDiffInternalRef::Modify(vals) => {
                let vals = vals
                    .into_iter()
                    .map(|change| match change {
                        UnorderedMapLikeRecursiveChangeRef::Insert((k, v)) => {
                            UnorderedMapLikeRecursiveChangeOwned::Insert((k.clone(), v.clone()))
                        }
                        UnorderedMapLikeRecursiveChangeRef::Remove(k) => {
                            UnorderedMapLikeRecursiveChangeOwned::Remove(k.clone())
                        }
                        UnorderedMapLikeRecursiveChangeRef::Change((k, diffs)) => {
                            let diffs = diffs
                                .into_iter()
                                .map(|x| {
                                    let ret: V::Diff = x.into();
                                    ret
                                })
                                .collect();
                            UnorderedMapLikeRecursiveChangeOwned::Change((k.clone(), diffs))
                        }
                    })
                    .collect::<Vec<UnorderedMapLikeRecursiveChangeOwned<K, V>>>();
                UnorderedMapLikeRecursiveDiffInternalOwned::Modify(vals)
            }
        };
        UnorderedMapLikeRecursiveDiffOwned(new_inner)
    }
}

fn collect_into_key_eq_map<
    'a,
    K: Hash + PartialEq + Eq + 'a,
    V: 'a,
    B: Iterator<Item = (&'a K, &'a V)>,
>(
    list: B,
) -> HashMap<&'a K, &'a V> {
    let mut map: HashMap<&K, &V> = HashMap::new();
    for (key, value) in list {
        map.insert(key, value);
    }
    map
}

enum Operation<V, VDIFF>
where
    V: StructDiff,
    VDIFF: Into<V::Diff> + Clone,
{
    Insert,
    Remove,
    Change(Vec<VDIFF>, PhantomData<V>),
}

impl<'a, K: Clone, V: StructDiff + Clone> UnorderedMapLikeRecursiveChangeRef<'a, K, V> {
    fn new(item: (&'a K, &'a V), insert_or_remove: Operation<V, V::DiffRef<'a>>) -> Self
    where
        Self: 'a,
    {
        match insert_or_remove {
            Operation::Insert => UnorderedMapLikeRecursiveChangeRef::Insert((item.0, item.1)),
            Operation::Remove => UnorderedMapLikeRecursiveChangeRef::Remove(item.0),
            Operation::Change(diff, ..) => {
                UnorderedMapLikeRecursiveChangeRef::Change((item.0, diff))
            }
        }
    }
}

pub fn unordered_hashcmp<
    'a,
    #[cfg(feature = "nanoserde")] K: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'a,
    #[cfg(not(feature = "nanoserde"))] K: Hash + Clone + PartialEq + Eq + 'a,
    V: Clone + PartialEq + StructDiff + 'a,
    B: Iterator<Item = (&'a K, &'a V)>,
>(
    previous: B,
    current: B,
    key_only: bool,
) -> Option<UnorderedMapLikeRecursiveDiffRef<'a, K, V>> {
    let (previous, mut current) = (
        collect_into_key_eq_map(previous),
        collect_into_key_eq_map(current),
    );

    // TODO look at replacing remove/insert pairs with a new type of change (K1, K2, V::Diff)
    // for space optimization. This method is fast but may send extra data over the wire.

    if key_only {
        if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
            return Some(UnorderedMapLikeRecursiveDiffRef(
                UnorderedMapLikeRecursiveDiffInternalRef::Replace(
                    current.into_iter().collect::<Vec<(&'a K, &'a V)>>(),
                ),
            ));
        }

        let mut ret: Vec<UnorderedMapLikeRecursiveChangeRef<'a, K, V>> = vec![];

        for prev_entry in previous.into_iter() {
            if current.remove_entry(prev_entry.0).is_none() {
                ret.push(UnorderedMapLikeRecursiveChangeRef::new(
                    prev_entry,
                    Operation::Remove,
                ));
            }
        }

        for add_entry in current.into_iter() {
            ret.push(UnorderedMapLikeRecursiveChangeRef::new(
                add_entry,
                Operation::Insert,
            ))
        }

        match ret.is_empty() {
            true => None,
            false => Some(UnorderedMapLikeRecursiveDiffRef(
                UnorderedMapLikeRecursiveDiffInternalRef::Modify(ret),
            )),
        }
    } else {
        if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
            return Some(UnorderedMapLikeRecursiveDiffRef(
                UnorderedMapLikeRecursiveDiffInternalRef::Replace(
                    current.into_iter().collect::<Vec<(&'a K, &'a V)>>(),
                ),
            ));
        }

        let mut ret: Vec<UnorderedMapLikeRecursiveChangeRef<'a, K, V>> = vec![];

        for prev_entry in previous.into_iter() {
            match current.remove_entry(prev_entry.0) {
                None => ret.push(UnorderedMapLikeRecursiveChangeRef::new(
                    prev_entry,
                    Operation::Remove,
                )),
                Some(current_entry) if prev_entry.1 != current_entry.1 => {
                    ret.push(UnorderedMapLikeRecursiveChangeRef::new(
                        current_entry,
                        Operation::Change(prev_entry.1.diff_ref(current_entry.1), PhantomData),
                    ))
                }
                _ => (), // no change
            }
        }

        for add_entry in current.into_iter() {
            ret.push(UnorderedMapLikeRecursiveChangeRef::new(
                add_entry,
                Operation::Insert,
            ))
        }

        match ret.is_empty() {
            true => None,
            false => Some(UnorderedMapLikeRecursiveDiffRef(
                UnorderedMapLikeRecursiveDiffInternalRef::Modify(ret),
            )),
        }
    }
}

pub fn apply_unordered_hashdiffs<
    #[cfg(feature = "nanoserde")] K: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'static,
    #[cfg(not(feature = "nanoserde"))] K: Hash + Clone + PartialEq + Eq + 'static,
    V: Clone + StructDiff + 'static,
    B: IntoIterator<Item = (K, V)>,
>(
    list: B,
    diffs: UnorderedMapLikeRecursiveDiffOwned<K, V>,
) -> Box<dyn Iterator<Item = (K, V)>> {
    let diffs = match diffs {
        UnorderedMapLikeRecursiveDiffOwned(
            UnorderedMapLikeRecursiveDiffInternalOwned::Replace(replacement),
        ) => {
            return Box::new(replacement.into_iter());
        }
        UnorderedMapLikeRecursiveDiffOwned(UnorderedMapLikeRecursiveDiffInternalOwned::Modify(
            diffs,
        )) => diffs,
    };

    let (insertions, rem): (Vec<_>, Vec<_>) = diffs
        .into_iter()
        .partition(|x| matches!(&x, UnorderedMapLikeRecursiveChangeOwned::Insert(_)));
    let (removals, changes): (Vec<_>, Vec<_>) = rem
        .into_iter()
        .partition(|x| matches!(&x, UnorderedMapLikeRecursiveChangeOwned::Remove(_)));

    let mut list_hash = HashMap::<K, V>::from_iter(list);

    for remove in removals {
        let UnorderedMapLikeRecursiveChangeOwned::Remove(key) = remove else {
            continue;
        };
        list_hash.remove(&key);
    }

    for change in changes {
        let UnorderedMapLikeRecursiveChangeOwned::Change((key, diff)) = change else {
            continue;
        };
        let Some(to_change) = list_hash.get_mut(&key) else {
            continue;
        };
        to_change.apply_mut(diff);
    }

    for insert in insertions {
        let UnorderedMapLikeRecursiveChangeOwned::Insert((key, value)) = insert else {
            continue;
        };
        list_hash.insert(key, value);
    }

    Box::new(list_hash.into_iter())
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use crate::StructDiff;

    use super::{
        DeBin, SerBin, UnorderedMapLikeRecursiveChangeOwned, UnorderedMapLikeRecursiveChangeRef,
        UnorderedMapLikeRecursiveDiffInternalOwned, UnorderedMapLikeRecursiveDiffInternalRef,
        UnorderedMapLikeRecursiveDiffOwned, UnorderedMapLikeRecursiveDiffRef,
    };

    impl<K, V> SerBin for UnorderedMapLikeRecursiveChangeOwned<K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                Self::Insert(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::Remove(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::Change(val) => {
                    2_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<K, V> SerBin for UnorderedMapLikeRecursiveChangeRef<'_, K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                Self::Insert(val) => {
                    0_u8.ser_bin(output);
                    val.0.ser_bin(output);
                    val.1.ser_bin(output);
                }
                Self::Remove(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::Change(val) => {
                    2_u8.ser_bin(output);
                    val.0.ser_bin(output);
                    val.1.ser_bin(output);
                }
            }
        }
    }

    impl<K, V> SerBin for UnorderedMapLikeRecursiveDiffOwned<K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match &self.0 {
                UnorderedMapLikeRecursiveDiffInternalOwned::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                UnorderedMapLikeRecursiveDiffInternalOwned::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<K, V> SerBin for UnorderedMapLikeRecursiveDiffRef<'_, K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match &self.0 {
                UnorderedMapLikeRecursiveDiffInternalRef::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.len().ser_bin(output);
                    for (key, value) in val {
                        key.ser_bin(output);
                        value.ser_bin(output)
                    }
                }
                UnorderedMapLikeRecursiveDiffInternalRef::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<K, V> SerBin for &UnorderedMapLikeRecursiveDiffRef<'_, K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        #[inline(always)]
        fn ser_bin(&self, output: &mut Vec<u8>) {
            (*self).ser_bin(output)
        }
    }

    impl<K, V> DeBin for UnorderedMapLikeRecursiveChangeOwned<K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedMapLikeRecursiveChangeOwned<K, V>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedMapLikeRecursiveChangeOwned::Insert(DeBin::de_bin(offset, bytes)?),
                1_u8 => UnorderedMapLikeRecursiveChangeOwned::Remove(DeBin::de_bin(offset, bytes)?),
                2_u8 => UnorderedMapLikeRecursiveChangeOwned::Change(DeBin::de_bin(offset, bytes)?),
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

    impl<K, V> DeBin for UnorderedMapLikeRecursiveDiffOwned<K, V>
    where
        K: SerBin + PartialEq + Clone + DeBin,
        V: SerBin + PartialEq + Clone + DeBin + StructDiff,
    {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedMapLikeRecursiveDiffOwned<K, V>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedMapLikeRecursiveDiffOwned(
                    UnorderedMapLikeRecursiveDiffInternalOwned::Replace(DeBin::de_bin(
                        offset, bytes,
                    )?),
                ),
                1_u8 => UnorderedMapLikeRecursiveDiffOwned(
                    UnorderedMapLikeRecursiveDiffInternalOwned::Modify(DeBin::de_bin(
                        offset, bytes,
                    )?),
                ),
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
    #[cfg(feature = "nanoserde")]
    use nanoserde::{DeBin, SerBin};
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    use crate::{Difference, StructDiff};
    use std::collections::{BTreeMap, HashMap};

    use crate as structdiff;

    #[test]
    fn test_key_only() {
        #[cfg_attr(feature = "nanoserde", derive(DeBin, SerBin))]
        #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        pub struct TestRecurse {
            recurse1: i32,
            recurse2: Option<String>,
        }

        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        struct TestCollection {
            #[difference(
                collection_strategy = "unordered_map_like",
                recurse,
                map_equality = "key_only"
            )]
            test1: HashMap<i32, TestRecurse>,
            #[difference(collection_strategy = "unordered_map_like", map_equality = "key_only")]
            test2: BTreeMap<i32, u64>,
        }

        let first = TestCollection {
            test1: vec![
                (
                    10,
                    TestRecurse {
                        recurse1: 0,
                        recurse2: None,
                    },
                ),
                (
                    15,
                    TestRecurse {
                        recurse1: 2,
                        recurse2: Some("Hello".to_string()),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 2)]
                .into_iter()
                .collect(),
        };

        let second = TestCollection {
            test1: vec![
                (
                    11,
                    TestRecurse {
                        recurse1: 0,
                        recurse2: Some("Hello World".to_string()),
                    },
                ),
                (
                    15,
                    TestRecurse {
                        recurse1: 2,
                        recurse2: Some("Hello World".to_string()),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 0)]
                .into_iter()
                .collect(),
        };

        let diffs = first.diff(&second);
        assert_eq!(diffs.len(), 2);
        let diffed = first.apply(diffs);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(
            diffed.test1.keys().collect::<Vec<_>>(),
            second.test1.keys().collect::<Vec<_>>()
        );
        assert_eq!(diffed.test1[&11], second.test1[&11]);
        assert_ne!(diffed.test1[&15], second.test1[&15]);
        assert_eq_unordered!(diffed.test2, second.test2);
    }

    #[test]
    fn test_key_value() {
        #[cfg_attr(feature = "nanoserde", derive(DeBin, SerBin))]
        #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        #[difference(setters)]
        pub struct TestRecurse {
            recurse1: i32,
            recurse2: Option<String>,
        }

        #[derive(Debug, PartialEq, Clone, Difference, Default)]
        #[difference(setters)]
        struct TestCollection {
            #[difference(
                collection_strategy = "unordered_map_like",
                map_equality = "key_and_value",
                recurse
            )]
            test1: HashMap<i32, TestRecurse>,
            #[difference(
                collection_strategy = "unordered_map_like",
                map_equality = "key_and_value"
            )]
            test2: BTreeMap<i32, u64>,
        }

        let first = TestCollection {
            test1: vec![
                (
                    10,
                    TestRecurse {
                        recurse1: 0,
                        recurse2: None,
                    },
                ),
                (
                    15,
                    TestRecurse {
                        recurse1: 2,
                        recurse2: Some("Hello".to_string()),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 2)]
                .into_iter()
                .collect(),
        };

        let second = TestCollection {
            test1: vec![
                (
                    11,
                    TestRecurse {
                        recurse1: 0,
                        recurse2: Some("Hello World".to_string()),
                    },
                ),
                (
                    15,
                    TestRecurse {
                        recurse1: 2,
                        recurse2: Some("Hello World".to_string()),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            test2: vec![(10, 0), (15, 2), (20, 0), (25, 0), (10, 0)]
                .into_iter()
                .collect(),
        };

        let diffs = first.diff(&second);
        let diffed = first.apply(diffs);

        use assert_unordered::assert_eq_unordered;
        assert_eq_unordered!(diffed.test1, second.test1);
        assert_eq_unordered!(diffed.test2, second.test2);
    }
}
