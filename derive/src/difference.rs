use std::collections::{HashMap, HashSet};

use alloc::format;
use alloc::string::String;

use crate::parse::Struct;
use crate::shared::{attrs_collection_type, attrs_recurse, attrs_skip};

use proc_macro::TokenStream;

pub(crate) fn derive_struct_diff_struct(struct_: &Struct) -> TokenStream {
    let derives: String = vec![
        "Debug",
        "Clone",
        #[cfg(feature = "nanoserde")]
        "nanoserde::SerBin",
        #[cfg(feature = "nanoserde")]
        "nanoserde::DeBin",
        #[cfg(feature = "serde")]
        "serde::Serialize",
        #[cfg(feature = "serde")]
        "serde::Deserialize",
    ]
    .join(", ");

    let mut diff_enum_body = String::new();
    let mut diff_body = String::new();
    let mut apply_single_body = String::new();
    let mut type_aliases = String::new();
    let mut used_generics: Vec<String> = Vec::new();

    let enum_name = String::from("__".to_owned() + struct_.name.as_str() + "StructDiffEnum");
    let struct_generics: HashMap<&String, &Vec<String>> = struct_
        .generics
        .iter()
        .map(|entry| (&entry.0, &entry.1))
        .collect();

    struct_
        .fields
        .iter()
        .filter(|x| !attrs_skip(&x.attributes))
        .enumerate()
        .for_each(|(index, field)| {
            let field_name = field.field_name.as_ref().unwrap();
            used_generics.extend(struct_.generics.iter().filter(|x| x.0 == field.ty.path).map(|x|x.0.clone()));
            if let Some(wrapped) = field.ty.wraps.as_ref() {
                let to_add = wrapped.iter().filter(|x| struct_generics.contains_key(x));
                used_generics.extend(to_add.cloned());
            }

            match (attrs_recurse(&field.attributes), attrs_collection_type(&field.attributes), field.ty.is_option) {
                (false, None, false) => {  // The default case
                    l!(diff_enum_body, " {}({}),", field_name, field.ty.path);
                    
                    l!(
                        apply_single_body,
                        "Self::Diff::{}(__{}) => self.{} = __{},",
                        field_name,
                        index,
                        field_name,
                        index
                    );

                    l!(
                        diff_body,
                        "if self.{} != updated.{} {{diffs.push(Self::Diff::{}(updated.{}.clone()))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                },
                (false, None, true) => {  // The default case, but with an option

                    l!(diff_enum_body, " {}(Option<{}>),", field_name, field.ty.path);

                    l!(
                        apply_single_body,
                        "Self::Diff::{}(__{}) => self.{} = __{},",
                        field_name,
                        index,
                        field_name,
                        index
                    );

                    l!(
                        diff_body,
                        "if self.{} != updated.{} {{diffs.push(Self::Diff::{}(updated.{}.clone()))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                },
                (true, None, false)  => { // Recurse inwards and generate a Vec<SubStructDiff> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.path);

                    l!(diff_enum_body, " {}({}),", field_name, typename);

                    l!(
                        apply_single_body,
                        "Self::Diff::{}(__{}) => self.{} = self.{}.apply_ref(__{}),",
                        field_name,
                        index,
                        field_name,
                        field_name,
                        index
                    );

                    l!(
                        diff_body,
                        "if &self.{} != &updated.{} {{diffs.push(Self::Diff::{}(self.{}.diff(&updated.{})))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                },
                (true, None, true)  => { // Recurse inwards and generate an Option<Vec<SubStructDiff>> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.path);

                    l!(diff_enum_body, " {}(Option<{}>),", field_name, typename);
                    l!(diff_enum_body, " {}_full({}),", field_name, field.ty.path);

                    l!(
                        apply_single_body,
                        "Self::Diff::{}(Some(__{})) => if let Some(ref mut inner) = self.{} {{ 
                            inner.apply_mut(__{});
                        }},",
                        field_name,
                        index,
                        field_name,
                        index
                    );

                    l!(
                        apply_single_body,
                        "Self::Diff::{}_full(__{}) => self.{} = Some(__{}),",
                        field_name,
                        index,
                        field_name,
                        index
                    );

                    l!(
                        apply_single_body,
                        "Self::Diff::{}(None) => self.{} = None,",
                        field_name,
                        field_name
                    );

                    l!(
                        diff_body,
                        "match (&self.{}, &updated.{}) {{
                            (Some(val1), Some(val2)) if &val1 != &val2 => diffs.push(Self::Diff::{}(Some(val1.diff(&val2)))),
                            (Some(val1), None) => diffs.push(Self::Diff::{}(None)),
                            (None, Some(val2)) => diffs.push(Self::Diff::{}_full(val2.clone())),
                            _ => (),
                        }};",
                        field_name,
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                },
                (true, Some(_), _) => panic!("Recursion inside of collections is not yet supported"),
                (false, Some(_), true) => panic!("Collection strategies inside of options are not yet supported"),
                (false, Some(strat), false) => match strat {
                    crate::shared::CollectionStrategy::UnorderedArrayLikeHash => {
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_array_like::UnorderedArrayLikeDiff<{}>),", field_name, field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection").join(","));

                        l!(
                            apply_single_body,
                            "Self::Diff::{}(__{}) => self.{} = structdiff::collections::unordered_array_like::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                            field_name,
                            index,
                            field_name,
                            field_name,
                            index
                        );

                        l!(
                            diff_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_array_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter()) {{
                                diffs.push(Self::Diff::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );
                    },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(map_strat) => match map_strat {
                        crate::shared::MapStrategy::KeyOnly => {
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name,field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection").join(","));

                            l!(
                                apply_single_body,
                                "Self::Diff::{}(__{}) => self.{} = structdiff::collections::unordered_map_like::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                                field_name,
                                index,
                                field_name,
                                field_name,
                                index
                            );

                            l!(
                                diff_body,
                                "if let Some(list_diffs) = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), true) {{
                                    diffs.push(Self::Diff::{}(list_diffs));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );
                        },
                        crate::shared::MapStrategy::KeyAndValue => {
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name, field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection").join(","));

                            l!(
                                apply_single_body,
                                "Self::Diff::{}(__{}) => self.{} = structdiff::collections::unordered_map_like::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                                field_name,
                                index,
                                field_name,
                                field_name,
                                index
                            );

                            l!(
                                diff_body,
                                "if let Some(list_diffs) = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), false) {{
                                    diffs.push(Self::Diff::{}(list_diffs));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );
                        },

                    }
                }
            }
        });

    #[allow(unused)]
    let nanoserde_hack = String::new();
    #[cfg(feature = "nanoserde")]
    let nanoserde_hack = String::from("\nuse nanoserde::*;");

    let used_generics = {
        let mut added: HashSet<String> = HashSet::new();
        let mut ret = Vec::new();
        for used in struct_.generics.iter() {
            if added.insert(used.0.clone()) && used_generics.contains(&used.0) {
                ret.push(used.0.clone())
            }
        }
        ret
    };

    let all_impl_generics = {
        let mut do_not_bound = struct_generics.clone();
        let mut bound_strs = vec![];
        for generic_typename in used_generics.iter() {
            do_not_bound.remove_entry(&generic_typename);
            let mut bounds = format!(
                "{}: core::clone::Clone + core::cmp::PartialEq",
                generic_typename
            );
            if let Some(&extra_bounds) = struct_generics.get(&generic_typename) {
                for x in extra_bounds {
                    if !x.contains("+") && !bounds.ends_with("+") {
                        bounds += " +";
                    }
                    bounds += " ";
                    bounds += x;
                }
            }
            bound_strs.push(bounds)
        }

        for (typename, extra_bounds) in do_not_bound.into_iter() {
            let bounds = format!(
                "{}: {}",
                typename,
                extra_bounds
                    .into_iter()
                    .filter(|x| !x.contains('+'))
                    .map(|x| x.clone())
                    .collect::<Vec<String>>()
                    .join(" + ")
            );
            bound_strs.push(bounds)
        }

        match bound_strs {
            list if list.is_empty() => String::from(""),
            list => format!("<{}>", list.join(", ")),
        }
    };

    let used_generics_string = match used_generics
        .into_iter()
        .collect::<Vec<String>>()
        .join(", ")
    {
        list if list.is_empty() => String::from(""),
        list => format!("<{}>", list),
    };

    let struct_generics = match struct_
        .generics
        .iter()
        .map(|x| x.0.clone())
        .collect::<Vec<_>>()
        .join(", ")
    {
        list if list.is_empty() => String::from(""),
        list => format!("<{}>", list),
    };

    format!(
        "const _: () = {{
        use structdiff::collections::*;
        {type_aliases}
        {nanoserde_hack}
        
        /// Generated type from StructDiff
        #[derive({derives})]
        pub enum {enum_name}{enum_impl_generics} {{
            {enum_body}
        }}
        
        impl{impl_generics} StructDiff for {struct_name}{struct_generics_first} {{
            type Diff = {enum_name}{struct_generics};

            fn diff(&self, updated: &Self) -> Vec<Self::Diff> {{
                let mut diffs = vec![];
                {diff_body}
                diffs
            }}

            #[inline(always)]
            fn apply_single(&mut self, diff: Self::Diff) {{
                match diff {{
                    {apply_single_body}
                }}
            }}
        }}
        }};",
        type_aliases = type_aliases,
        nanoserde_hack = nanoserde_hack,
        derives = derives,
        struct_name = struct_.name,
        diff_body = diff_body,
        enum_name = enum_name,
        enum_body = diff_enum_body,
        apply_single_body = apply_single_body,
        enum_impl_generics = all_impl_generics.clone(),
        impl_generics = all_impl_generics,
        struct_generics_first = struct_generics.clone(),
        struct_generics = used_generics_string,
    )
    .parse()
    .unwrap()
}
