use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Add, Index, IndexMut, RangeBounds, Sub},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

const MAX_SLOT_SIZE: usize = 16;
const DEF_SLOT_SIZE: usize = 8;
const UNDERSIZED_SLOT: usize = 3;

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

impl Into<usize> for Key {
    fn into(self) -> usize {
        self.0.load(Relaxed)
    }
}

impl Into<usize> for &Key {
    fn into(self) -> usize {
        self.0.load(Relaxed)
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
            .map(|(k, content)| content.get(index - Into::<usize>::into(k)))
            .flatten()
            .unwrap()
    }
}

impl<T> IndexMut<usize> for Rope<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.0
            .iter_mut()
            .skip_while(|(k, content)| Into::<usize>::into(*k) + content.len() < index + 1)
            .next()
            .map(|(k, content)| content.get_mut(index - Into::<usize>::into(k)))
            .flatten()
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
                if let Some(_) = self.self_ref.0.get(key).and_then(|v| v.get(new_in_key)) {
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
        let mut iter = iter.into_iter();
        let mut counter = 0;
        let mut current = VecDeque::with_capacity(MAX_SLOT_SIZE);
        let mut map = BTreeMap::new();
        while let Some(item) = iter.next() {
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
            .map(|(k, _)| Key::from(Into::<usize>::into(k)))
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
        let mut v_iter = self.0.range(start_key..);
        let start = v_iter
            .next()
            .map(|(k, v)| Into::<usize>::into(k) + v.len())
            .unwrap_or(0);
        let mut total = v_iter.fold(start, |acc, (_, x)| acc + x.len());

        let v_iter = self.0.range(self.key_for_index(from)..);
        v_iter.rev().for_each(|(k, v)| {
            total -= v.len();
            k_set(&k.0, total);
        });
    }

    fn rebalance(&mut self, from: usize) {
        let key = self.key_for_index(from);
        let prev_high_index = self
            .0
            .range(..key)
            .rev()
            .next()
            .map(|(k, _)| Key::from(Into::<usize>::into(k)))
            .unwrap_or_default();
        let keys: Vec<Key> = self
            .0
            .range(&prev_high_index..)
            .map(|(k, _)| Key::from(Into::<usize>::into(k)))
            .collect();

        let mut carry = VecDeque::<T>::with_capacity(16);

        for (idx, key) in keys.iter().enumerate() {
            let entry = self.0.get_mut(&key).unwrap();
            if entry.is_empty() {
                continue;
            }
            if ((DEF_SLOT_SIZE - UNDERSIZED_SLOT + 1)..=(DEF_SLOT_SIZE + (DEF_SLOT_SIZE / 2)))
                .contains(&entry.len())
                && carry.is_empty()
            {
                break;
            }

            let mut hold = std::mem::take(entry);
            'inner: for inner_key in keys[(idx + 1)..].iter() {
                let inner_entry = self.0.get_mut(inner_key).unwrap();

                match (hold.len().cmp(&DEF_SLOT_SIZE), carry.len()) {
                    (std::cmp::Ordering::Less, 0) => hold.extend(
                        inner_entry.drain(..(DEF_SLOT_SIZE - hold.len()).min(inner_entry.len())),
                    ),
                    (std::cmp::Ordering::Equal, 0) => break 'inner,
                    (std::cmp::Ordering::Greater, 0) => carry.extend(hold.drain(DEF_SLOT_SIZE..)),
                    (_, carry_len) => {
                        carry.extend(hold.drain(..));
                        hold.extend(carry.drain(..DEF_SLOT_SIZE.min(carry_len)))
                    }
                }
            }

            if hold.len() > DEF_SLOT_SIZE {
                carry.extend(hold.drain(DEF_SLOT_SIZE..));
            }

            *self.0.get_mut(&key).unwrap() = hold;
        }

        self.0.retain(|_, v| !v.is_empty());
        self.renumber(prev_high_index.into());

        if carry.is_empty() {
            return;
        }

        let mut new_key = self
            .0
            .last_key_value()
            .map(|(k, v)| Into::<usize>::into(k) + v.len())
            .unwrap_or_default();
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
        let vec = self
            .0
            .entry(Key::from(Into::<usize>::into(&key)))
            .or_default();
        vec.insert(index - Into::<usize>::into(key), element);
        match vec.len() {
            ..=MAX_SLOT_SIZE => self.renumber(index),
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
    use super::Rope;

    #[test]
    fn test_insert() {
        let mut rope = Rope::new();
        let mut vec = Vec::new();
        for i in 0..17_u16 {
            rope.insert(rope.len(), i);
            vec.insert(vec.len(), i);
            println!("{}", rope);
        }

        rope.insert(3, 1337);
        vec.insert(3, 1337);

        println!("{}", rope);
        assert_eq!(rope.into_iter().collect::<Vec<_>>(), vec);
    }

    #[test]
    fn test_delete() {
        let mut rope = Rope::new();
        let mut vec = Vec::new();
        for i in 0..17_u16 {
            rope.insert(rope.len(), i);
            vec.insert(vec.len(), i);
            println!("{}", rope);
        }

        assert_eq!(vec.len(), rope.len());
        rope.remove(3);
        vec.remove(3);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        assert_eq!(vec.len(), rope.len());
        rope.remove(3);
        vec.remove(3);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        assert_eq!(vec.len(), rope.len());
        rope.remove(3);
        vec.remove(3);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        assert_eq!(vec.len(), rope.len());
        rope.remove(3);
        vec.remove(3);
        println!("{}", rope);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        assert_eq!(vec.len(), rope.len());
        rope.remove(3);
        vec.remove(3);
        println!("{}", rope);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);
    }

    #[test]
    fn test_drain() {
        let mut rope = Rope::new();
        let mut vec = Vec::new();
        for i in 0..17_u16 {
            rope.insert(rope.len(), i);
            vec.insert(vec.len(), i);
            println!("{}", rope);
        }

        rope.drain(..1);
        vec.drain(..1);
        println!("{}", rope);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        rope.drain(15..);
        vec.drain(15..);
        println!("{}", rope);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);

        rope.drain(6..10);
        vec.drain(6..10);
        println!("{}", rope);
        assert_eq!(&(&rope).into_iter().cloned().collect::<Vec<_>>(), &vec);
    }
}
