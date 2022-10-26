pub use structdiff_derive::Difference;

pub trait StructDiff: PartialEq + Clone {
    type Diff;

    fn diff(&self, prev: &Self) -> Vec<Self::Diff>;
    fn apply_single(&mut self, diff: Self::Diff);
    fn apply(self, diffs: Vec<Self::Diff>) -> Self {
        let mut mut_self = self;
        for diff in diffs {
            mut_self.apply_single(diff);
        }
        mut_self
    }

    fn apply_ref(&self, diffs: Vec<Self::Diff>) -> Self {
        self.clone().apply(diffs)
    }

    fn apply_mut(&mut self, diffs: Vec<Self::Diff>) {
        for diff in diffs {
            self.apply_single(diff);
        }
    }
}
