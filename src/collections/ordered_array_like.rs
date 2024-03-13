#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "nanoserde", derive(SerBin))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Change<'a, T> {
    Replace(&'a T, usize),
    Insert(&'a T, usize),
    Delete(usize, Option<usize>),
    Swap(usize, usize),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "nanoserde", derive(SerBin, DeBin))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChangeOwned<T> {
    Replace(T, usize),
    Insert(T, usize),
    Delete(usize, Option<usize>),
    Swap(usize, usize),
}

impl<'a, T: Clone> From<Change<'a, T>> for ChangeOwned<T> {
    fn from(value: Change<'a, T>) -> Self {
        match value {
            Change::Replace(val, idx) => Self::Replace(val.to_owned(), idx),
            Change::Insert(val, idx) => Self::Insert(val.to_owned(), idx),
            Change::Delete(idx, range) => Self::Delete(idx, range),
            Change::Swap(l, r) => Self::Swap(l, r),
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

#[cfg(test)]
impl<'a, T: Clone> Change<'a, T> {
    fn apply(self, container: &mut Vec<T>) {
        match self {
            Change::Replace(val, loc) => container[loc] = val.clone(),
            Change::Insert(val, loc) => container.insert(loc, val.clone()),
            Change::Delete(loc, None) => {
                container.remove(loc);
            }
            Change::Delete(l, Some(r)) => {
                container.drain(l..=r);
            }
            Change::Swap(l, r) => container.swap(l, r),
        }
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
) -> Vec<Change<'src, T>> {
    let target = target.into_iter().collect::<Vec<_>>();
    let source = source.into_iter().collect::<Vec<_>>();
    let mut table = vec![vec![ChangeInternal::NoOp(0); source.len() + 1]; target.len() + 1];

    for (i, entry) in table.iter_mut().enumerate().skip(1) {
        entry[0] = ChangeInternal::Insert(i);
    }

    for j in 0..=source.len() {
        table[0][j] = ChangeInternal::Delete(j)
    }

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

    while target_pos > 0 && source_pos > 0 {
        match &(table[target_pos][source_pos]) {
            ChangeInternal::NoOp(_) => {
                target_pos -= 1;
                source_pos -= 1;
            }
            ChangeInternal::Replace(_) => {
                changelist.push(Change::Replace(target[target_pos - 1], source_pos - 1));
                target_pos -= 1;
                source_pos -= 1;
            }
            ChangeInternal::Insert(_) => {
                changelist.push(Change::Insert(target[target_pos - 1], source_pos));
                target_pos -= 1;
            }
            ChangeInternal::Delete(_) => {
                changelist.push(Change::Delete(source_pos - 1, None));
                source_pos -= 1;
            }
        }
        if changelist.len() == table[target.len()][source.len()].cost() {
            target_pos = 0;
            source_pos = 0;
            break;
        }
    }

    while target_pos > 0 {
        changelist.push(Change::Insert(target[target_pos - 1], source_pos));
        target_pos -= 1;
    }

    if source_pos > 0 {
        changelist.push(Change::Delete(0, Some(source_pos - 1)));
    }
    changelist
}

#[cfg(test)]
pub fn apply_changes<'src, T: Clone, L: IntoIterator<Item = T> + FromIterator<T>>(
    changes: Vec<Change<'src, T>>,
    existing: L,
) -> L {
    let mut ret = existing.into_iter().collect::<Vec<_>>();

    for change in changes {
        change.apply(&mut ret)
    }

    ret.into_iter().collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use nanorand::{Rng, WyRand};

    #[test]
    fn test_string() {
        let s1 = String::from("tested");
        let s2 = String::from("testing");

        let s1_vec = s1.chars().collect::<Vec<_>>();
        let s2_vec = s2.chars().collect::<Vec<_>>();

        let changes = levenshtein(&s1_vec, &s2_vec);
        let changed = apply_changes(changes, s2.chars().collect::<Vec<_>>())
            .into_iter()
            .collect::<String>();
        assert_eq!(s1, changed)
    }

    #[test]
    fn test_one_empty_string() {
        let s1: Vec<char> = "abc".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        let changes = levenshtein(&s1, &s2);
        assert_eq!(
            changes.len(),
            s1.len(),
            "Should require deletions for all characters in the non-empty string."
        );
    }

    #[test]
    fn test_empty_strings() {
        let s1: Vec<char> = "".chars().collect();
        let s2: Vec<char> = "".chars().collect();

        let changes = levenshtein(&s1, &s2);
        assert!(
            changes.is_empty(),
            "No changes should be needed for two empty strings."
        );
    }

    #[test]
    fn test_identical_strings() {
        let s1: Vec<char> = "rust".chars().collect();
        let changes = levenshtein(&s1, &s1);
        assert!(
            changes.is_empty(),
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

            let changes = levenshtein(&s1_vec, &s2_vec);
            let changed = apply_changes(changes, s2_vec.clone())
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

            let changes = levenshtein(&list1, &list2);
            let changed = apply_changes(changes, list2.clone());
            assert_eq!(list1, changed)
        }
    }
}
