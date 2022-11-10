#![allow(dead_code)]
use std::{collections::HashMap, hash::Hash};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "nanoserde")]
use nanoserde::{SerBin, DeBin};


// pub(crate) enum ItemIdentifier<T: PartialEq + Clone> {
//     Index(usize),
//     Value(T),
// }

// pub(crate) enum OrderedItemChange<T: PartialEq + Clone> {
//     Insert {
//         item: T,
//         indices: Vec<usize>,
//     },
//     Duplicate {
//         source: ItemIdentifier<T>,
//         destinations: Vec<usize>,
//     },
//     Remove(ItemIdentifier<T>),
//     Swap((usize, usize)),
// }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemChangeSpec<T> {
    item: T,
    count: usize,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnorderedItemChange<T>
{
    Insert(ItemChangeSpec<T>),
    Remove(ItemChangeSpec<T>),
}

fn collect_into_map<
'a, 
T: Hash + PartialEq + Eq + 'a,
B: Iterator<Item = T>
>(
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
    #[cfg(feature = "nanoserde")] 
    T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'a,
    #[cfg(not(feature = "nanoserde"))] 
    T: Hash + Clone + PartialEq + Eq + 'a,
    B:Iterator<Item = &'a T>,
>(
    previous: B,
    current: B,
) -> Vec<UnorderedItemChange<T>> 
    {
    let mut previous = collect_into_map(previous);
    let current = collect_into_map(current);

    let mut ret: Vec<UnorderedItemChange<T>> = vec![];

    for (&k, current_count) in current.iter() {
        match previous.remove(&k) {
            Some(prev_count) => match (*current_count as isize) - (prev_count as isize) {
                add if add > 0 => ret.push(UnorderedItemChange::Insert(ItemChangeSpec {
                    item: k.clone(),
                    count: add as usize,
                })),
                sub if sub < 0 => ret.push(UnorderedItemChange::Remove(ItemChangeSpec {
                    item: k.clone(),
                    count: -sub as usize,
                })),
                _ => (),
            },
            None => ret.push(UnorderedItemChange::Insert(ItemChangeSpec {
                item: k.clone(),
                count: *current_count,
            })),
        }
    }

    for (k, v) in previous.into_iter() {
        ret.push(UnorderedItemChange::Remove(ItemChangeSpec {
            item: k.clone(),
            count: v,
        }))
    }

    ret
}

pub fn apply_unordered_hashdiffs<
    #[cfg(feature = "nanoserde")] T: Hash + Clone + PartialEq + Eq + SerBin + DeBin + 'static,
    #[cfg(not(feature = "nanoserde"))] T: Hash + Clone + PartialEq + Eq + 'static,
    B: IntoIterator<Item = T>,
>(
    list: B,
    diffs: Vec<UnorderedItemChange<T>>,
) -> impl Iterator<Item = T> {
    let (insertions, removals): (Vec<UnorderedItemChange<T>>, Vec<UnorderedItemChange<T>>) =
        diffs.into_iter().partition(|x| match &x {
            UnorderedItemChange::Insert(_) => true,
            UnorderedItemChange::Remove(_) => false,
        });
    let mut list_hash = collect_into_map(list.into_iter());

    for remove in removals {
        if let UnorderedItemChange::Remove(ItemChangeSpec { item, count }) = remove {
            match list_hash.get_mut(&item) {
                Some(val) if *val > count => {
                    *val -= count;
                }
                Some(val) if *val <= count => {
                    drop(val);
                    list_hash.remove(&item);
                }
                _ => (),
            }
        }
    }

    for insertion in insertions.into_iter() {
        if let UnorderedItemChange::Insert(ItemChangeSpec { item, count }) = insertion {
            match list_hash.get_mut(&item) {
                Some(val) => {
                    *val += count;
                }
                None => {
                    list_hash.insert(item, count);
                }
            }
        }
    }

    list_hash
        .into_iter()
        .flat_map(|(k, v)| std::iter::repeat(k).take(v))
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::{UnorderedItemChange, ItemChangeSpec, SerBin, DeBin};

    impl<T: SerBin + DeBin> SerBin for ItemChangeSpec<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            self.item.ser_bin(output);
            self.count.ser_bin(output)
        }
    }

    impl<T: SerBin+PartialEq+Clone + DeBin> SerBin for UnorderedItemChange<T> {
        fn ser_bin(&self, output: &mut Vec<u8>) {
            match self {
                Self::Insert(val)=> {0_u16.ser_bin(output);val.ser_bin(output);},
                Self::Remove(val)=> {1_u16.ser_bin(output);val.ser_bin(output);},
            }
        }
    }

    // impl<'a, T: SerBin+PartialEq+Clone + DeBin> SerBin for &'a UnorderedItemChange<T> {
    //     fn ser_bin(&self, output: &mut Vec<u8>) {
    //         match self {
    //             UnorderedItemChange::Insert(val)=> val.ser_bin(output),
    //             UnorderedItemChange::Remove(val)=> val.ser_bin(output),
    //         }
    //     }
    // }

    impl<T: DeBin + SerBin> DeBin for ItemChangeSpec<T> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<Self, nanoserde::DeBinErr> {
            core::result::Result::Ok(Self {
                item: DeBin::de_bin(offset, bytes)?,
                count: DeBin::de_bin(offset, bytes)?,
            })
        }
    }

    impl<T: DeBin + PartialEq + Clone + SerBin> DeBin for UnorderedItemChange<T> {
        fn de_bin(offset: &mut usize, bytes: &[u8]) -> Result<UnorderedItemChange<T>, nanoserde::DeBinErr> {
            let id: u16 = DeBin::de_bin(offset,bytes)?;
            core::result::Result::Ok(match id {
                0u16 => UnorderedItemChange::Insert(DeBin::de_bin(offset, bytes)?),
                1u16 => UnorderedItemChange::Remove(DeBin::de_bin(offset, bytes)?),
                _ => return core::result::Result::Err(nanoserde::DeBinErr{o:*offset, l:0, s:bytes.len()})
            })
        }
    }
}