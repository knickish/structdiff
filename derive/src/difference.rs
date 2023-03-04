use std::collections::HashSet;

use alloc::format;
use alloc::string::String;

use crate::parse::{Category, ConstValType, Generic, Struct, Type};
use crate::shared::{attrs_collection_type, attrs_recurse, attrs_skip};

use proc_macro::TokenStream;

fn get_used_lifetimes(ty: &Type) -> Vec<String> {
    let mut ret = if let Some(Some(lt)) = &ty.ref_type {
        vec![lt.ident.clone()]
    } else {
        vec![]
    };
    if let Some(wraps) = &ty.wraps {
        for wrapped in wraps {
            ret.extend(get_used_lifetimes(wrapped))
        }
    }

    ret
}

fn get_array_lens(ty: &Type) -> Vec<String> {
    let mut ret = if let Category::Array {
        len: Some(ConstValType::Named(val)),
        ..
    } = &ty.ident
    {
        vec![val.full()]
    } else {
        vec![]
    };
    if let Some(wraps) = &ty.wraps {
        for wrapped in wraps {
            ret.extend(get_array_lens(wrapped))
        }
    }

    ret
}

const BOUNDS: &[&str] = &[
    "core::clone::Clone",
    "core::cmp::PartialEq",
    #[cfg(feature = "debug_diffs")]
    "core::fmt::Debug",
    #[cfg(feature = "nanoserde")]
    "nanoserde::DeBin",
    #[cfg(feature = "nanoserde")]
    "nanoserde::SerBin",
    #[cfg(feature = "serde")]
    "serde::Serialize",
    #[cfg(feature = "serde")]
    "serde::de::DeserializeOwned",
];

pub(crate) fn derive_struct_diff_struct(struct_: &Struct) -> TokenStream {
    let derives: String = vec![
        #[cfg(feature = "debug_diffs")]
        "core::fmt::Debug",
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
    let mut used_generics: Vec<&Generic> = Vec::new();

    let enum_name = String::from("__".to_owned() + struct_.name.as_str() + "StructDiffEnum");
    let struct_generics_names_hash: HashSet<String> =
        struct_.generics.iter().map(|x| x.full()).collect();

    struct_
        .fields
        .iter()
        .filter(|x| !attrs_skip(&x.attributes))
        .enumerate()
        .for_each(|(index, field)| {
            let field_name = field.field_name.as_ref().unwrap();
            used_generics.extend(struct_.generics.iter().filter(|x| x.full() == field.ty.ident.path(&field.ty, false)));
            

            let to_add = struct_.generics.iter().filter(|x| field.ty.wraps().iter().find(|&wrapped_type| &x.full() == wrapped_type ).is_some());
            used_generics.extend(to_add);
            
            used_generics.extend(get_used_lifetimes(&field.ty).into_iter().filter_map(|x| match struct_generics_names_hash.contains(&x) {
                true => Some(struct_.generics.iter().find(|generic| generic.full() == x ).unwrap()),
                false => None,
            }));

            for val in get_array_lens(&field.ty) {
                if let Some(const_gen) = struct_.generics.iter().find(|x| x.full() == val) {
                    used_generics.push(const_gen)
                }
            }

            match (attrs_recurse(&field.attributes), attrs_collection_type(&field.attributes), field.ty.base() == "Option") {

                (false, None, false) => {  // The default case
                    l!(diff_enum_body, " {}({}),", field_name, field.ty.full());

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

                    l!(diff_enum_body, " {}({}),", field_name, field.ty.full());

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
                    l!(type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.full());

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
                    l!(type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full());

                    l!(diff_enum_body, " {}(Option<{}>),", field_name, typename);
                    l!(diff_enum_body, " {}_full({}),", field_name, field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full());

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
                (true, Some(strat), false) => match strat {
                    crate::shared::CollectionStrategy::UnorderedArrayLikeHash => { panic!("Recursion inside of array-like collections is not yet supported"); },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(crate::shared::MapStrategy::KeyAndValue) => {
                        let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full().clone()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiff<{}>),", field_name, generic_names);

                        l!(
                            apply_single_body,
                            "Self::Diff::{}(__{}) => self.{} = structdiff::collections::unordered_map_like_recursive::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                            field_name,
                            index,
                            field_name,
                            field_name,
                            index
                        );

                        l!(
                            diff_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), false) {{
                                diffs.push(Self::Diff::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                    },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(crate::shared::MapStrategy::KeyOnly) => {
                        let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiff<{}>),", field_name, generic_names);

                        l!(
                            apply_single_body,
                            "Self::Diff::{}(__{}) => self.{} = structdiff::collections::unordered_map_like_recursive::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                            field_name,
                            index,
                            field_name,
                            field_name,
                            index
                        );

                        l!(
                            diff_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), true) {{
                                diffs.push(Self::Diff::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                    }
                },
                (false, Some(_), true) => panic!("Collection strategies inside of options are not yet supported"),
                (false, Some(strat), false) => match strat {
                    crate::shared::CollectionStrategy::UnorderedArrayLikeHash => {
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_array_like::UnorderedArrayLikeDiff<{}>),", field_name, field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection")[0].full());

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
                            let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name, generic_names);

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
                            let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name, generic_names);

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
                },
                #[allow(unreachable_patterns)]
                _ => panic!("this combination of options is not yet supported, please file an issue")
            }
        });

    #[allow(unused)]
    let nanoserde_hack = String::new();
    #[cfg(feature = "nanoserde")]
    let nanoserde_hack = String::from("\nuse nanoserde::*;");

    let used_generics = {
        let mut added: HashSet<String> = HashSet::new();
        let mut ret = Vec::new();
        for maybe_used in struct_.generics.iter() {
            if added.insert(maybe_used.full()) && used_generics.contains(&maybe_used) {
                ret.push(maybe_used.clone())
            }
        }

        ret
    };

    #[inline(always)]
    fn get_used_generic_bounds() -> &'static [&'static str] {
        BOUNDS
    }

    #[cfg(feature = "serde")]
    let serde_bound = {
        let start = "\n#[serde(bound = \"";
        let mid = used_generics
            .iter()
            .filter(|gen| !matches!(gen, Generic::Lifetime { .. } | Generic::ConstGeneric { .. }))
            .map(|x| {
                format!(
                    "{}: serde::Serialize + serde::de::DeserializeOwned",
                    x.ident_only()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let end = "\")]";
        vec![start, &mid, end].join("")
    };
    #[cfg(not(feature = "serde"))]
    let serde_bound = "";

    format!(
        "const _: () = {{
            use structdiff::collections::*;
            {type_aliases}
            {nanoserde_hack}
            
            /// Generated type from StructDiff
            #[derive({derives})]{serde_bounds}
            pub enum {enum_name}{enum_def_generics} 
            where
            {enum_where_bounds}
            {{
                {enum_body}
            }}
            
            impl{impl_generics} StructDiff for {struct_name}{struct_generics} 
            where 
            {struct_where_bounds}
            {{
                type Diff = {enum_name}{enum_impl_generics};

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
        enum_def_generics = format!(
            "<{}>",
            used_generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_with_const(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        enum_where_bounds = format!(
            "{}",
            used_generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        impl_generics = format!(
            "<{}>",
            struct_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_with_const(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        struct_generics = format!(
            "<{}>",
            struct_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_only(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        struct_where_bounds = format!(
            "{}",
            struct_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::ConstGeneric { .. } | Generic::WhereBounded { .. }))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), true))
                .collect::<Vec<_>>().into_iter().chain(struct_
                    .generics
                    .iter()
                    .filter(|gen| matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::full_with_const(gen, &[], true)).collect::<Vec<_>>().into_iter()).collect::<Vec<_>>()
                .join(",\n")
        ),
        enum_impl_generics = format!(
            "<{}>",
            used_generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_only(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_bounds = serde_bound
    )
    .parse()
    .unwrap()
}
