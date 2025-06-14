use std::{
    cmp::Ordering::{Equal, Greater},
    collections::VecDeque,
    fmt::Debug,
    num::NonZeroUsize,
    ops::{Index, IndexMut, Neg},
};

/// helper type for [`NodesWithCount::slot_mut_internal`] and [`NodesWithCount::remove_internal`]
enum RetType {
    Further(usize),
    This(usize),
}

#[derive(Clone, Default)]
enum Node<T> {
    #[default]
    Empty,
    Single(T),
    Multiple(NodesWithCount<T>),
}

impl<T> Node<T> {
    fn is_occupied(&self) -> bool {
        matches!(
            self,
            Node::Multiple(NodesWithCount {
                count: Some(..),
                ..
            }) | Node::Single(..)
        )
    }
}

#[derive(Clone)]
struct NodesWithCount<T> {
    count: Option<NonZeroUsize>,
    nodes: VecDeque<Node<T>>,
}

impl<T> Default for NodesWithCount<T> {
    fn default() -> Self {
        Self {
            count: None,
            nodes: VecDeque::new(),
        }
    }
}

#[derive(Clone)]
pub struct Rope<T>(NodesWithCount<T>);

impl<T> NodesWithCount<T> {
    #[inline]
    fn shallow_len(&self) -> usize {
        self.count.map(NonZeroUsize::get).unwrap_or_default()
    }

    fn pop_front(&mut self) -> Option<T> {
        let node_mut = self
            .nodes
            .iter_mut()
            .find(|v| matches!(v, Node::Single(..) | Node::Multiple(..)))?;
        match node_mut {
            Node::Empty => unreachable!(),
            Node::Single(_) => {
                let Node::Single(v) = std::mem::take(node_mut) else {
                    unreachable!()
                };
                self.count = match self
                    .count
                    .expect("we're removing a node, count should be >= 1")
                {
                    NonZeroUsize::MIN => None,
                    higher => Some(NonZeroUsize::new(higher.get() - 1).unwrap()),
                };
                Some(v)
            }
            Node::Multiple(nodes) => {
                let v = nodes
                    .pop_front()
                    .expect("MUST be at least one value in a Node::Multiple");
                self.count = match self
                    .count
                    .expect("we're removing a node, count should be >= 1")
                {
                    NonZeroUsize::MIN => None,
                    higher => Some(NonZeroUsize::new(higher.get() - 1).unwrap()),
                };
                if nodes.shallow_len() == 0 {
                    std::mem::take(node_mut);
                }
                Some(v)
            }
        }
    }

    fn index_internal(&self, idx: usize, mut last: Option<usize>) -> Option<&T> {
        for node in &self.nodes {
            match (node, &mut last) {
                (Node::Empty, _) => continue,
                (Node::Single(v), Some(last)) if *last == idx - 1 => return Some(v),
                (Node::Single(_), Some(last)) => *last += 1,
                (Node::Single(v), None) if idx == 0 => return Some(v),
                (Node::Single(_), last @ None) => *last = Some(0),
                (Node::Multiple(nodes), last)
                    if nodes.shallow_len() + last.unwrap_or_default() >= idx =>
                {
                    return nodes.index_internal(idx, *last)
                }
                (Node::Multiple(nodes), last) => {
                    *last =
                        Some(last.unwrap_or_default() + nodes.count.map_or(0, NonZeroUsize::get))
                }
            }
        }

        None
    }

    fn index_mut_internal(&mut self, idx: usize, mut last: Option<usize>) -> Option<&mut T> {
        for node in self.nodes.iter_mut() {
            match node {
                Node::Empty => continue,
                Node::Single(v) => {
                    let curr = last.map(|l| l + 1).unwrap_or_default();
                    match curr == idx {
                        true => return Some(v),
                        false => last = Some(curr),
                    }
                }
                Node::Multiple(nodes) => {
                    // relying here on the fact that a Multiple MUST have at least one element
                    let endex = match last {
                        Some(last) => last + nodes.shallow_len(),
                        None => nodes.shallow_len() - 1,
                    };
                    match endex >= idx {
                        true => return nodes.index_mut_internal(idx, last),
                        false => last = Some(endex),
                    }
                }
            }
        }

        None
    }

    /// returns the parent of the Node::Single which holds the item at the relevant index, the
    /// index within the parent of that Node::Single, and whether cleanups are needed due to empty Nodes::Multiple.
    ///
    /// Apply an optional `adjustment` to the node count at each layer
    fn slot_mut_internal(
        &mut self,
        idx: usize,
        mut last: Option<usize>,
        adjustment: Option<isize>,
    ) -> Option<(&mut NodesWithCount<T>, usize)> {
        let mut return_idx = None;
        for (inner_idx, node) in self.nodes.iter_mut().enumerate() {
            match node {
                Node::Empty => continue,
                Node::Single(_) => {
                    let curr = last.map(|l| l + 1).unwrap_or_default();
                    match curr == idx {
                        true => {
                            return_idx = Some(RetType::This(inner_idx));
                            break;
                        }
                        false => last = Some(curr),
                    }
                }
                Node::Multiple(nodes) => {
                    // relying here on the fact that a Multiple MUST have at least one element
                    let endex = match last {
                        Some(last) => last + nodes.shallow_len(),
                        None => nodes.shallow_len() - 1,
                    };
                    match endex >= idx {
                        true => {
                            return_idx = Some(RetType::Further(inner_idx));
                            break;
                        }
                        false => last = Some(endex),
                    }
                }
            }
        }

        return_idx.map(|inner_idx| {
            match adjustment {
                Some(pos @ 1..) => {
                    self.count = Some(self.count.unwrap().checked_add(pos as usize).unwrap())
                }
                Some(neg @ ..-1) => {
                    self.count = NonZeroUsize::new(self.count.unwrap().get() - neg.neg() as usize);
                }
                _ => (),
            }
            match inner_idx {
                RetType::Further(i) => {
                    let Some(Node::Multiple(mult)) = self.nodes.get_mut(i) else {
                        panic!()
                    };
                    mult.slot_mut_internal(idx, last, adjustment).unwrap()
                }
                RetType::This(i) => (self, i),
            }
        })
    }

    fn insert_internal(&mut self, idx: usize, element: T) {
        match (self.count, idx) {
            (None, 0) => {
                return {
                    let len = self.nodes.len();
                    match len == 0 {
                        true => self.nodes.push_back(Node::Single(element)),
                        false => self.nodes[len.saturating_div(2)] = Node::Single(element),
                    }
                    self.count = Some(NonZeroUsize::MIN);
                }
            }
            (Some(len), idx) if len.get() == idx => {
                return {
                    // check if there's a Node::Empty that we can replace with a Node::Single(element), otherwise append to top level
                    let last_occupied = self
                        .nodes
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(idx, v)| v.is_occupied().then_some(idx))
                        .unwrap();
                    self.count = Some(
                        self.count
                            .map_or(NonZeroUsize::MIN, |v| v.checked_add(1).unwrap()),
                    );
                    match self.nodes.get_mut(last_occupied + 1) {
                        None => self.nodes.push_back(Node::Single(element)),
                        Some(empty) => *empty = Node::Single(element),
                    };
                };
            }
            _ => (),
        }

        // it's neither a front nor a back, find the internal slot that holds the item at the relevent index and either push it if
        // it's at front/back of its vec, or replace the Node::Single with a Node::Multiple
        let (nodes, slot, ..) = self.slot_mut_internal(idx, None, Some(1)).unwrap();

        // check for special cases
        if slot == 0 {
            return nodes.nodes.push_front(Node::Single(element));
        }
        if let Some(Node::Multiple(before)) = nodes.nodes.get_mut(slot - 1) {
            before.count = before.count.map(|c| c.checked_add(1)).unwrap();
            before.nodes.push_back(Node::Single(element));
            return;
        }

        let node_slot = &mut nodes.nodes[slot];
        let prev = std::mem::take(node_slot);
        debug_assert!(matches!(prev, Node::Single(..)));

        *node_slot = Node::Multiple(NodesWithCount {
            count: Some(NonZeroUsize::new(2).unwrap()),
            nodes: vec![Node::Single(element), prev].into(),
        });
    }

    /// Get the item at index `idx`, patching up Node type at each level if they become empty
    fn remove_internal(&mut self, idx: usize, mut last: Option<usize>) -> Option<T> {
        let mut return_idx = None;
        for (inner_idx, node) in self.nodes.iter_mut().enumerate() {
            match node {
                Node::Empty => continue,
                Node::Single(_) => {
                    let curr = last.map(|l| l + 1).unwrap_or_default();
                    match curr == idx {
                        true => {
                            return_idx = Some(RetType::This(inner_idx));
                            break;
                        }
                        false => last = Some(curr),
                    }
                }
                Node::Multiple(nodes) => {
                    // relying here on the fact that a Multiple MUST have at least one element
                    let endex = match last {
                        Some(last) => last + nodes.count.unwrap().get(),
                        None => nodes.count.unwrap().get() - 1,
                    };
                    match endex >= idx {
                        true => {
                            return_idx = Some(RetType::Further(inner_idx));
                            break;
                        }
                        false => last = Some(endex),
                    }
                }
            }
        }

        let inner_idx = return_idx?;

        self.count = NonZeroUsize::new(self.count.unwrap().get() - 1_usize);

        match inner_idx {
            RetType::Further(i) => {
                let Some(Node::Multiple(mult)) = self.nodes.get_mut(i) else {
                    panic!()
                };
                let ret = mult.remove_internal(idx, last);

                if mult.count.is_none() {
                    self.nodes[i] = Node::Empty;
                }

                ret
            }
            RetType::This(i) => {
                let Some(Node::Single(single)) = self.nodes.get_mut(i).map(std::mem::take) else {
                    panic!()
                };
                Some(single)
            }
        }
    }

    fn swap_internal(&mut self, [low, high]: [usize; 2], _arg: i32) {
        let high_elem = self.remove_internal(high, None).unwrap();
        let (nodes, slot) = self.slot_mut_internal(low, None, None).unwrap();

        let Node::Single(low) = std::mem::replace(&mut nodes.nodes[slot], Node::Single(high_elem))
        else {
            panic!();
        };
        self.insert_internal(high, low);
    }

    fn drain_internal(&mut self, l_idx: usize, r_idx: usize, mut last: Option<usize>) {
        let mut removed = 0;
        for node in self.nodes.iter_mut() {
            match node {
                Node::Empty => continue,
                single @ Node::Single(_) => {
                    let new_last = last.map(|i| i + 1).unwrap_or_default();
                    if (l_idx..=r_idx).contains(&new_last) {
                        *single = Node::Empty;
                        removed += 1;
                    }
                    last = Some(new_last);
                }
                Node::Multiple(nodes_with_count) => {
                    let nodes_with_count_len = nodes_with_count.count.unwrap().get();
                    // get the endex of the last element of nodes_with_count
                    let last_after_sub = match last {
                        Some(last) => last + nodes_with_count_len,
                        None => nodes_with_count_len - 1,
                    };

                    // check if the endex is still lower than the lowest value we are to remove
                    if last_after_sub < l_idx {
                        last = Some(last_after_sub);
                        continue;
                    }

                    // we need to remove some things; figure out if we can remove the entire subtree or need to recurse
                    match r_idx.cmp(&last_after_sub) {
                        // we can yeet the whole thing
                        Equal | Greater
                            if last.map_or_else(
                                || l_idx == last.unwrap_or_default(),
                                |l| l_idx <= l + 1,
                            ) =>
                        {
                            let before = nodes_with_count_len;
                            *node = Node::Empty;
                            removed += before;
                        }
                        // we need to remove only part of this node
                        _ => {
                            let before = nodes_with_count_len;
                            nodes_with_count.drain_internal(l_idx, r_idx, last);
                            let after = nodes_with_count.count.unwrap().get();
                            removed += before.abs_diff(after);
                        }
                    }
                    last = Some(last_after_sub);
                }
            }

            if last.is_some_and(|l| l >= r_idx) {
                break;
            }
        }

        self.count = self
            .count
            .map(|c| c.get() - removed)
            .and_then(NonZeroUsize::new);
    }
}

impl<T> FromIterator<T> for Rope<T> {
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let nodes = iter
            .into_iter()
            .map(|t| Node::Single(t))
            .collect::<VecDeque<_>>();
        Self(NodesWithCount {
            count: NonZeroUsize::try_from(nodes.len()).ok(),
            nodes,
        })
    }
}

pub struct IntoIter<T> {
    owned: NodesWithCount<T>,
}

impl<T> IntoIterator for Rope<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { owned: self.0 }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.owned.pop_front()
    }
}

impl<T> Index<usize> for Rope<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.0
            .index_internal(index, None)
            .expect("Failed to find element")
    }
}

impl<T> IndexMut<usize> for Rope<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0
            .index_mut_internal(index, None)
            .expect("Failed to find element")
    }
}

impl<T: Debug> std::fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Node::Empty"),
            Self::Single(arg0) => f.debug_tuple("Node::Single").field(arg0).finish(),
            Self::Multiple(arg0) => f.debug_tuple("Node::Multiple").field(arg0).finish(),
        }
    }
}

impl<T: Debug> std::fmt::Debug for NodesWithCount<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodesWithCount")
            .field("count", &self.count)
            .field("nodes", &self.nodes)
            .finish()
    }
}

impl<T: Debug> std::fmt::Debug for Rope<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Rope").field(&self.0).finish()
    }
}

impl<T> Rope<T> {
    pub fn len(&self) -> usize {
        self.0.count.map_or(0, usize::from)
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.0.insert_internal(index, element);
    }

    pub fn remove(&mut self, index: usize) {
        self.0
            .remove_internal(index, None)
            .expect("No item at index");
    }

    pub fn drain<R: std::ops::RangeBounds<usize>>(&mut self, range: R) {
        use std::ops::Bound;

        let (l_idx, r_idx) = match (range.start_bound(), range.end_bound()) {
            (Bound::Included(l_i), Bound::Excluded(r_e)) if l_i == r_e => return,
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

        if r_idx == 0 && l_idx == 0 && self.0.shallow_len() == 0 {
            return;
        }

        self.0.drain_internal(l_idx, r_idx, None);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a != b {
            self.0.swap_internal([a.min(b), a.max(b)], 0);
        }
    }
}
