macro_rules! l {
    ($target:ident, $line:expr) => {
        $target.push_str($line)
    };

    ($target:ident, $line:expr, $($param:expr),*) => {
        $target.push_str(&::alloc::format!($line, $($param,)*))
    };
}

#[derive(Debug, Default)]
pub enum MapStrategy {
    KeyOnly,
    #[default]
    KeyAndValue,
}

#[derive(Debug)]
pub enum CollectionStrategy {
    OrderedArrayLike,
    UnorderedArrayLikeHash,
    UnorderedMapLikeHash(MapStrategy),
}

#[cfg(feature = "generated_setters")]
pub fn attrs_setter(attributes: &[crate::parse::Attribute]) -> (bool, bool, Option<String>) {
    let skip = attributes
        .iter()
        .any(|attr| attr.tokens.len() == 1 && attr.tokens[0] == "skip_setter");
    let local = attributes
        .iter()
        .any(|attr| attr.tokens.len() == 1 && attr.tokens[0] == "setter");

    let Some(name_override) = attributes.iter().find_map(|attr| {
        if attr.tokens.len() == 2 && attr.tokens[0] == "setter_name" {
            Some(attr.tokens[1].clone())
        } else {
            None
        }
    }) else {
        return (local, skip, None);
    };

    (local, skip, Some(name_override))
}

#[cfg(feature = "generated_setters")]
pub fn attrs_all_setters(attributes: &[crate::parse::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| attr.tokens.len() == 1 && attr.tokens[0] == "setters")
}

pub fn attrs_recurse(attributes: &[crate::parse::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| attr.tokens.len() == 1 && attr.tokens[0] == "recurse")
}

pub fn attrs_skip(attributes: &[crate::parse::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| attr.tokens.len() == 1 && attr.tokens[0] == "skip")
}

pub fn attrs_collection_type(attributes: &[crate::parse::Attribute]) -> Option<CollectionStrategy> {
    attributes.iter().find_map(|attr| {
        if attr.tokens.len() == 2 && attr.tokens[0] == "collection_strategy" {
            let strategy = match attr.tokens[1].clone().as_str() {
                "ordered_array_like" => CollectionStrategy::OrderedArrayLike,
                "unordered_array_like" => CollectionStrategy::UnorderedArrayLikeHash,
                "unordered_map_like" => {
                    let map_compare_type = attrs_map_strategy(attributes).unwrap_or_default();
                    CollectionStrategy::UnorderedMapLikeHash(map_compare_type)
                }
                _ => {
                    return None;
                }
            };
            Some(strategy)
        } else {
            None
        }
    })
}

pub fn attrs_map_strategy(attributes: &[crate::parse::Attribute]) -> Option<MapStrategy> {
    attributes.iter().find_map(|attr| {
        if attr.tokens.len() == 2 && attr.tokens[0] == "map_equality" {
            let strategy = match attr.tokens[1].clone().as_str() {
                "key_only" => MapStrategy::KeyOnly,
                "key_and_value" => MapStrategy::KeyAndValue,
                _ => {
                    return None;
                }
            };
            Some(strategy)
        } else {
            None
        }
    })
}

pub fn attrs_expose(attributes: &[crate::parse::Attribute]) -> Option<Option<String>> {
    attributes.iter().find_map(|attr| match attr.tokens.len() {
        1 if attr.tokens[0].starts_with("expose") => Some(None),
        2.. if attr.tokens[0] == "expose" => Some(Some(attr.tokens[1].to_string())),
        _ => None,
    })
}
