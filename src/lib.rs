#[cfg(feature = "nanoserde")]
use nanoserde::{DeBin, SerBin};

#[cfg(feature = "serde")]
use serde::{de::DeserializeOwned, Serialize};

pub use structdiff_derive::Difference;

pub mod collections;

#[cfg(all(feature = "nanoserde", feature = "serde", feature = "debug_diffs"))]
pub(crate) mod __private {
    use super::*;
    pub trait StructDiffOwnedBound:
        SerBin + DeBin + Serialize + DeserializeOwned + Clone + std::fmt::Debug
    {
    }
    impl<T: SerBin + DeBin + Serialize + DeserializeOwned + Clone + std::fmt::Debug>
        StructDiffOwnedBound for T
    {
    }

    pub trait StructDiffRefBound: SerBin + Serialize + Clone + std::fmt::Debug {}
    impl<T: SerBin + Serialize + Clone + std::fmt::Debug> StructDiffRefBound for T {}
}

#[cfg(all(feature = "nanoserde", not(feature = "serde"), feature = "debug_diffs"))]
pub(crate) mod __private {
    use super::*;

    pub trait StructDiffOwnedBound: SerBin + DeBin + Clone + std::fmt::Debug {}
    impl<T: SerBin + DeBin + Clone + std::fmt::Debug> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: SerBin + Clone + std::fmt::Debug {}
    impl<T: SerBin + Clone + std::fmt::Debug> StructDiffRefBound for T {}
}

#[cfg(all(feature = "serde", not(feature = "nanoserde"), feature = "debug_diffs"))]
pub(crate) mod __private {
    use super::*;

    pub trait StructDiffOwnedBound: Serialize + DeserializeOwned + Clone + std::fmt::Debug {}
    impl<T: Serialize + DeserializeOwned + Clone + std::fmt::Debug> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: Serialize + Clone + std::fmt::Debug {}
    impl<T: Serialize + Clone + std::fmt::Debug> StructDiffRefBound for T {}
}

#[cfg(all(
    not(feature = "serde"),
    not(feature = "nanoserde"),
    feature = "debug_diffs"
))]
pub(crate) mod __private {
    use super::*;

    pub trait StructDiffOwnedBound: Clone + std::fmt::Debug {}
    impl<T: Clone + std::fmt::Debug> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: Clone + std::fmt::Debug {}
    impl<T: Clone + std::fmt::Debug> StructDiffRefBound for T {}
}

#[cfg(all(feature = "nanoserde", feature = "serde", not(feature = "debug_diffs")))]
pub(crate) mod __private {
    use super::*;
    pub trait StructDiffOwnedBound: SerBin + DeBin + Serialize + DeserializeOwned + Clone {}
    impl<T: SerBin + DeBin + Serialize + DeserializeOwned + Clone> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: SerBin + Serialize + Clone {}
    impl<T: SerBin + Serialize + Clone> StructDiffRefBound for T {}
}

#[cfg(all(
    feature = "nanoserde",
    not(feature = "serde"),
    not(feature = "debug_diffs")
))]
pub(crate) mod __private {
    use super::*;

    pub trait StructDiffOwnedBound: SerBin + DeBin + Clone {}
    impl<T: SerBin + DeBin + Clone> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: SerBin + Clone {}
    impl<T: SerBin + Clone> StructDiffRefBound for T {}
}

#[cfg(all(
    feature = "serde",
    not(feature = "nanoserde"),
    not(feature = "debug_diffs")
))]
pub(crate) mod __private {
    use super::*;

    pub trait StructDiffOwnedBound: Serialize + DeserializeOwned + Clone {}
    impl<T: Serialize + DeserializeOwned + Clone> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: Serialize + Clone {}
    impl<T: Serialize + Clone> StructDiffRefBound for T {}
}

#[cfg(all(
    not(feature = "serde"),
    not(feature = "nanoserde"),
    not(feature = "debug_diffs")
))]
pub(crate) mod __private {

    pub trait StructDiffOwnedBound: Clone {}
    impl<T: Clone> StructDiffOwnedBound for T {}

    pub trait StructDiffRefBound: Clone {}
    impl<T: Clone> StructDiffRefBound for T {}
}

pub trait StructDiff {
    /// A generated type used to represent the difference
    /// between two instances of a struct which implements
    /// the StructDiff trait.
    type Diff: __private::StructDiffOwnedBound;

    /// A generated type used to represent the difference
    /// between two instances of a struct which implements
    /// the StructDiff trait (using references).
    type DiffRef<'target>: __private::StructDiffRefBound + Into<Self::Diff>
    where
        Self: 'target;

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
    /// let diffs: Vec<<Example as StructDiff>::Diff> = first.diff(&second);
    ///
    /// let diffed = first.apply(diffs);
    /// assert_eq!(diffed, second);
    /// ```
    fn diff(&self, updated: &Self) -> Vec<Self::Diff>;

    /// Generate a diff between two instances of a struct, for
    /// use in passing to serializer. Much more efficient for
    /// structs with large fields where the diff will not be stored.
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
    /// let diffs: Vec<<Example as StructDiff>::DiffRef<'_>> = first.diff_ref(&second);
    ///
    /// let diffed = first.clone().apply(diffs.into_iter().map(Into::into).collect());
    /// assert_eq!(diffed, second);
    /// ```
    fn diff_ref<'target>(&'target self, updated: &'target Self) -> Vec<Self::DiffRef<'target>>;

    /// Apply a single-field diff to a mutable self ref
    fn apply_single(&mut self, diff: Self::Diff);

    /// Apply a full diff to an owned self
    fn apply(mut self, diffs: Vec<Self::Diff>) -> Self
    where
        Self: Sized,
    {
        for diff in diffs {
            self.apply_single(diff);
        }
        self
    }

    /// Apply a full diff to a self ref, returning a cloned version of self
    /// after diff is applied
    fn apply_ref(&self, diffs: Vec<Self::Diff>) -> Self
    where
        Self: Clone,
    {
        self.clone().apply(diffs)
    }

    /// Apply a full diff to a mutable self ref
    fn apply_mut(&mut self, diffs: Vec<Self::Diff>) {
        for diff in diffs {
            self.apply_single(diff);
        }
    }
}
