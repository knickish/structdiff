use std::{
    cmp::Ordering::{Equal, Greater, Less},
    collections::VecDeque,
    ops::{Index, IndexMut, RangeBounds},
};

mod slots;

const MAX_SLOT_SIZE: usize = 16;
const BASE_SLOT_SIZE: usize = 8;
const UNDERSIZED_SLOT: usize = 1;

type Container<T> = slots::ArrayMap<T, MAX_SLOT_SIZE>;

#[cfg_attr(test, derive(Clone))]
pub struct Rope<T>(Vec<Container<T>>);

pub struct Iter<'rope, T> {
    self_ref: &'rope Rope<T>,
    key: usize,
    in_key: usize,
    exhausted: bool,
}
pub struct IntoIter<T> {
    self_own: VecDeque<slots::ArrayMap<T, MAX_SLOT_SIZE>>,
    internal: Option<<slots::ArrayMap<T, MAX_SLOT_SIZE> as IntoIterator>::IntoIter>,
}

impl<T> Index<usize> for Rope<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let mut seen = 0;
        for entry in self.0.iter() {
            seen += entry.len();
            if seen > index {
                seen -= entry.len();
                return &entry[index - seen];
            }
        }
        panic!("Index is {index} but len is {seen}")
    }
}

impl<T> IndexMut<usize> for Rope<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        let mut seen = 0;
        for entry in self.0.iter_mut() {
            seen += entry.len();
            if seen > index {
                seen -= entry.len();
                return &mut entry[index - seen];
            }
        }
        panic!("Index is {index} but len is {seen}")
    }
}

impl<'rope, T: 'rope> Iterator for Iter<'rope, T> {
    type Item = &'rope T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        let ret = &self.self_ref.0[self.key][self.in_key];

        self.in_key += 1;
        if self.in_key >= self.self_ref.0[self.key].len() {
            self.in_key = 0;
            self.key += 1;
        }

        if self.key >= self.self_ref.0.len() {
            self.exhausted = true;
        }

        Some(ret)
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let ret @ Some(_) = self.internal.as_mut().and_then(|internal| internal.next()) {
            return ret;
        }

        while let Some(mut vec_iter) = self.self_own.pop_front().map(IntoIterator::into_iter) {
            let ret @ Some(_) = vec_iter.next() else {
                continue;
            };

            self.internal = Some(vec_iter);
            return ret;
        }

        None
    }
}

impl<T> IntoIterator for Rope<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            self_own: self.0.into(),
            internal: None,
        }
    }
}

impl<'rope, T: 'rope> IntoIterator for &'rope Rope<T> {
    type Item = &'rope T;

    type IntoIter = Iter<'rope, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            self_ref: self,
            key: 0,
            in_key: 0,
            exhausted: self.0.is_empty() || self.0[0].is_empty(),
        }
    }
}

impl<T> FromIterator<T> for Rope<T> {
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let mut iter = iter.into_iter().peekable();
        let mut map = Vec::new();
        while iter.peek().is_some() {
            let arrmap = slots::ArrayMap::from_iter(iter.by_ref().take(8));
            map.push(arrmap);
            if map.last().unwrap().len() != 8 {
                break;
            }
        }

        Self(map)
    }
}

impl<T> Default for Rope<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Rope<T> {
    pub fn new() -> Self {
        Self(Vec::from([slots::ArrayMap::new()]))
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    #[inline]
    fn _key_for_index(&self, index: usize) -> usize {
        let mut seen = 0;
        for (idx, entry) in self.0.iter().enumerate() {
            seen += entry.len();
            if seen > index {
                return idx;
            }
        }
        self.0.len()
    }

    #[inline]
    fn key_with_count_for_index(&self, index: usize) -> (usize, usize) {
        let mut seen = 0;
        for (idx, entry) in self.0.iter().enumerate() {
            seen += entry.len();
            if seen > index {
                seen -= entry.len();
                return (idx, seen);
            }
        }
        (self.0.len(), seen)
    }

    #[inline]
    fn key_with_count_for_index_from_prev(
        &self,
        index: usize,
        prev: usize,
        mut seen: usize,
    ) -> (usize, usize) {
        if seen > index {
            // it's in the same chunk, return early
            return (prev, seen);
        }
        for (idx, entry) in self.0.iter().enumerate().skip(prev) {
            seen += entry.len();
            if seen > index {
                seen -= entry.len();
                return (idx, seen);
            }
        }
        (self.0.len(), seen)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0
            .iter()
            .map(slots::ArrayMap::len)
            .fold(0, std::ops::Add::add)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn rebalance_from_key(&mut self, start_key: usize) {
        let mut carry = VecDeque::with_capacity(16);
        let mut hold = Container::new();
        for key in start_key..(self.0.len()) {
            let entry = self.0.get_mut(key).unwrap();
            if entry.is_empty() {
                continue;
            }

            const LOW: usize = BASE_SLOT_SIZE - (BASE_SLOT_SIZE / 2);
            const HIGH: usize = BASE_SLOT_SIZE + (BASE_SLOT_SIZE / 2);
            if (LOW..=HIGH).contains(&entry.len()) && carry.is_empty() {
                break;
            }

            // put the empty holder in the list for now
            std::mem::swap(entry, &mut hold);

            // adjust size of hold, either taking elements from later chunks or carrying them
            match (hold.len().cmp(&BASE_SLOT_SIZE), carry.is_empty()) {
                (Less, carry_empty) => {
                    if !carry_empty {
                        carry.extend(hold.drain(..));
                        hold.extend(carry.drain(..BASE_SLOT_SIZE.min(carry.len())));
                    }

                    let mut iter = self.0.iter_mut().skip(key);
                    while let (Some(take_from), false) = (iter.next(), hold.len() == BASE_SLOT_SIZE)
                    {
                        hold.extend(take_from.drain(
                            ..(BASE_SLOT_SIZE.saturating_sub(hold.len())).min(take_from.len()),
                        ));
                    }
                }
                (Equal, true) => (),
                (Equal | Greater, false) => {
                    carry.extend(hold.drain(..));
                    hold.extend(carry.drain(..BASE_SLOT_SIZE.min(carry.len())));
                }
                (Greater, true) => {
                    carry.extend(hold.drain(BASE_SLOT_SIZE..));
                }
            }

            // take the empty holder back and leave the values in the map entry
            std::mem::swap(self.0.get_mut(key).unwrap(), &mut hold);
        }

        assert!(hold.is_empty());

        self.0.retain(|v| !v.is_empty());

        // fix up the last entry with any carried values
        match (carry.len(), self.0.last_mut()) {
            (0, ..) => {
                return;
            }
            (_, Some(l_entry)) => {
                l_entry.extend(
                    carry.drain(..(BASE_SLOT_SIZE.saturating_sub(l_entry.len())).min(carry.len())),
                );
            }
            _ => (),
        }

        // add any remaining carry values into new slots at the end
        while carry.len() > BASE_SLOT_SIZE {
            self.0.push(Container::from_iter(
                carry.drain(..BASE_SLOT_SIZE.min(carry.len())),
            ));
        }
        if !carry.is_empty() {
            self.0.push(Container::from_iter(carry));
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        let (key, count) = self.key_with_count_for_index(index);
        if key == self.0.len() {
            self.0.push(Container::new());
        }
        let vec = self.0.get_mut(key).unwrap();
        vec.insert(index - count, element);
        if vec.len() == MAX_SLOT_SIZE {
            self.rebalance_from_key(key);
        }
    }

    pub fn remove(&mut self, index: usize) {
        let (key, count) = self.key_with_count_for_index(index);
        let Some(vec) = self.0.get_mut(key) else {
            panic!(
                "Failed to remove item with index {index} from rope with {} elements",
                self.len()
            );
        };
        vec.remove(index - count);
        if (0..=UNDERSIZED_SLOT).contains(&vec.len()) {
            self.rebalance_from_key(key.saturating_sub(1));
        }
    }

    pub fn drain<R>(&mut self, range: R)
    where
        R: RangeBounds<usize>,
    {
        use std::ops::Bound;

        let (l_idx, r_idx) = match (range.start_bound(), range.end_bound()) {
            (Bound::Included(l_i), Bound::Included(r_i)) => (*l_i, *r_i),
            (Bound::Included(l_i), Bound::Excluded(r_e)) => (*l_i, r_e - 1),
            (Bound::Included(l_i), Bound::Unbounded) => (*l_i, self.len() - 1),
            (Bound::Excluded(l_e), Bound::Included(r_i)) => (l_e + 1, *r_i),
            (Bound::Excluded(l_e), Bound::Excluded(r_e)) => (l_e + 1, r_e - 1),
            (Bound::Excluded(l_e), Bound::Unbounded) => (l_e + 1, self.len() - 1),
            (Bound::Unbounded, Bound::Included(r_i)) => (0, *r_i),
            (Bound::Unbounded, Bound::Excluded(r_e)) => (0, r_e - 1),
            (Bound::Unbounded, Bound::Unbounded) => (0, self.len() - 1),
        };

        let (l_key, l_key_count) = self.key_with_count_for_index(l_idx);
        let (r_key, r_key_count) =
            self.key_with_count_for_index_from_prev(r_idx, l_key, l_key_count);

        match l_key == r_key {
            true => {
                let v = self.0.get_mut(l_key).expect("we just looked this key up");
                v.drain((l_idx - l_key_count)..=(r_idx - l_key_count));
                if v.len() <= UNDERSIZED_SLOT {
                    self.rebalance_from_key(l_key.saturating_sub(1));
                }
            }
            false => {
                let l_mut = self.0.get_mut(l_key).unwrap();
                l_mut.drain((l_idx - l_key_count)..);
                let l_len = l_mut.len();
                let r_mut = self.0.get_mut(r_key).unwrap();
                r_mut.drain(..=(r_idx - r_key_count));
                let r_len = r_mut.len();
                let _ = self.0.drain((l_key + 1)..r_key);

                if l_len <= UNDERSIZED_SLOT || r_len <= UNDERSIZED_SLOT {
                    self.rebalance_from_key(l_key);
                }
            }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let [a, b] = [a.min(b), a.max(b)];
        let (l_key, l_key_count) = self.key_with_count_for_index(a);
        let (r_key, r_key_count) = self.key_with_count_for_index_from_prev(b, l_key, l_key_count);
        match l_key == r_key {
            true => self
                .0
                .get_mut(l_key)
                .unwrap()
                .swap(a - l_key_count, b - l_key_count),
            false => {
                let (l, r) = self.0.split_at_mut(r_key);
                std::mem::swap(&mut l[l_key][a - l_key_count], &mut r[0][b - r_key_count]);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use nanorand::{Rng, WyRand};

    use super::{Rope, BASE_SLOT_SIZE, MAX_SLOT_SIZE};

    #[derive(Debug, Clone)]
    pub enum Mutation<T> {
        Insert(T, usize),
        Remove(usize),
        Swap(usize, usize),
        Drain(usize, usize),
    }

    pub(crate) trait Random {
        fn generate_random(rng: &mut WyRand) -> Self;
        fn generate_random_large(rng: &mut WyRand) -> Self;
        fn random_mutate(self, mutation: Mutation<String>) -> Self;
    }

    pub fn rand_string(rng: &mut WyRand) -> String {
        let base = vec![(); 8];
        base.into_iter()
            .map(|_| rng.generate_range::<u32, _>(65..=90))
            .filter_map(char::from_u32)
            .collect::<String>()
    }

    impl Mutation<String> {
        pub fn random_mutation(rng: &mut WyRand, len: usize) -> Option<Mutation<String>> {
            match rng.generate_range(0..4) {
                0 => Some(Self::Insert(rand_string(rng), rng.generate_range(0..=len))),
                1 => match len == 0 {
                    false => Some(Self::Remove(rng.generate_range(0..len))),
                    true => None,
                },
                2 => {
                    if len == 0 {
                        return None;
                    }
                    let l = rng.generate_range(0..len);
                    let r = rng.generate_range(0..len);
                    if l == r {
                        None
                    } else {
                        Some(Self::Swap(l, r))
                    }
                }
                3 => {
                    let l = rng.generate_range(0..len);
                    let r = rng.generate_range(l..len);
                    Some(Self::Drain(l, r))
                }
                _ => None,
            }
        }
    }

    impl Random for Vec<String> {
        fn generate_random(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| rand_string(rng))
                .collect()
        }

        fn generate_random_large(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .collect()
        }

        fn random_mutate(mut self, mutation: Mutation<String>) -> Self {
            match mutation {
                Mutation::Insert(s, i) => self.insert(i, s),
                Mutation::Remove(i) => {
                    self.remove(i);
                }
                Mutation::Swap(l, r) => self.swap(l, r),
                Mutation::Drain(l, r) => {
                    self.drain(l..=r);
                }
            }
            self
        }
    }

    impl<T: std::fmt::Display> std::fmt::Display for Rope<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut seen = 0;
            for vals in self.0.iter() {
                write!(f, "{seen}: [")?;
                for val in vals {
                    write!(f, "{},", val)?;
                }
                writeln!(f, "];")?;
                seen += vals.len();
            }
            Ok(())
        }
    }

    impl Random for Rope<String> {
        fn generate_random(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| rand_string(rng))
                .collect()
        }

        fn generate_random_large(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .collect()
        }

        fn random_mutate(mut self, mutation: Mutation<String>) -> Self {
            match mutation {
                Mutation::Insert(s, i) => self.insert(i, s),
                Mutation::Remove(i) => self.remove(i),
                Mutation::Swap(l, r) => self.swap(l, r),
                Mutation::Drain(l, r) => self.drain(l..=r),
            }
            self
        }
    }

    fn test(generator: impl Fn(&mut WyRand) -> Vec<String>, count: usize) {
        let mut rng = WyRand::new();
        let mut start_vec = generator(&mut rng);
        let mut start_rope = start_vec.clone().into_iter().collect::<Rope<_>>();
        assert_eq!(
            start_rope.clone().into_iter().collect::<Vec<_>>(),
            start_vec
        );
        for _ in 0..count {
            let prev_rope = start_rope.clone();
            let Some(mutation) = Mutation::random_mutation(&mut rng, start_vec.len()) else {
                continue;
            };

            let sr_clone = start_rope.clone();
            let mut_clone = mutation.clone();
            let result = std::panic::catch_unwind(|| {
                sr_clone.random_mutate(mut_clone);
            });

            let Ok(_) = result else {
                println!("{:?}", mutation);
                println!("prev_rope: {}", prev_rope);
                panic!("Caught panic");
            };

            start_rope = start_rope.random_mutate(mutation.clone());
            start_vec = start_vec.random_mutate(mutation.clone());

            if start_rope.clone().into_iter().collect::<Vec<_>>() != start_vec {
                println!("{:?}", mutation);
                println!("prev_rope: {}", prev_rope);
                println!("curr_rope: {}", start_rope);
            }
            pretty_assertions::assert_eq!(
                (&start_rope).into_iter().cloned().collect::<Vec<_>>(),
                start_rope.clone().into_iter().collect::<Vec<_>>()
            );
            pretty_assertions::assert_eq!(
                (&start_rope).into_iter().cloned().collect::<Vec<_>>(),
                start_vec
            );
        }
    }

    #[test]
    fn paired_small() {
        test(Vec::generate_random, 1_000_000)
    }

    #[test]
    fn paired_large() {
        test(Vec::generate_random_large, 100_000)
    }

    #[test]
    #[should_panic]
    fn get_from_empty() {
        #[expect(clippy::unnecessary_operation)]
        Rope::<()>::new()[0];
    }

    #[test]
    #[should_panic]
    fn get_past_end() {
        #[expect(clippy::unnecessary_operation)]
        Rope::<()>::from_iter([(), ()])[2];
    }

    #[test]
    fn get_last() {
        for i in 1..33 {
            assert_eq!(
                Rope::from_iter(vec![(); i].into_iter().enumerate().map(|(i, _)| i))[i - 1],
                i - 1
            );
        }
    }

    #[test]
    fn delete_rebalance() {
        let arr_map = {
            let mut collection = Rope::from_iter((0..).take(BASE_SLOT_SIZE));
            for i in (BASE_SLOT_SIZE..).take(9) {
                collection.insert(i, i);
            }
            collection.remove(MAX_SLOT_SIZE);
            collection
        };

        let vec = {
            let mut collection = Vec::from_iter((0..).take(BASE_SLOT_SIZE));
            for i in (BASE_SLOT_SIZE..).take(9) {
                collection.insert(i, i);
            }

            collection.remove(MAX_SLOT_SIZE);
            collection
        };

        assert_eq!(
            vec.iter().collect::<Vec<_>>(),
            arr_map.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            vec.into_iter().collect::<Vec<_>>(),
            arr_map.into_iter().collect::<Vec<_>>()
        );
    }
}
