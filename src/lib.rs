pub use structdiff_derive::Difference;

pub mod collections;

pub trait StructDiff: PartialEq + Sized {
    /// A generated type used to represent the difference
    /// between two instances of a struct which implements
    /// the StructDiff trait.
    type Diff;

    /// Generate a diff between two instances of a struct.
    /// This diff may be serialized if one of the serialization
    /// features is enabled.
    ///
    /// ```
    /// use structdiff::{Difference, StructDiff};
    ///
    /// #[derive(Debug, PartialEq, Clone, Difference)]
    /// struct Example {
    ///     field1: f64,
    /// }
    ///
    /// let first = Example {
    ///     field1: 0.0,
    /// };
    ///
    /// let second = Example {
    ///     field1: 3.14,
    /// };
    ///
    /// let diffs = first.diff(&second);
    ///
    /// let diffed = first.apply(diffs);
    /// assert_eq!(diffed, second);
    /// ```
    fn diff(&self, updated: &Self) -> Vec<Self::Diff>;

    /// Apply a single-field diff to a mutable self ref
    fn apply_single(&mut self, diff: Self::Diff);

    /// Apply a full diff to an owned self
    fn apply(mut self, diffs: Vec<Self::Diff>) -> Self {
        for diff in diffs {
            self.apply_single(diff);
        }
        self
    }

    /// Apply a full diff to a self ref, returning a cloned version of self
    /// after diff is applied
    fn apply_ref(&self, diffs: Vec<Self::Diff>) -> Self where Self: Clone {
        self.clone().apply(diffs)
    }

    /// Apply a full diff to a mutable self ref
    fn apply_mut(&mut self, diffs: Vec<Self::Diff>) {
        for diff in diffs {
            self.apply_single(diff);
        }
    }
}
