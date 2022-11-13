#![allow(dead_code)]
#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemChangeSpec<T> {
    item: T,
    count: usize,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnorderedItemChange<T> {
    Insert(ItemChangeSpec<T>),
    Remove(ItemChangeSpec<T>),
    InsertSingle(T),
    RemoveSingle(T),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnorderedCollectionDiff<T> {
    Replace(Vec<T>),
    Modify(Vec<UnorderedItemChange<T>>),
}

fn collect_into_map<'a, T: Hash + PartialEq + Eq + 'a, B: Iterator<Item = T>>(
    list: B,
) -> HashMap<T, usize> {
    let mut map: HashMap<T, usize> = HashMap::new();
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

pub fn unordered_hashcmp<
    'a,
    #[cfg(feature = "nanoserde")] T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'a,
    #[cfg(not(feature = "nanoserde"))] T: Hash + Clone + PartialEq + Eq + 'a,
    B: Iterator<Item = &'a T>,
>(
    previous: B,
    current: B,
) -> Option<UnorderedCollectionDiff<T>> {
    let mut previous = collect_into_map(previous);
    let current = collect_into_map(current);

    if (current.len() as isize) < ((previous.len() as isize) - (current.len() as isize)) {
        return Some(UnorderedCollectionDiff::Replace(
            current
                .into_iter()
                .flat_map(|(k, v)| std::iter::repeat(k.clone()).take(v))
                .collect(),
        ));
    }

    let mut ret: Vec<UnorderedItemChange<T>> = vec![];

    for (&k, current_count) in current.iter() {
        match previous.remove(&k) {
            Some(prev_count) => match (*current_count as isize) - (prev_count as isize) {
                add if add > 1 => ret.push(UnorderedItemChange::Insert(ItemChangeSpec {
                    item: k.clone(),
                    count: add as usize,
                })),
                add if add == 1 => ret.push(UnorderedItemChange::InsertSingle(k.clone())),
                sub if sub < 0 => ret.push(UnorderedItemChange::Remove(ItemChangeSpec {
                    item: k.clone(),
                    count: -sub as usize,
                })),
                sub if sub == -1 => ret.push(UnorderedItemChange::RemoveSingle(k.clone())),
                _ => (),
            },
            None => ret.push(match *current_count {
                1 => UnorderedItemChange::InsertSingle(k.clone()),
                _ => UnorderedItemChange::Insert(ItemChangeSpec {
                    item: k.clone(),
                    count: *current_count,
                }),
            }),
        }
    }

    for (k, v) in previous.into_iter() {
        ret.push(UnorderedItemChange::Remove(ItemChangeSpec {
            item: k.clone(),
            count: v,
        }))
    }

    match ret.is_empty() {
        true => None,
        false => Some(UnorderedCollectionDiff::Modify(ret)),
    }
}

pub fn apply_unordered_hashdiffs<
    #[cfg(feature = "nanoserde")] T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'static,
    #[cfg(not(feature = "nanoserde"))] T: Hash + Clone + PartialEq + Eq + 'static,
    B: IntoIterator<Item = T>,
>(
    list: B,
    diffs: UnorderedCollectionDiff<T>,
) -> impl Iterator<Item = T> {
    let diffs = match diffs {
        UnorderedCollectionDiff::Replace(replacement) => {
            return replacement.into_iter();
        }
        UnorderedCollectionDiff::Modify(diffs) => diffs,
    };

    let (insertions, removals): (Vec<UnorderedItemChange<T>>, Vec<UnorderedItemChange<T>>) =
        diffs.into_iter().partition(|x| match &x {
            UnorderedItemChange::Insert(_) | UnorderedItemChange::InsertSingle(_) => true,
            UnorderedItemChange::Remove(_) | UnorderedItemChange::RemoveSingle(_) => false,
        });
    let mut list_hash = collect_into_map(list.into_iter());

    for remove in removals {
        match remove {
            UnorderedItemChange::Remove(ItemChangeSpec { item, count }) => {
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
            UnorderedItemChange::RemoveSingle(item) => match list_hash.get_mut(&item) {
                Some(val) if *val > 1 => {
                    *val -= 1;
                }
                Some(val) if *val <= 1 => {
                    list_hash.remove(&item);
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
            UnorderedItemChange::Insert(ItemChangeSpec { item, count }) => {
                match list_hash.get_mut(&item) {
                    Some(val) => {
                        *val += count;
                    }
                    None => {
                        list_hash.insert(item, count);
                    }
                }
            }
            UnorderedItemChange::InsertSingle(item) => match list_hash.get_mut(&item) {
                Some(val) => {
                    *val += 1;
                }
                None => {
                    list_hash.insert(item, 1);
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
    use super::{DeBin, ItemChangeSpec, SerBin, UnorderedCollectionDiff, UnorderedItemChange};

    impl<T: SerBin + DeBin> SerBin for ItemChangeSpec<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for UnorderedItemChange<T> {
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
                Self::InsertSingle(val) => {
                    2_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::RemoveSingle(val) => {
                    3_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<T: SerBin + PartialEq + Clone + DeBin> SerBin for UnorderedCollectionDiff<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                Self::Replace(val) => {
                    0_u8.ser_bin(output);
                    val.ser_bin(output);
                }
                Self::Modify(val) => {
                    1_u8.ser_bin(output);
                    val.ser_bin(output);
                }
            }
        }
    }

    impl<T: DeBin + SerBin> DeBin for ItemChangeSpec<T> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                item: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<T: DeBin + PartialEq + Clone + SerBin> DeBin for UnorderedItemChange<T> {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedItemChange<T>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedItemChange::Insert(DeBin::de_bin(offset, bytes)?),
                1_u8 => UnorderedItemChange::Remove(DeBin::de_bin(offset, bytes)?),
                2_u8 => UnorderedItemChange::InsertSingle(DeBin::de_bin(offset, bytes)?),
                3_u8 => UnorderedItemChange::RemoveSingle(DeBin::de_bin(offset, bytes)?),
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

    impl<T: DeBin + PartialEq + Clone + SerBin> DeBin for UnorderedCollectionDiff<T> {
        fn de_bin(
            offset: &mut usize,
            bytes: &[u8],
        ) -> Result<UnorderedCollectionDiff<T>, nanoserde::DeBinErr> {
            let id: u8 = DeBin::de_bin(offset, bytes)?;
            core::result::Result::Ok(match id {
                0_u8 => UnorderedCollectionDiff::Replace(DeBin::de_bin(offset, bytes)?),
                1_u8 => UnorderedCollectionDiff::Modify(DeBin::de_bin(offset, bytes)?),
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
