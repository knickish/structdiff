use std::{
    fmt::Debug,
    ops::{Index, IndexMut, RangeBounds},
};

#[derive(Clone)]
pub(crate) struct ArrayMap<T, const N: usize>([Option<(u8, T)>; N], usize);

impl<T: Debug, const N: usize> Debug for ArrayMap<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ArrayMap")
            .field(&self.0.iter().filter_map(|o| o.as_ref()).collect::<Vec<_>>())
            .field(&self.1)
            .finish()
    }
}

impl<T, const N: usize> ArrayMap<T, N> {
    #[inline]
    fn find_empty_slot(&self) -> Option<usize> {
        self.0.iter().position(Option::is_none)
    }

    #[inline]
    fn find_empty_slots(&self) -> [Option<usize>; N] {
        let mut ret = [const { None }; N];
        for (slot, idx) in self
            .0
            .iter()
            .enumerate()
            .filter(|i| i.1.is_none())
            .map(|e| e.0)
            .zip(0..)
        {
            ret[idx] = Some(slot);
        }
        ret
    }

    fn get_lookups(&self) -> [Option<usize>; N] {
        let list_of_logical_indices: [Option<u8>; N] = std::array::from_fn(|storage_idx| {
            self.0[storage_idx]
                .as_ref()
                .map(|(logical_idx, _)| Some(*logical_idx))
                .unwrap_or_default()
        });
        let mut lookup_tmp: [(usize, Option<u8>); N] =
            std::array::from_fn(|storage_idx| (storage_idx, list_of_logical_indices[storage_idx]));
        lookup_tmp.sort_unstable_by_key(|(_storage_idx, logical_idx)| *logical_idx);
        let start_of_somes = lookup_tmp
            .iter()
            .position(|(_storage, logical)| logical.is_some())
            .unwrap_or_default();
        let lookups = std::array::from_fn(|i| {
            lookup_tmp
                .get(start_of_somes + i)
                .map(|(storage, _logical)| *storage)
        });
        lookups
    }
}

impl<T, const N: usize> ArrayMap<T, N> {
    pub const fn new() -> Self {
        if N > u8::MAX as usize {
            panic!("N > u8::MAX is unsupported");
        }
        Self([const { None }; N], 0)
    }

    pub const fn len(&self) -> usize {
        self.1
    }

    pub const fn is_empty(&self) -> bool {
        self.1 == 0
    }

    pub fn insert(&mut self, position: usize, value: T) {
        assert!(
            position < N,
            "Position {position} is greater than max position of {}",
            N - 1
        );
        assert!(
            self.0.iter().any(Option::is_none),
            "No space to insert in ArrayMap with len = {}",
            self.1
        );

        // bump each following index number
        self.0
            .iter_mut()
            .filter_map(|o| o.as_mut().map(|some| &mut some.0))
            .filter(|i| **i as usize >= position)
            .for_each(|i| *i += 1);

        // find a free slot and put it in
        let slot_idx = self.find_empty_slot().expect("failed to find free slot");
        self.0[slot_idx] = Some((position as u8, value));

        self.1 += 1;
    }

    pub fn remove(&mut self, position: usize) -> T {
        let u8_idx = position as u8;
        let Some(val) = self
            .0
            .iter_mut()
            .find(|o| o.as_ref().map(|(i, _)| *i == u8_idx).unwrap_or_default())
            .and_then(Option::take)
            .map(|o| o.1)
        else {
            panic!("No element found at position {}", position);
        };

        // lower each following index number
        self.0
            .iter_mut()
            .filter_map(|o| o.as_mut().map(|some| &mut some.0))
            .filter(|i| **i > u8_idx)
            .for_each(|i| *i -= 1);

        self.1 -= 1;
        val
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }

        let mut indices = [a, b].map(|find| {
            self.0
                .iter()
                .position(|o| o.as_ref().map(|(i, _)| *i == find as u8).unwrap_or(false))
                .unwrap_or_else(|| panic!("unable to find item at idx: {}", find))
        });
        indices.sort();
        let [lower, upper] = indices;
        let (l, r) = self.0.split_at_mut(upper);
        std::mem::swap(
            l[lower].as_mut().map(|o| &mut o.0).unwrap(),
            r[0].as_mut().map(|o| &mut o.0).unwrap(),
        );
    }

    pub fn drain<R>(&mut self, range: R) -> Drain<T, N>
    where
        R: RangeBounds<usize>,
    {
        // stores the highest removed value, so that later ones can be decremented
        let mut max_removed: Option<u8> = None;
        // store the length before removing things
        let before = self.1;

        let removals = self
            .0
            .iter_mut()
            .filter(|o| matches!(o, Some((i, _)) if range.contains(&(*i as usize))));

        // move the items to the new list and decrement the item count
        let mut drained = [const { None }; N];
        for (idx, removal) in removals.enumerate() {
            let removal_logical_idx = removal.as_ref().map(|o| o.0).unwrap();
            max_removed = max_removed
                .map(|current| Some(current.max(removal_logical_idx)))
                .unwrap_or_else(|| Some(removal_logical_idx));
            drained[idx] = removal.take();
            self.1 -= 1;
        }

        drained.sort_by_key(|e| e.as_ref().map(|o| o.0));
        let ret = Self::from_iter(drained.into_iter().filter_map(|e| e.map(|o| o.1)));

        // decrement all indices after the last removed index by the
        // number of items drained
        if let Some(max) = max_removed {
            let removed_count = before.abs_diff(self.1) as u8;

            self.0
                .iter_mut()
                .filter_map(|o| o.as_mut().map(|some| &mut some.0))
                .filter(|i| **i > max)
                .for_each(|i| *i -= removed_count);
        }

        Drain(ret.into_iter())
    }

    pub fn extend<I: Iterator<Item = T>>(&mut self, values: I) {
        let free_slots = self.find_empty_slots();

        for (v, loc) in values.zip(free_slots.into_iter().flatten()) {
            self.0[loc] = Some((self.1 as u8, v));
            self.1 += 1;
        }
    }
}

pub struct Drain<T, const N: usize>(<ArrayMap<T, N> as IntoIterator>::IntoIter);

impl<T, const N: usize> Iterator for Drain<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<T, const N: usize> DoubleEndedIterator for Drain<T, N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<T, const N: usize> Index<usize> for ArrayMap<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.0
            .iter()
            .filter_map(|o| o.as_ref())
            .find(|(idx, _)| *idx as usize == index)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("No element found at index {index}"))
    }
}

impl<T, const N: usize> IndexMut<usize> for ArrayMap<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0
            .iter_mut()
            .filter_map(|o| o.as_mut())
            .find(|(idx, _)| *idx as usize == index)
            .map(|(_, v)| v)
            .unwrap()
    }
}

// self.iter()
const _: () = {
    pub struct Iter<'rope, T, const N: usize> {
        self_ref: &'rope ArrayMap<T, N>,
        // list of indices into the backing array, stored in logical array order
        lookups: [Option<usize>; N],
        pos: usize,
    }

    impl<'rope, T, const N: usize> Iterator for Iter<'rope, T, N> {
        type Item = &'rope T;

        fn next(&mut self) -> Option<Self::Item> {
            let v_ref = self
                .lookups
                .get(self.pos)
                .copied()
                .flatten()
                .and_then(|idx| self.self_ref.0.get(idx))
                .and_then(|v| v.as_ref().map(|(_, v)| v))?;

            self.pos += 1;
            Some(v_ref)
        }
    }

    impl<'rope, T, const N: usize> IntoIterator for &'rope ArrayMap<T, N> {
        type Item = &'rope T;

        type IntoIter = Iter<'rope, T, N>;

        fn into_iter(self) -> Self::IntoIter {
            let lookups = self.get_lookups();
            Iter {
                self_ref: self,
                lookups,

                pos: 0,
            }
        }
    }
};

// self.into_iter()
const _: () = {
    pub struct Iter<T, const N: usize> {
        self_owned: ArrayMap<T, N>,
        // list of indices into the backing array, stored in logical array order
        lookups: [Option<usize>; N],
        pos: usize,
        rev_pos: usize,
    }

    impl<T, const N: usize> Iterator for Iter<T, N> {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            let v_ref = self
                .lookups
                .get(self.pos)
                .copied()
                .flatten()
                .and_then(|idx| self.self_owned.0.get_mut(idx))
                .and_then(|v| v.take().map(|(_, v)| v))?;

            self.pos += 1;
            Some(v_ref)
        }
    }

    impl<T, const N: usize> DoubleEndedIterator for Iter<T, N> {
        fn next_back(&mut self) -> Option<Self::Item> {
            let v_ref = self
                .lookups
                .get(self.rev_pos)
                .copied()
                .flatten()
                .and_then(|idx| self.self_owned.0.get_mut(idx))
                .and_then(|v| v.take().map(|(_, v)| v))?;

            self.rev_pos += 1;
            Some(v_ref)
        }
    }

    impl<T, const N: usize> IntoIterator for ArrayMap<T, N> {
        type Item = T;

        type IntoIter = Iter<T, N>;

        fn into_iter(self) -> Self::IntoIter {
            let lookups = self.get_lookups();
            let rev_pos = self.len().saturating_sub(1);
            Iter {
                self_owned: self,
                lookups,

                pos: 0,
                rev_pos,
            }
        }
    }
};

impl<T, const N: usize> FromIterator<T> for ArrayMap<T, N> {
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let mut ret = Self::new();
        let mut count = 0;
        for (i, v) in iter.into_iter().enumerate() {
            ret.0[i] = Some((i as u8, v));
            count += 1;
        }

        ret.1 = count;
        ret
    }
}
