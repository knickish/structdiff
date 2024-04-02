use std::fmt::Debug;

pub fn hirschberg<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: impl IntoIterator<Item = &'target T>,
    source: impl IntoIterator<Item = &'src T>,
) -> Option<OrderedArrayLikeDiffRef<'target, T>> {
    let target = target.into_iter().collect::<Vec<_>>();
    let source = source.into_iter().collect::<Vec<_>>();

    match hirschberg_impl(
        &target,
        &source,
        Indices {
            target_start: 0,
            target_end: target.len(),
            source_start: 0,
            source_end: source.len(),
        },
    )
    .into_iter()
    .collect::<Vec<_>>()
    {
        empty if empty.is_empty() => None,
        mut nonempty => {
            nonempty.reverse();
            Some(OrderedArrayLikeDiffRef(nonempty))
        }
    }
}

pub fn levenshtein<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: impl IntoIterator<Item = &'target T>,
    source: impl IntoIterator<Item = &'src T>,
) -> Option<OrderedArrayLikeDiffRef<'target, T>> {
    let target = target.into_iter().collect::<Vec<_>>();
    let source = source.into_iter().collect::<Vec<_>>();

    match levenshtein_impl(
        &target,
        &source,
        Indices {
            target_start: 0,
            target_end: target.len(),
            source_start: 0,
            source_end: source.len(),
        },
    )
    .into_iter()
    .collect::<Vec<_>>()
    {
        empty if empty.is_empty() => None,
        nonempty => Some(OrderedArrayLikeDiffRef(nonempty)),
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub(crate) enum OrderedArrayLikeChangeRef<'a, T> {
    Replace(&'a T, usize),
    Insert(&'a T, usize),
    /// (start, optional end) range for deletion
    Delete(usize, Option<usize>),
    #[allow(unused)]
    Swap(usize, usize),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum OrderedArrayLikeChangeOwned<T> {
    Replace(T, usize),
    Insert(T, usize),
    /// (start, optional end) range for deletion
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

impl<T: Debug> OrderedArrayLikeChangeOwned<T> {
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

#[derive(Debug, Copy, Clone)]
struct Indices {
    target_start: usize,
    target_end: usize,
    source_start: usize,
    source_end: usize,
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

#[cfg(unused)]
fn print_table_2(table: &[Vec<ChangeInternal>; 2]) {
    for row in table {
        println!("{:?}", row)
    }
    println!("")
}

#[inline]
fn create_full_change_table<T: PartialEq>(
    target: &[&T],
    source: &[&T],
) -> Vec<Vec<ChangeInternal>> {
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
    table
}

#[inline]
fn create_last_change_row<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: &[&'target T],
    target_start: usize,
    target_end: usize,
    source: &[&'src T],
    source_start: usize,
    source_end: usize,
) -> Vec<ChangeInternal> {
    let source_len = source_start.abs_diff(source_end);
    let rev = target_start > target_end || source_start > source_end;

    debug_assert_eq!(
        target_start <= target_end,
        source_start <= source_end,
        "\ntarget start: {}\ntarget end: {}\nsource start: {}\nsource end: {}",
        target_start,
        target_end,
        source_start,
        source_end
    );

    let mut table = std::array::from_fn::<_, 2, _>(|_| {
        Vec::from_iter((0..(source_len + 1)).map(|i| ChangeInternal::Delete(i)))
    });

    let (target_range, source_range): (
        Box<dyn Iterator<Item = usize>>,
        Box<dyn Fn() -> Box<dyn Iterator<Item = usize>>>,
    ) = match rev {
        true => (
            Box::new((target_end..target_start).rev()),
            Box::new(|| Box::new((source_end..source_start).rev())),
        ),
        false => (
            Box::new(target_start..target_end),
            Box::new(|| Box::new(source_start..source_end)),
        ),
    };
    for target_index in target_range {
        let target_entry = target[target_index];
        table[1][0] = ChangeInternal::Insert(table[0][0].cost() + 1); // TODO make this configurable
        for (prev, source_index) in (source_range()).enumerate() {
            let source_entry = source[source_index];
            let curr = prev + 1;
            if target_entry == source_entry {
                table[1][curr] = ChangeInternal::NoOp(table[0][prev].cost());
                // char matches, skip comparisons
                continue;
            }

            let insert = table[1][prev].cost();
            let delete = table[0][curr].cost();
            let replace = table[0][prev].cost();

            let min = insert.min(delete).min(replace);

            if min == replace {
                table[1][curr] = ChangeInternal::Replace(min + 1);
            } else if min == delete {
                table[1][curr] = ChangeInternal::Delete(min + 1);
            } else {
                table[1][curr] = ChangeInternal::Insert(min + 1);
            }
        }
        table.swap(0, 1);
    }

    let [ret, ..] = table;
    ret
}

fn hirschberg_impl<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: &[&'target T],
    source: &[&'src T],
    Indices {
        target_start,
        target_end,
        source_start,
        source_end,
    }: Indices,
) -> Box<dyn DoubleEndedIterator<Item = OrderedArrayLikeChangeRef<'target, T>> + 'target> {
    let indices = Indices {
        target_start,
        target_end,
        source_start,
        source_end,
    };
    // base cases
    match (target_start == target_end, source_start == source_end) {
        (true, true) => return Box::new(std::iter::empty()),
        (true, false) => {
            return Box::new(std::iter::once(OrderedArrayLikeChangeRef::Delete(
                source_start,
                Some(source_end - 1),
            )));
        }
        (false, true) => {
            let iter: Box<dyn Iterator<Item = _>> = Box::new(
                target[target_start..target_end]
                    .into_iter()
                    .map(|a| *a)
                    .enumerate()
                    .map(|(i, v)| {
                        let idx = source_end + i;
                        OrderedArrayLikeChangeRef::Insert(v, idx)
                    })
                    .rev(),
            );

            return Box::new(iter.collect::<Vec<_>>().into_iter());
        }
        (false, false)
            if target_start.abs_diff(target_end) == 1 || source_start.abs_diff(source_end) == 1 =>
        {
            let lev = levenshtein_impl(target, source, indices);
            return Box::new(lev.rev());
        }
        _ => (),
    }

    let target_split_index = target_start + ((target_end - target_start) / 2);
    let left = create_last_change_row(
        target,
        target_start,
        target_split_index,
        source,
        source_start,
        source_end,
    );

    let right = create_last_change_row(
        target,
        target_end,
        target_split_index,
        source,
        source_end,
        source_start,
    );

    let source_split_index = left
        .into_iter()
        .zip(right.into_iter().rev())
        .map(|(l, r)| l.cost() + r.cost())
        .enumerate()
        .min_by_key(|(_, v)| *v)
        .map(|(idx, _)| source_start + idx)
        .unwrap();

    let left = hirschberg_impl(
        &target,
        &source,
        Indices {
            target_end: target_split_index,
            source_end: source_split_index,
            ..indices
        },
    );

    let right = hirschberg_impl(
        &target,
        &source,
        Indices {
            target_start: target_split_index,
            source_start: source_split_index,
            ..indices
        },
    );

    Box::new(left.chain(right))
}

fn levenshtein_impl<'src, 'target: 'src, T: Clone + PartialEq + Debug + 'target>(
    target: &[&'target T],
    source: &[&'src T],
    Indices {
        target_start,
        target_end,
        source_start,
        source_end,
    }: Indices,
) -> Box<dyn DoubleEndedIterator<Item = OrderedArrayLikeChangeRef<'target, T>> + 'target> {
    #[inline]
    fn changelist_from_change_table<'src, 'target: 'src, T: PartialEq + Debug>(
        table: Vec<Vec<ChangeInternal>>,
        target: &[&'target T],
        _source: &[&'src T],
        Indices {
            target_start,
            target_end,
            source_start,
            source_end,
        }: Indices,
    ) -> Box<dyn DoubleEndedIterator<Item = OrderedArrayLikeChangeRef<'target, T>> + 'target> {
        let rev = target_start > target_end || source_start > source_end;
        let mut target_pos = target_start.abs_diff(target_end);
        let mut source_pos = source_start.abs_diff(source_end);
        let mut changelist = Vec::with_capacity(
            table
                .last()
                .and_then(|r| r.last())
                .map(|c| c.cost())
                .unwrap_or_default(),
        );

        // collect required changes to make source into target
        while target_pos > 0 && source_pos > 0 {
            match rev {
                true => {
                    match &(table[target_pos][source_pos]) {
                        ChangeInternal::NoOp(_) => {
                            target_pos -= 1;
                            source_pos -= 1;
                        }
                        ChangeInternal::Replace(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Replace(
                                target[target_end - target_pos],
                                source_end - source_pos,
                            ));
                            target_pos -= 1;
                            source_pos -= 1;
                        }
                        ChangeInternal::Insert(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Insert(
                                target[target_end - target_pos],
                                source_end - source_pos,
                            ));
                            target_pos -= 1;
                        }
                        ChangeInternal::Delete(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Delete(
                                source_end - source_pos,
                                None,
                            ));
                            source_pos -= 1;
                        }
                    }
                    if changelist.len() == table.last().unwrap().last().unwrap().cost() {
                        target_pos = 0;
                        source_pos = 0;
                        break;
                    }
                }
                false => {
                    match &(table[target_pos][source_pos]) {
                        ChangeInternal::NoOp(_) => {
                            target_pos -= 1;
                            source_pos -= 1;
                        }
                        ChangeInternal::Replace(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Replace(
                                target[target_start + target_pos - 1],
                                source_start + source_pos - 1,
                            ));
                            target_pos -= 1;
                            source_pos -= 1;
                        }
                        ChangeInternal::Insert(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Insert(
                                target[target_start + target_pos - 1],
                                source_start + source_pos,
                            ));
                            target_pos -= 1;
                        }
                        ChangeInternal::Delete(_) => {
                            changelist.push(OrderedArrayLikeChangeRef::Delete(
                                source_start + source_pos - 1,
                                None,
                            ));
                            source_pos -= 1;
                        }
                    }
                    if changelist.len() == table.last().unwrap().last().unwrap().cost() {
                        target_pos = 0;
                        source_pos = 0;
                        break;
                    }
                }
            }
        }

        match rev {
            true => {
                // target is longer than source, add the missing elements
                while target_pos > 0 {
                    changelist.push(OrderedArrayLikeChangeRef::Insert(
                        target[target_end - target_pos],
                        source_end - source_pos,
                    ));
                    target_pos -= 1;
                }

                // source is longer than target, remove the extra elements
                if source_pos > 0 {
                    changelist.push(OrderedArrayLikeChangeRef::Delete(
                        source_start,
                        Some(source_end - source_pos),
                    ));
                }
            }
            false => {
                // target is longer than source, add the missing elements
                while target_pos > 0 {
                    changelist.push(OrderedArrayLikeChangeRef::Insert(
                        target[target_start + target_pos - 1],
                        source_start + source_pos,
                    ));
                    target_pos -= 1;
                }

                // source is longer than target, remove the extra elements
                if source_pos > 0 {
                    changelist.push(OrderedArrayLikeChangeRef::Delete(
                        source_start,
                        Some(source_start + source_pos - 1),
                    ));
                }
            }
        }

        Box::new(changelist.into_iter())
    }

    let table = match (target_start > target_end, source_start > source_end) {
        (false, false) => create_full_change_table(
            &target[target_start..target_end],
            &source[source_start..source_end],
        ),
        (true, true) => create_full_change_table(
            &target[target_end..target_start],
            &source[source_end..source_start],
        ),
        (false, true) => create_full_change_table(
            &target[target_start..target_end],
            &source[source_end..source_start],
        ),
        (true, false) => create_full_change_table(
            &target[target_end..target_start],
            &source[source_start..source_end],
        ),
    };

    changelist_from_change_table(
        table,
        &target,
        &source,
        Indices {
            target_start,
            target_end,
            source_start,
            source_end,
        },
    )
}

pub fn apply<T, L>(
    changes: impl Into<OrderedArrayLikeDiffOwned<T>>,
    existing: L,
) -> Box<dyn Iterator<Item = T>>
where
    T: Clone + Debug + 'static,
    L: IntoIterator<Item = T> + FromIterator<T>,
{
    let mut ret = existing.into_iter().collect::<Vec<_>>();

    for change in changes.into().0 {
        change.apply(&mut ret);
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

    use crate as structdiff;
    use crate::collections::ordered_array_like::{
        apply, OrderedArrayLikeDiffOwned, OrderedArrayLikeDiffRef,
    };
    use nanorand::{Rng, WyRand};

    use structdiff::{Difference, StructDiff};

    use super::hirschberg;
    use super::levenshtein;

    #[test]
    fn test_string() {
        let s1 = String::from("tested");
        let s2 = String::from("testing");

        let s1_vec = s1.chars().collect::<Vec<_>>();
        let s2_vec = s2.chars().collect::<Vec<_>>();
        for diff_type in [levenshtein, hirschberg] {
            let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                assert_eq!(&s1_vec, &s2_vec);
                return;
            };

            let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                .into_iter()
                .collect::<String>();
            assert_eq!(s1, changed)
        }
    }

    #[test]
    fn test_dna() {
        let s1 = String::from("ACCCGGTCGTCAATTA");
        let s2 = String::from("ACCACCGGTTGGTCCAATAA");

        let s1_vec = s1.chars().collect::<Vec<_>>();
        let s2_vec = s2.chars().collect::<Vec<_>>();

        for diff_type in [
            // levenshtein,
            hirschberg,
        ] {
            let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                assert_eq!(&s1_vec, &s2_vec);
                return;
            };

            let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                .into_iter()
                .collect::<String>();
            assert_eq!(s1, changed)
        }
    }

    #[test]
    fn test_one_empty_string() {
        let s1: Vec<char> = "abc".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        for diff_type in [
            // levenshtein,
            hirschberg,
        ] {
            let Some(changes) = diff_type(&s1, &s2) else {
                assert_eq!(s1, s2);
                return;
            };

            assert_eq!(
                changes.0.len(),
                s1.len(),
                "Should require deletions for all characters in the non-empty string."
            );
        }
    }

    #[test]
    fn test_empty_strings() {
        let s1: Vec<char> = "".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        for diff_type in [levenshtein, hirschberg] {
            let Some(changes) = diff_type(&s1, &s2) else {
                assert_eq!(s1, s2);
                return;
            };

            assert!(
                changes.0.is_empty(),
                "No changes should be needed for two empty strings."
            );
        }
    }

    #[test]
    fn test_identical_strings() {
        let s1: Vec<char> = "rust".chars().collect();
        for diff_type in [levenshtein, hirschberg] {
            let changes = diff_type(&s1, &s1);
            assert!(
                changes.is_none(),
                "No changes should be needed for identical strings."
            );
        }
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

            for diff_type in [levenshtein, hirschberg] {
                let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                    assert_eq!(&s1_vec, &s2_vec);
                    continue;
                };

                let changed = apply(changes, s2_vec.clone())
                    .into_iter()
                    .collect::<Vec<char>>();
                assert_eq!(&s1_vec, &changed);
            }
        }
    }

    #[test]
    fn test_random_f64_lists() {
        let mut rng = WyRand::new();

        for _ in 0..1 {
            // Generate and test 100 pairs of lists
            let list1_len = rng.generate_range(8..10);
            let list2_len = rng.generate_range(8..10);

            let list1: Vec<f64> = (0..list1_len).map(|_| rng.generate::<f64>()).collect();
            let list2: Vec<f64> = (0..list2_len).map(|_| rng.generate::<f64>()).collect();

            for diff_type in [levenshtein, hirschberg] {
                let Some(changes) = diff_type(&list1, &list2) else {
                    assert_eq!(&list1, &list2);
                    return;
                };

                let changed = apply(changes, list2.clone()).collect::<Vec<_>>();
                assert_eq!(list1, changed)
            }
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

    mod problem_cases {
        use super::*;

        #[test]
        fn test_string() {
            let s1 = String::from("AGTACGCA");
            let s2 = String::from("TATGC");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();

            let Some(changes) = hirschberg(&s1_vec, &s2_vec) else {
                assert_eq!(&s1_vec, &s2_vec);
                return;
            };

            let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                .into_iter()
                .collect::<String>();
            assert_eq!(s1, changed)
        }

        #[test]
        fn test_string_2() {
            let s1 = String::from("testinged");
            let s2 = String::from("testeding");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();

            let Some(changes) = hirschberg(&s1_vec, &s2_vec) else {
                assert_eq!(&s1_vec, &s2_vec);
                return;
            };

            let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                .into_iter()
                .collect::<String>();
            assert_eq!(s1, changed)
        }

        #[test]
        fn test_string_3() {
            let s1 = String::from("tested");
            let s2 = String::from("testing");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();
            for diff_type in [levenshtein, hirschberg] {
                let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                    assert_eq!(&s1_vec, &s2_vec);
                    return;
                };

                let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                    .into_iter()
                    .collect::<String>();
                assert_eq!(s1, changed)
            }
        }

        #[test]
        fn test_string_4() {
            let s1 = String::from("oanehxv");
            let s2 = String::from("yfh");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();
            for diff_type in [hirschberg, levenshtein] {
                let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                    assert_eq!(&s1_vec, &s2_vec);
                    return;
                };

                let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                    .into_iter()
                    .collect::<String>();
                assert_eq!(s1, changed)
            }
        }

        #[test]
        fn test_string_5() {
            let s1 = String::from("lllzrsul");
            let s2 = String::from("eoz");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();
            for diff_type in [levenshtein, hirschberg] {
                let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                    assert_eq!(&s1_vec, &s2_vec);
                    return;
                };

                let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                    .into_iter()
                    .collect::<String>();
                assert_eq!(s1, changed)
            }
        }

        #[test]
        fn test_string_6() {
            let s1 = String::from("mc");
            let s2 = String::from("rbuzmjw");

            let s1_vec = s1.chars().collect::<Vec<_>>();
            let s2_vec = s2.chars().collect::<Vec<_>>();
            for diff_type in [hirschberg, levenshtein] {
                let Some(changes) = diff_type(&s1_vec, &s2_vec) else {
                    assert_eq!(&s1_vec, &s2_vec);
                    return;
                };

                let changed = apply(changes, s2.chars().collect::<Vec<_>>())
                    .into_iter()
                    .collect::<String>();
                assert_eq!(s1, changed)
            }
        }
    }
}
