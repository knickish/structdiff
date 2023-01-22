# structdiff

A lightweight, zero-dependency struct diffing library which allows changed fields to be collected and applied. Derive `Difference` on a struct, then use the `StructDiff` trait to make and apply diffs. 

## Example:

```rust
use structdiff::{Difference, StructDiff};

#[derive(Debug, PartialEq, Clone, Difference)]
struct Example {
    field1: f64,
    #[difference(skip)]
    field2: Vec<i32>,
    #[difference(collection_strategy="unordered_array_like")]
    field3: BTreeSet<usize>,
}

let first = Example {
    field1: 0.0,
    field2: vec![],
    field3: vec![1, 2, 3].into_iter().collect(),
};

let second = Example {
    field1: 3.14,
    field2: vec![1],
    field3: vec![2, 3, 4].into_iter().collect(),
};

let diffs = first.diff(&second);
// diffs is now a Vec of differences between the two instances, 
// with length equal to number of changed/unskipped fields
assert_eq!(diffs.len(), 2);

let diffed = first.apply(diffs);
// diffed is now equal to second, except for skipped field
assert_eq!(diffed.field1, second.field1);
assert_eq!(&diffed.field3, &second.field3);
assert_ne!(diffed, second);
```

For more examples take a look at [integration tests](/tests)

## Derive macro attributes
- `#[difference(skip)]`     - Do not consider this field when creating a diff
- `#[difference(recurse)]`  - Generate a StructDiff for this field when creating a diff
- `#[difference(collection_strategy = {})]` 
    - `"unordered_array_like"` - Generates a changeset for array-like collections of items which implement `Hash + Eq`, rather than cloning the entire list. (currently works for `Vec`, `BTreeSet`, `LinkedList`, and `HashSet`)
    - `"unordered_map_like"` - Generates a changeset for map-like collections for which the key implements `Hash + Eq`, rather than cloning the entire map. (currently works for `HashMap` and `BTreeMap`)
- `#[difference(map_equality = {})]` - Used with `unordered_map_like`, specifies whether the equality check should consider keys only, or both keys and values
    - `"key_only"` - only replace a key-value pair for which the key has changed
    - `"key_and_value"` - replace a key-value pair if either the key or value has changed

## Optional features
- [`nanoserde`, `serde`] - Serialization of `Difference` derived associated types. Allows diffs to easily be sent over network.

### Development status 
This is being used actively for my own projects, although it's mostly working now. PRs will be accepted for either more tests or functionality.