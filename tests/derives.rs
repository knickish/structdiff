#![allow(clippy::type_complexity)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
    num::Wrapping,
};
use structdiff::Difference;

// Trying to come up with all the edge cases that might be relevant
#[allow(dead_code)]
#[cfg(not(any(feature = "serde", feature = "nanoserde")))]
#[derive(Difference)]
// #[difference(setters)]
pub struct TestDeriveAll<
    'a,
    'b: 'a,
    A: PartialEq + 'static,
    const C: usize,
    B,
    D,
    LM: Ord = Option<isize>,
    const N: usize = 4,
> where
    A: core::hash::Hash + std::cmp::Eq + Default,
    LM: Ord + IntoIterator<Item = isize>,
    [A; N]: Default,
    [B; C]: Default,
    [i32; N]: Default,
    [B; N]: Default,
    dyn Fn(&B): PartialEq + Clone + core::fmt::Debug,
    (dyn core::fmt::Debug + Send + 'static): Debug,
{
    f1: (),
    f2: [A; N],
    f3: [i32; N],
    f4: BTreeMap<LM, BTreeSet<<LM as IntoIterator>::Item>>,
    f5: Option<(A, Option<&'a <LM as IntoIterator>::Item>)>,
    f6: HashMap<A, BTreeSet<LM>>,
    f7: Box<(Vec<LM>, HashSet<A>, [i128; u8::MIN as usize])>,
    f8: BTreeSet<Wrapping<D>>,
    #[difference(skip)]
    f9: [B; C],
    f10: [B; N],
    r#f11: Option<&'b Option<usize>>,
    #[difference(skip)]
    f12: Option<Box<dyn Fn()>>,
    #[difference(skip)]
    f13: Vec<fn(A, &(dyn core::fmt::Debug + Sync + 'static)) -> !>,
    #[difference(skip)]
    f14: Vec<Box<dyn FnMut(A, LM) -> Box<dyn Fn(i32) -> i32>>>,
    #[difference(skip)]
    f15: Vec<fn()>,
}
