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
    field3: String,
}

let first = Example {
    field1: 0.0,
    field2: Vec::new(),
    field3: String::from("Hello Diff"),
};

let second = Example {
    field1: 3.14,
    field2: vec![1],
    field3: String::from("Hello Diff"),
};

let diffs = first.diff(&second);
// diffs is now a Vec of differences, with length 
// equal to number of changed/unskipped fields
assert_eq!(diffs.len(), 1);

let diffed = first.apply(diffs);
// diffed is now equal to second, except for skipped field
assert_eq!(diffed.field1, second.field1);
assert_eq!(diffed.field3, second.field3);
assert_ne!(diffed, second); 
```

For more examples take a look at [integration tests](/tests)

## Derive macro attributes
- `#[difference(skip)]`     - Do not consider this field when creating a diff
- `#[difference(recurse)]`  - Generate a StructDiff for this field when creating a diff
- `#[difference(collection_strategy = "{}")` 
    - `unordered_hash` - Generates a changeset for collections of items which implement `Hash + Eq`, rather than cloning the entire list. (currently works for `Vec`, `BTreeSet`, `LinkedList`, and `HashSet`)

## Optional features
- [`nanoserde`, `serde`] - Serialization of `Difference` derived associated types

### Development status 
This is being actively worked on (especially on collections strategies). PRs will be accepted for either more tests or functionality.