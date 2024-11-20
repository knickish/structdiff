use std::{
    cmp::Ordering::{Equal, Greater, Less},
    collections::{BTreeMap, VecDeque},
    ops::{Add, Index, IndexMut, RangeBounds, Sub},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

const MAX_SLOT_SIZE: usize = 16;
const DEF_SLOT_SIZE: usize = 8;
const UNDERSIZED_SLOT: usize = 1;

#[cfg_attr(test, derive(Clone))]
pub struct Rope<T>(BTreeMap<Key, VecDeque<T>>);

pub struct Iter<'rope, T> {
    self_ref: &'rope Rope<T>,
    key: usize,
    in_key: usize,
    exhausted: bool,
}
pub struct IntoIter<T> {
    self_own: Rope<T>,
    internal: Option<<VecDeque<T> as IntoIterator>::IntoIter>,
}

#[derive(Debug, Default)]
#[repr(transparent)]
struct Key(AtomicUsize);

impl Clone for Key {
    #[inline]
    fn clone(&self) -> Self {
        Key::from(Into::<usize>::into(self))
    }
}

impl Add for Key {
    type Output = Key;

    fn add(self, rhs: Self) -> Self::Output {
        self.0.fetch_add(k_load(rhs.0), Relaxed);
        self
    }
}

impl Add<usize> for Key {
    type Output = Key;

    fn add(self, rhs: usize) -> Self::Output {
        self.0.fetch_add(rhs, Relaxed);
        self
    }
}

impl Sub for Key {
    type Output = Key;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.fetch_sub(k_load(rhs.0), Relaxed);
        self
    }
}

impl Sub<usize> for Key {
    type Output = Key;

    fn sub(self, rhs: usize) -> Self::Output {
        self.0.fetch_sub(rhs, Relaxed);
        self
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(Relaxed) == other.0.load(Relaxed)
    }
}

impl From<usize> for Key {
    fn from(value: usize) -> Self {
        Self(AtomicUsize::new(value))
    }
}

impl From<Key> for usize {
    fn from(val: Key) -> Self {
        val.0.load(Relaxed)
    }
}

impl From<&Key> for usize {
    fn from(val: &Key) -> Self {
        val.0.load(Relaxed)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.load(Relaxed).partial_cmp(&other.0.load(Relaxed))
    }
}

impl Eq for Key {}
impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // this is a usize, unless the load fails
        // we should always succeed at comparing
        self.partial_cmp(other).unwrap()
    }
}

#[inline(always)]
fn k_load(k: AtomicUsize) -> usize {
    k.load(Relaxed)
}

#[inline(always)]
fn k_set(k: &AtomicUsize, val: usize) {
    k.store(val, Relaxed)
}

impl<T> Index<usize> for Rope<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.0
            .iter()
            .skip_while(|(k, content)| Into::<usize>::into(*k) + content.len() < index + 1)
            .next()
            .and_then(|(k, content)| content.get(index - Into::<usize>::into(k)))
            .unwrap()
    }
}

impl<T> IndexMut<usize> for Rope<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.0
            .iter_mut()
            .skip_while(|(k, content)| Into::<usize>::into(*k) + content.len() < index + 1)
            .next()
            .and_then(|(k, content)| content.get_mut(index - Into::<usize>::into(k)))
            .unwrap()
    }
}

impl<'rope, T: 'rope> Iterator for Iter<'rope, T> {
    type Item = &'rope T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        let ret = self
            .self_ref
            .0
            .get(&self.key.into())
            .and_then(|v| v.get(self.in_key));
        let mut new_in_key = self.in_key + 1;
        for key in self
            .self_ref
            .0
            .keys()
            .skip_while(|k| Into::<usize>::into(*k) != self.key)
        {
            let max_in_slot = self
                .self_ref
                .0
                .get(key)
                .map(VecDeque::len)
                .unwrap_or_default();

            while new_in_key < max_in_slot {
                if self
                    .self_ref
                    .0
                    .get(key)
                    .and_then(|v| v.get(new_in_key))
                    .is_some()
                {
                    self.key = Into::<usize>::into(key);
                    self.in_key = new_in_key;
                    return ret;
                }
                new_in_key += 1;
            }
            new_in_key = 0;
        }
        self.exhausted = true;
        ret
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let ret @ Some(_) = self.internal.as_mut().and_then(|internal| internal.next()) {
            return ret;
        }

        while let Some(mut vec_iter) = self.self_own.0.pop_first().map(|(_, vec)| vec.into_iter()) {
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
            self_own: self,
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
            exhausted: false,
        }
    }
}

impl<T> FromIterator<T> for Rope<T> {
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let iter = iter.into_iter();
        let mut counter = 0;
        let mut current = VecDeque::with_capacity(MAX_SLOT_SIZE);
        let mut map = BTreeMap::new();
        for item in iter {
            current.push_back(item);
            counter += 1;
            if counter % DEF_SLOT_SIZE == 0 {
                map.insert(
                    Key::from(counter - DEF_SLOT_SIZE),
                    std::mem::replace(&mut current, VecDeque::with_capacity(MAX_SLOT_SIZE)),
                );
            }
        }

        if !current.is_empty() {
            map.insert(Key::from(counter - current.len()), current);
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
        Self(BTreeMap::from([(
            Key(AtomicUsize::default()),
            VecDeque::with_capacity(MAX_SLOT_SIZE),
        )]))
    }

    #[inline]
    fn key_for_index(&self, index: usize) -> Key {
        use std::ops::Bound::*;
        self.0
            .range((Unbounded, Included(Key::from(index))))
            .last()
            .map(|(k, _)| k.clone())
            .unwrap_or_default()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0
            .last_key_value()
            .map(|(k, v)| Into::<usize>::into(k) + v.len())
            .unwrap_or_default()
    }

    fn renumber(&mut self, from: usize) {
        let start_key = self.key_for_index(from);
        let mut v_iter = self.0.range(&start_key..);
        let start = v_iter
            .next()
            .map(|(k, v)| Into::<usize>::into(k) + v.len())
            .unwrap_or(0);
        let mut total = v_iter.fold(start, |acc, (_, x)| acc + x.len());

        let v_iter = self.0.range(&start_key..);
        v_iter.rev().for_each(|(k, v)| {
            total -= v.len();
            k_set(&k.0, total);
        });
    }

    fn rebalance(&mut self, from: usize) {
        use std::ops::Bound::*;
        let key = self.key_for_index(from);
        let prev_high_index = self
            .0
            .range(..key)
            .next_back()
            .map(|(k, _)| k.clone())
            .unwrap_or_default();
        let keys: Vec<Key> = self
            .0
            .range(&prev_high_index..)
            .map(|(k, _)| k.clone())
            .collect();

        let mut carry = VecDeque::<T>::with_capacity(16);
        let mut hold = VecDeque::<T>::with_capacity(0);

        for key in keys.iter() {
            let entry = self.0.get_mut(key).unwrap();
            if entry.is_empty() {
                continue;
            }
            if ((DEF_SLOT_SIZE - UNDERSIZED_SLOT + 1)..=(DEF_SLOT_SIZE + (DEF_SLOT_SIZE / 2)))
                .contains(&entry.len())
                && carry.is_empty()
            {
                break;
            }

            // put the empty holder in the list for now
            std::mem::swap(entry, &mut hold);

            'inner: for (_inner_key, inner_entry) in self.0.range_mut((Excluded(key), Unbounded)) {
                match (hold.len().cmp(&DEF_SLOT_SIZE), carry.len()) {
                    (Less, 0) => hold.extend(
                        inner_entry.drain(..(DEF_SLOT_SIZE - hold.len()).min(inner_entry.len())),
                    ),
                    (Equal, 0) => break 'inner,
                    (Greater, 0) => {
                        carry.extend(hold.drain(DEF_SLOT_SIZE..));
                        break 'inner;
                    }
                    (_, _) => {
                        carry.extend(hold.drain(..));
                        hold.extend(carry.drain(..DEF_SLOT_SIZE.min(carry.len())));
                        if hold.len() == DEF_SLOT_SIZE {
                            break 'inner;
                        }
                    }
                }
            }

            // take the empty holder back and leave the values in the map entry
            std::mem::swap(self.0.get_mut(key).unwrap(), &mut hold);
        }

        self.0.retain(|_, v| !v.is_empty());
        self.renumber(prev_high_index.into());

        // fix up the last entry with any carried values
        match (carry.len(), self.0.last_entry()) {
            (0, ..) => return,
            (_, Some(mut l_entry)) => {
                let l_entry = l_entry.get_mut();
                carry.extend(l_entry.drain(..));
                l_entry.extend(carry.drain(..DEF_SLOT_SIZE.min(carry.len())));
            }
            _ => (),
        }

        // add any remaining carry values into new slots at the end
        let mut new_key = self.len();
        while !carry.is_empty() {
            let carry_len = carry.len();
            match carry_len > DEF_SLOT_SIZE {
                true => {
                    self.0
                        .insert(Key::from(new_key), carry.drain(..DEF_SLOT_SIZE).collect());
                    new_key += DEF_SLOT_SIZE;
                }
                false => {
                    self.0.insert(Key::from(new_key), carry);
                    return;
                }
            }
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        let key = self.key_for_index(index);
        let vec = self.0.entry(key.clone()).or_default();
        vec.insert(index - Into::<usize>::into(key), element);
        match vec.len() {
            oversized if (0..MAX_SLOT_SIZE).contains(&oversized) => self.renumber(index),
            _ => self.rebalance(index),
        }
    }

    pub fn remove(&mut self, index: usize) {
        let key = self.key_for_index(index);
        let vec = self.0.get_mut(&key).unwrap();
        vec.remove(index - Into::<usize>::into(&key));
        match vec.len() {
            0..=UNDERSIZED_SLOT => self.rebalance(Into::<usize>::into(&key)),
            _ => self.renumber(key.into()),
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

        let [l_key, r_key] = [l_idx, r_idx].map(|idx| self.key_for_index(idx));

        match l_key == r_key {
            true => {
                let v = self.0.get_mut(&l_key).expect("we just looked this key up");
                v.drain(
                    (l_idx - Into::<usize>::into(&l_key))..=(r_idx - Into::<usize>::into(&l_key)),
                );
                if v.len() <= UNDERSIZED_SLOT {
                    self.rebalance(Into::<usize>::into(l_key).saturating_sub(1));
                } else {
                    self.renumber(Into::<usize>::into(&l_key).saturating_sub(1));
                }
            }
            false => {
                self.0
                    .get_mut(&l_key)
                    .unwrap()
                    .drain((l_idx - Into::<usize>::into(&l_key))..);
                self.0
                    .range_mut((Bound::Excluded(&l_key), Bound::Excluded(&r_key)))
                    .for_each(|(_, v)| v.clear());
                self.0
                    .get_mut(&r_key)
                    .unwrap()
                    .drain(..=(r_idx - Into::<usize>::into(r_key)));
                self.rebalance(l_idx);
            }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let [l_key, r_key] = [a, b].map(|idx| self.key_for_index(idx));
        match l_key == r_key {
            true => self.0.get_mut(&l_key).unwrap().swap(
                a - Into::<usize>::into(&l_key),
                b - Into::<usize>::into(&l_key),
            ),
            false => {
                // more complicated with safe rust than in stdlib Vec
                let (rk, mut rv) = self.0.remove_entry(&r_key).unwrap();
                std::mem::swap(
                    self.0
                        .get_mut(&l_key)
                        .unwrap()
                        .get_mut(a - Into::<usize>::into(l_key))
                        .unwrap(),
                    rv.get_mut(b - Into::<usize>::into(r_key)).unwrap(),
                );
                self.0.insert(rk, rv);
            }
        }
    }
}

#[cfg(test)]
impl<T: std::fmt::Display> std::fmt::Display for Rope<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, vals) in self.0.iter() {
            write!(f, "{}: [", Into::<usize>::into(key))?;
            for val in vals {
                write!(f, "{},", val)?;
            }
            write!(f, "];\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use nanorand::{Rng, WyRand};

    use super::Rope;

    fn rand_string(rng: &mut WyRand) -> String {
        let base = vec![(); rng.generate_range::<usize, _>(1..=50)];
        base.into_iter()
            .map(|_| rng.generate_range::<u32, _>(65..=90) as u32)
            .filter_map(char::from_u32)
            .collect::<String>()
    }

    trait Random {
        fn generate_random(rng: &mut WyRand) -> Self;
        fn generate_random_large(rng: &mut WyRand) -> Self;
        fn random_mutate(self, mutation: Mutation<String>) -> Self;
    }

    impl Random for Rope<String> {
        fn generate_random(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u8, _>(5..15))
                .map(|_| rand_string(rng))
                .into_iter()
                .collect()
        }

        fn generate_random_large(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .into_iter()
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

    #[derive(Debug, Clone)]
    enum Mutation<T> {
        Insert(T, usize),
        Remove(usize),
        Swap(usize, usize),
        Drain(usize, usize),
    }

    impl Mutation<String> {
        fn random_mutation(rng: &mut WyRand, len: usize) -> Option<Mutation<String>> {
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
                    Some(Self::Swap(l, r))
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
                .into_iter()
                .collect()
        }

        fn generate_random_large(rng: &mut WyRand) -> Self {
            (0..rng.generate_range::<u16, _>(0..(u16::MAX / 5)))
                .map(|_| rand_string(rng))
                .into_iter()
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
            assert_eq!(
                (&start_rope).into_iter().cloned().collect::<Vec<_>>(),
                start_rope.clone().into_iter().collect::<Vec<_>>()
            );
            assert_eq!(
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
        test(Vec::generate_random_large, 500)
    }
}
