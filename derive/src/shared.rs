macro_rules! l {
    ($target:ident, $line:expr) => {
        $target.push_str($line)
    };

    ($target:ident, $line:expr, $($param:expr),*) => {
        $target.push_str(&::alloc::format!($line, $($param,)*))
    };
}

#[derive(Debug)]
pub enum CollectionStrategy {
    UnorderedHash,
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

pub fn attrs_collection(attributes: &[crate::parse::Attribute]) -> Option<CollectionStrategy> {
    attributes.iter().find_map(|attr| {
        if attr.tokens.len() == 2 && attr.tokens[0] == "collection_strategy" {
            let strategy = match attr.tokens[1].clone().as_str() {
                "unordered_hash" => CollectionStrategy::UnorderedHash,
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
