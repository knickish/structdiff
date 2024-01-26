#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Debug,
    num::Wrapping,
};
use structdiff::{Difference, StructDiff};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Clone, Difference, Default)]
#[difference(setters)]
pub struct Test {
    pub test1: i32,
    pub test2: String,
    pub test3: Vec<i32>,
    pub test4: f32,
    pub test5: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Difference)]
#[difference(setters)]
pub struct TestSkip<A>
where
    A: PartialEq,
{
    pub test1: A,
    pub test2: String,
    #[difference(skip)]
    pub test3skip: Vec<i32>,
    pub test4: f32,
}

#[allow(unused)]
#[cfg(not(any(feature = "serde", feature = "nanoserde")))]
#[derive(PartialEq, Difference, Clone, Debug)]
// #[derive(PartialEq, Clone, Debug)]
pub enum TestDeriveAllEnum<
    'a,
    'b: 'a,
    A: PartialEq + 'static,
    const C: usize,
    B: PartialEq,
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
    (dyn std::cmp::PartialEq<A> + Send + 'static): Debug + Clone + PartialEq,
{
    F1(()),
    F2([A; N]),
    F3([i32; N]),
    F4(BTreeMap<LM, BTreeSet<<LM as IntoIterator>::Item>>),
    F5(Option<(A, Option<&'a <LM as IntoIterator>::Item>)>),
    F6(HashMap<A, BTreeSet<LM>>),
    F8(BTreeSet<Wrapping<D>>, BTreeSet<Wrapping<B>>),
    F9 {},
    F10 { subfield1: u64, subfield2: Test },
    r#F11(Option<&'b Option<usize>>),
    F12(TestSkip<A>, TestSkip<A>),
    F13((TestSkip<A>, TestSkip<B>)),
}
