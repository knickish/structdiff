use std::collections::HashSet;

use alloc::format;
use alloc::string::String;

use crate::parse::{Category, ConstValType, Enum, Generic, Struct, Type};
#[cfg(feature = "generated_setters")]
use crate::shared::{attrs_all_setters, attrs_setter};
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

const REF_BOUNDS: &[&str] = &[
    "core::clone::Clone",
    "core::cmp::PartialEq",
    #[cfg(feature = "debug_diffs")]
    "core::fmt::Debug",
    #[cfg(feature = "nanoserde")]
    "nanoserde::SerBin",
    #[cfg(feature = "serde")]
    "serde::Serialize",
];

pub(crate) fn derive_struct_diff_struct(struct_: &Struct) -> TokenStream {
    let owned_derives: String = vec![
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

    let ref_derives: String = vec![
        #[cfg(feature = "debug_diffs")]
        "core::fmt::Debug",
        "Clone",
        #[cfg(feature = "nanoserde")]
        "nanoserde::SerBin",
        #[cfg(feature = "serde")]
        "serde::Serialize",
    ]
    .join(", ");

    let mut diff_enum_body = String::new();
    let mut diff_body = String::new();
    let mut diff_ref_enum_body = String::new();
    let mut diff_ref_body = String::new();
    let mut apply_single_body = String::new();
    let mut owned_type_aliases = String::new();
    let mut ref_type_aliases = String::new();
    let mut used_generics: Vec<&Generic> = Vec::new();
    let mut ref_into_owned_body = String::new();
    #[cfg(feature = "generated_setters")]
    let mut setters_body = String::new();

    let enum_name =
        String::from("__".to_owned() + struct_.name.as_ref().unwrap().as_str() + "StructDiffEnum");
    let struct_generics_names_hash: HashSet<String> =
        struct_.generics.iter().map(|x| x.full()).collect();

    #[cfg(feature = "generated_setters")]
    let all_setters = attrs_all_setters(&struct_.attributes);

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
                    l!(diff_ref_enum_body, " {}(&'__diff_target {}),", field_name, field.ty.full());

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

                    l!(
                        diff_ref_body,
                        "if self.{} != updated.{} {{diffs.push(Self::DiffRef::{}(&updated.{}))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );

                    l!(
                        ref_into_owned_body,
                        "\t {}Ref::{}(v) => {}::{}(v.clone()),",
                        enum_name,
                        field_name,
                        enum_name,
                        field_name
                    );


                    #[cfg(feature = "generated_setters")]
                    match (all_setters, attrs_setter(&field.attributes)) {
                        (_, (_, true, _)) => (),
                        (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                            l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(value.clone());", field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        (true, (_, false, None)) | (false, (true, false, None)) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                            l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(value.clone());", field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        _ => ()
                    };
                },
                (false, None, true) => {  // The default case, but with an option

                    l!(diff_enum_body, " {}({}),", field_name, field.ty.full());
                    l!(diff_ref_enum_body, " {}(&'__diff_target {}),", field_name, field.ty.full());

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

                    l!(
                        diff_ref_body,
                        "if self.{} != updated.{} {{diffs.push(Self::DiffRef::{}(&updated.{}))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );

                    l!(
                        ref_into_owned_body,
                        "\t {}Ref::{}(v) => {}::{}(v.clone()),",
                        enum_name,
                        field_name,
                        enum_name,
                        field_name
                    );

                    #[cfg(feature = "generated_setters")]
                    match (all_setters, attrs_setter(&field.attributes)) {
                        (_, (_, true, _)) => (),
                        (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                            l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(value.clone());", field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        (true, (_, false, None)) | (false, (true, false, None)) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                            l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(value.clone());", field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        _ => ()
                    };
                },
                (true, None, false)  => { // Recurse inwards and generate a Vec<SubStructDiff> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(owned_type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.full());
                    let typename_ref = format!("__{}StructDiffRefVec<'__diff_target>", field_name);
                    l!(ref_type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::DiffRef<'__diff_target>>;", typename_ref, field.ty.full());

                    l!(diff_enum_body, " {}({}),", field_name, typename);
                    l!(diff_ref_enum_body, " {}({}),", field_name, typename_ref);

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

                    l!(
                        diff_ref_body,
                        "if &self.{} != &updated.{} {{diffs.push(Self::DiffRef::{}(self.{}.diff_ref(&updated.{})))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );

                    l!(
                        ref_into_owned_body,
                        "\t {}Ref::{}(v) => {}::{}(v.into_iter().map(Into::into).collect()),",
                        enum_name,
                        field_name,
                        enum_name,
                        field_name
                    );

                    #[cfg(feature = "generated_setters")]
                    match (all_setters, attrs_setter(&field.attributes)) {
                        (_, (_, true, _)) => (),
                        (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                            l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(self.{}.diff(&value));", field_name, field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        (true, (_, false, None)) | (false, (true, false, None)) => {
                            l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                            l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                            l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                            l!(setters_body, "\n\tlet diff = <Self as StructDiff>::Diff::{}(self.{}.diff(&value));", field_name, field_name);
                            l!(setters_body, "\n\tself.{} = value;", field_name);
                            l!(setters_body, "\n\treturn Some(diff)");
                            l!(setters_body, "\n}");
                        },
                        _ => ()
                    };
                },
                (true, None, true)  => { // Recurse inwards and generate an Option<Vec<SubStructDiff>> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(owned_type_aliases, "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", 
                        typename,
                        field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full()
                    );

                    let ref_typename = format!("__{}StructDiffRefVec<'__diff_target>", field_name);
                    l!(
                        ref_type_aliases,
                        "///Generated aliases from StructDiff\n type {} = Vec<<{} as StructDiff>::DiffRef<'__diff_target>>;", 
                        ref_typename,
                        field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full()
                    );

                    l!(diff_enum_body, " {}(Option<{}>),", field_name, typename);
                    l!(diff_enum_body, " {}_full({}),", field_name, field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full());

                    l!(diff_ref_enum_body, " {}(Option<{}>),", field_name, ref_typename);
                    l!(diff_ref_enum_body, " {}_full(&'__diff_target {}),", field_name, field.ty.wraps.as_ref().expect("Option must wrap a type").get(0).expect("Option must wrap a type").full());

                    let apply_single_body_partial = format!(
                        "Self::Diff::{}(Some(__{})) => if let Some(ref mut inner) = self.{} {{ 
                            inner.apply_mut(__{});
                        }},",
                        field_name,
                        index,
                        field_name,
                        index
                    );


                    let apply_single_body_full = format!(
                        "Self::Diff::{}_full(__{}) => self.{} = Some(__{}),",
                        field_name,
                        index,
                        field_name,
                        index
                    );

                    let apply_single_body_none = format!(
                        "Self::Diff::{}(None) => self.{} = None,",
                        field_name,
                        field_name
                    );

                    let diff_body_fragment = format!(
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

                    let diff_body_fragment_ref = format!(
                        "match (&self.{}, &updated.{}) {{
                            (Some(val1), Some(val2)) if &val1 != &val2 => diffs.push(Self::DiffRef::{}(Some(val1.diff_ref(&val2)))),
                            (Some(val1), None) => diffs.push(Self::DiffRef::{}(None)),
                            (None, Some(val2)) => diffs.push(Self::DiffRef::{}_full(&val2)),
                            _ => (),
                        }};",
                        field_name,
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );

                    #[cfg(feature = "generated_setters")]
                    {
                        let diff_body_fragment_setter = format!(
                            "match (&self.{}, &value) {{
                                (Some(val1), Some(val2)) if &val1 != &val2 => <Self as StructDiff>::Diff::{}(Some(val1.diff(&val2))),
                                (Some(val1), None) => <Self as StructDiff>::Diff::{}(None),
                                (None, Some(val2)) => <Self as StructDiff>::Diff::{}_full(val2.clone()),
                                _ => return None,
                            }};",
                            field_name,
                            field_name,
                            field_name,
                            field_name
                        );
                        match (all_setters, attrs_setter(&field.attributes)) {
                            (_, (_, true, _)) => (),

                            (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                l!(setters_body, "\n\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                                l!(setters_body, "\n\tlet diff = {}", diff_body_fragment_setter);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\treturn Some(diff)");
                                l!(setters_body, "\n}");
                            },
                            (true, (_, false, None)) | (false, (true, false, None)) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                l!(setters_body, "\n\tif self.{} == value {{return None}};", field_name);
                                l!(setters_body, "\n\tlet diff = {}", diff_body_fragment_setter);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\treturn Some(diff)");
                                l!(setters_body, "\n}");
                            },
                            _ => ()
                        };
                    }

                    l!(
                        ref_into_owned_body,
                        "\t {}Ref::{}(v) => {}::{}(v.map(|vals| vals.into_iter().map(Into::into).collect())),",
                        enum_name,
                        field_name,
                        enum_name,
                        field_name
                    );

                    l!(
                        ref_into_owned_body,
                        "\t {}Ref::{}_full(v) => {}::{}_full(v.clone()),",
                        enum_name,
                        field_name,
                        enum_name,
                        field_name
                    );

                    l!(apply_single_body, "{}", apply_single_body_partial);
                    l!(apply_single_body, "{}", apply_single_body_full);
                    l!(apply_single_body, "{}", apply_single_body_none);
                    l!(diff_body, "{}", diff_body_fragment);

                    l!(diff_ref_body, "{}", diff_body_fragment_ref);
                },
                (true, Some(strat), false) => match strat {
                    crate::shared::CollectionStrategy::OrderedArrayLike => { panic!("Recursion inside of array-like collections is not yet supported"); },
                    crate::shared::CollectionStrategy::UnorderedArrayLikeHash => { panic!("Recursion inside of array-like collections is not yet supported"); },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(crate::shared::MapStrategy::KeyAndValue) => {
                        let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full().clone()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiffOwned<{}>),", field_name, generic_names);
                        l!(diff_ref_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiffRef<'__diff_target, {}>),", field_name, generic_names);

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
                                diffs.push(Self::Diff::{}(list_diffs.into()));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            diff_ref_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), false) {{
                                diffs.push(Self::DiffRef::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            ref_into_owned_body,
                            "\t {}Ref::{}(v) => {}::{}(v.into()),",
                            enum_name,
                            field_name,
                            enum_name,
                            field_name
                        );
                    },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(crate::shared::MapStrategy::KeyOnly) => {
                        let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiffOwned<{}>),", field_name, generic_names);
                        l!(diff_ref_enum_body, " {}(structdiff::collections::unordered_map_like_recursive::UnorderedMapLikeRecursiveDiffRef<'__diff_target, {}>),", field_name, generic_names);

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
                                diffs.push(Self::Diff::{}(list_diffs.into()));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            diff_ref_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), true) {{
                                diffs.push(Self::DiffRef::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            ref_into_owned_body,
                            "\t {}Ref::{}(v) => {}::{}(v.into()),",
                            enum_name,
                            field_name,
                            enum_name,
                            field_name
                        );

                        #[cfg(feature = "generated_setters")]
                        match (all_setters, attrs_setter(&field.attributes)) {
                            (_, (_, true, _)) => (),
                            (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), value.iter(), true);", field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            (true, (_, false, None)) | (false, (true, false, None)) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like_recursive::unordered_hashcmp(self.{}.iter(), value.iter(), true);", field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            _ => ()
                        };

                    }


                },
                (false, Some(_), true) => panic!("Collection strategies inside of options are not yet supported"),
                (false, Some(strat), false) => match strat {
                    crate::shared::CollectionStrategy::OrderedArrayLike => {
                        let generic_ref_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(", &'__diff_target ");
                        l!(diff_enum_body, " {}(structdiff::collections::ordered_array_like::OrderedArrayLikeDiffOwned<{}>),", field_name, field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection")[0].full());
                        l!(diff_ref_enum_body, " {}(structdiff::collections::ordered_array_like::OrderedArrayLikeDiffRef<'__diff_target, {}>),", field_name, generic_ref_names);

                        l!(
                            apply_single_body,
                            "Self::Diff::{}(__{}) => self.{} = structdiff::collections::ordered_array_like::apply(__{}, std::mem::take(&mut self.{})).collect(),",
                            field_name,
                            index,
                            field_name,
                            index,
                            field_name
                        );

                        l!(
                            diff_body,
                            "if let Some(list_diffs) = structdiff::collections::ordered_array_like::hirschberg(&updated.{}, &self.{}) {{
                                diffs.push(Self::Diff::{}(list_diffs.into()));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            diff_ref_body,
                            "if let Some(list_diffs) = structdiff::collections::ordered_array_like::hirschberg(&updated.{}, &self.{}) {{
                                diffs.push(Self::DiffRef::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            ref_into_owned_body,
                            "\t {}Ref::{}(v) => {}::{}(v.into()),",
                            enum_name,
                            field_name,
                            enum_name,
                            field_name
                        );

                        #[cfg(feature = "generated_setters")]
                        match (all_setters, attrs_setter(&field.attributes)) {
                            (_, (_, true, _)) => (),
                            (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::ordered_array_like::hirschberg(&value, &self.{}).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            (true, (_, false, None)) | (false, (true, false, None)) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::ordered_array_like::hirschberg(&value, &self.{}).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            _ => ()
                        };
                    }
                    crate::shared::CollectionStrategy::UnorderedArrayLikeHash => {
                        let generic_ref_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(", &'__diff_target ");
                        l!(diff_enum_body, " {}(structdiff::collections::unordered_array_like::UnorderedArrayLikeDiff<{}>),", field_name, field.ty.wraps.as_ref().expect("Using collection strategy on a non-collection")[0].full());
                        l!(diff_ref_enum_body, " {}(structdiff::collections::unordered_array_like::UnorderedArrayLikeDiff<&'__diff_target {}>),", field_name, generic_ref_names);

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
                                diffs.push(Self::Diff::{}(list_diffs.into()));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            diff_ref_body,
                            "if let Some(list_diffs) = structdiff::collections::unordered_array_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter()) {{
                                diffs.push(Self::DiffRef::{}(list_diffs));
                            }};"
                            ,
                            field_name,
                            field_name,
                            field_name
                        );

                        l!(
                            ref_into_owned_body,
                            "\t {}Ref::{}(v) => {}::{}(v.into()),",
                            enum_name,
                            field_name,
                            enum_name,
                            field_name
                        );

                        #[cfg(feature = "generated_setters")]
                        match (all_setters, attrs_setter(&field.attributes)) {
                            (_, (_, true, _)) => (),
                            (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_array_like::unordered_hashcmp(self.{}.iter(), value.iter()).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            (true, (_, false, None)) | (false, (true, false, None)) => {
                                l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_array_like::unordered_hashcmp(self.{}.iter(), value.iter()).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                l!(setters_body, "\n\tself.{} = value;", field_name);
                                l!(setters_body, "\n\tret");
                                l!(setters_body, "\n}");
                            },
                            _ => ()
                        };
                    },
                    crate::shared::CollectionStrategy::UnorderedMapLikeHash(map_strat) => match map_strat {
                        crate::shared::MapStrategy::KeyOnly => {
                            let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                            let generic_ref_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(", &'__diff_target ");
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name, generic_names);
                            l!(diff_ref_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<&'__diff_target {}>),", field_name,generic_ref_names);

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
                                    diffs.push(Self::Diff::{}(list_diffs.into()));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );

                            l!(
                                diff_ref_body,
                                "if let Some(list_diffs) = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), true) {{
                                    diffs.push(Self::DiffRef::{}(list_diffs));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );

                            l!(
                                ref_into_owned_body,
                                "\t {}Ref::{}(v) => {}::{}(v.into()),",
                                enum_name,
                                field_name,
                                enum_name,
                                field_name
                            );

                            #[cfg(feature = "generated_setters")]
                            match (all_setters, attrs_setter(&field.attributes)) {
                                (_, (_, true, _)) => (),
                                (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                    l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                    l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                    l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), value.iter(), true).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                    l!(setters_body, "\n\tself.{} = value;", field_name);
                                    l!(setters_body, "\n\tret");
                                    l!(setters_body, "\n}");
                                },
                                (true, (_, false, None)) | (false, (true, false, None)) => {
                                    l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                    l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                    l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), value.iter(), true).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                    l!(setters_body, "\n\tself.{} = value;", field_name);
                                    l!(setters_body, "\n\tret");
                                    l!(setters_body, "\n}");
                                },
                                _ => ()
                            };
                        },
                        crate::shared::MapStrategy::KeyAndValue => {
                            let generic_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(",");
                            let generic_ref_names = field.ty.wraps.as_ref().map(|x| x.iter().map(|y| y.full()).collect::<Vec<_>>()).expect("Missing types for map creation").join(", &'__diff_target ");
                            l!(diff_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<{}>),", field_name, generic_names);
                            l!(diff_ref_enum_body, " {}(structdiff::collections::unordered_map_like::UnorderedMapLikeDiff<&'__diff_target {}>),", field_name,generic_ref_names);

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
                                    diffs.push(Self::Diff::{}(list_diffs.into()));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );

                            l!(
                                diff_ref_body,
                                "if let Some(list_diffs) = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), updated.{}.iter(), false) {{
                                    diffs.push(Self::DiffRef::{}(list_diffs));
                                }};"
                                ,
                                field_name,
                                field_name,
                                field_name
                            );

                            l!(
                                ref_into_owned_body,
                                "\t {}Ref::{}(v) => {}::{}(v.into()),",
                                enum_name,
                                field_name,
                                enum_name,
                                field_name
                            );

                            #[cfg(feature = "generated_setters")]
                            match (all_setters, attrs_setter(&field.attributes)) {
                                (_, (_, true, _)) => (),
                                (true, (_, false, Some(name_override))) | (false, (true, false, Some(name_override))) => {
                                    l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", name_override);
                                    l!(setters_body, "\npub fn {}(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", name_override, field.ty.full());
                                    l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), value.iter(), false).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                    l!(setters_body, "\n\tself.{} = value;", field_name);
                                    l!(setters_body, "\n\tret");
                                    l!(setters_body, "\n}");
                                },
                                (true, (_, false, None)) | (false, (true, false, None)) => {
                                    l!(setters_body, "\n/// Setter generated by StructDiff. Use to set the {} field and generate a diff if necessary", field_name);
                                    l!(setters_body, "\npub fn set_{}_with_diff(&mut self, value: {}) -> Option<<Self as StructDiff>::Diff> {{", field_name, field.ty.full());
                                    l!(setters_body, "\n\tlet ret = structdiff::collections::unordered_map_like::unordered_hashcmp(self.{}.iter(), value.iter(), false).map(|x| <Self as StructDiff>::Diff::{}(x.into()));", field_name, field_name);
                                    l!(setters_body, "\n\tself.{} = value;", field_name);
                                    l!(setters_body, "\n\tret");
                                    l!(setters_body, "\n}");
                                },
                                _ => ()
                            };
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

    #[inline]
    fn get_used_generic_bounds() -> &'static [&'static str] {
        BOUNDS
    }

    #[inline]
    fn get_used_generic_bounds_ref() -> &'static [&'static str] {
        REF_BOUNDS
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

    let setters = {
        #[cfg(feature = "generated_setters")]
        {
            if setters_body.is_empty() {
                String::new()
            } else {
                format!(
                    "impl<{}> {}<{}> where 
                    {}
                    {{
                        {}
                    }}
                    ",
                    struct_
                        .generics
                        .iter()
                        .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                        .map(|gen| Generic::ident_with_const(gen))
                        .collect::<Vec<_>>()
                        .join(", "),
                    struct_
                        .name
                        .clone()
                        .unwrap_or_else(|| String::from("Anonymous")),
                    struct_
                        .generics
                        .iter()
                        .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                        .map(|gen| Generic::ident_only(gen))
                        .collect::<Vec<_>>()
                        .join(", "),
                    struct_
                        .generics
                        .iter()
                        .filter(|gen| !matches!(
                            gen,
                            Generic::ConstGeneric { .. } | Generic::WhereBounded { .. }
                        ))
                        .map(|gen| Generic::full_with_const(
                            gen,
                            get_used_generic_bounds(),
                            &[],
                            true
                        ))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .chain(
                            struct_
                                .generics
                                .iter()
                                .filter(|gen| matches!(gen, Generic::WhereBounded { .. }))
                                .map(|gen| Generic::full_with_const(gen, &[], &[], true))
                                .collect::<Vec<_>>()
                                .into_iter()
                        )
                        .collect::<Vec<_>>()
                        .join(",\n"),
                    setters_body
                )
            }
        }

        #[cfg(not(feature = "generated_setters"))]
        ""
    };

    format!(
        "#[allow(non_camel_case_types)]
        const _: () = {{
            use structdiff::collections::*;
            {type_aliases}
            {ref_type_aliases}
            {nanoserde_hack}
            
            #[allow(non_camel_case_types)]
            /// Generated type from StructDiff
            #[derive({owned_derives})]{serde_bounds}
            pub enum {enum_name}{owned_enum_def_generics} 
            where
            {owned_enum_where_bounds}
            {{
                {enum_body}
            }}

            #[allow(non_camel_case_types)]
            /// Generated type from StructDiff
            #[derive({ref_derives})]
            pub enum {enum_name}Ref{ref_enum_def_generics} 
            where
            {ref_enum_where_bounds}
            {{
                {diff_ref_enum_body}
            }}

            impl{ref_enum_def_generics} Into<{enum_name}{owned_enum_impl_generics}> for {enum_name}Ref{ref_enum_impl_generics}
            where
            {into_impl_where_bounds}
            {{
                fn into(self) -> {enum_name}{owned_enum_impl_generics} {{
                    match self {{
                        {ref_into_owned_body}
                    }}
                }}
            }}
            
            impl{impl_generics} StructDiff for {struct_name}{struct_generics} 
            where 
            {struct_where_bounds}
            {{
                type Diff = {enum_name}{owned_enum_impl_generics};
                type DiffRef<'__diff_target> = {enum_name}Ref{ref_enum_impl_generics} where
                    {diff_ref_type_where_bounds};

                fn diff(&self, updated: &Self) -> Vec<Self::Diff> {{
                    let mut diffs = vec![];
                    {diff_body}
                    diffs
                }}

                fn diff_ref<'__diff_target>(&'__diff_target self, updated: &'__diff_target Self) -> Vec<Self::DiffRef<'__diff_target>> {{
                    let mut diffs = vec![];
                    {diff_ref_body}
                    diffs
                }}


                #[inline(always)]
                fn apply_single(&mut self, diff: Self::Diff) {{
                    match diff {{
                        {apply_single_body}
                    }}
                }}
            }}

            {setters}
        }};",
        type_aliases = owned_type_aliases,
        ref_type_aliases = ref_type_aliases,
        nanoserde_hack = nanoserde_hack,
        owned_derives = owned_derives,
        ref_derives = ref_derives,
        struct_name = struct_.name.as_ref().unwrap(),
        diff_body = diff_body,
        diff_ref_body = diff_ref_body,
        enum_name = enum_name,
        enum_body = diff_enum_body,
        diff_ref_enum_body = diff_ref_enum_body,
        ref_into_owned_body = ref_into_owned_body,
        apply_single_body = apply_single_body,
        owned_enum_def_generics = format!(
            "<{}>",
            used_generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_with_const(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ref_enum_def_generics = format!(
            "<{}>",
            std::iter::once(String::from("'__diff_target")).chain(
                used_generics
                    .iter()
                    .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::ident_with_const(gen)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        owned_enum_where_bounds = format!(
            "{}",
            used_generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, false, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), &[], true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        ref_enum_where_bounds =format!(
            "{}",
            used_generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds_ref(), &["\'__diff_target"], true))
                .chain(std::iter::once(String::from("Self: \'__diff_target")))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        into_impl_where_bounds = format!(
            "{}",
            used_generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), &["\'__diff_target"], true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        diff_ref_type_where_bounds = format!(
            "{}",
            struct_
            .generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, &[], &["\'__diff_target"], true))
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
                .filter(|g| Generic::has_where_bounds(g, false, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), &[], true))
                .collect::<Vec<_>>().into_iter().chain(struct_
                    .generics
                    .iter()
                    .filter(|gen| matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::full_with_const(gen, &[], &[], true)).collect::<Vec<_>>().into_iter()).collect::<Vec<_>>()
                .join(",\n")
        ),
        owned_enum_impl_generics = format!(
            "<{}>",
            used_generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_only(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ref_enum_impl_generics = format!(
            "<{}>",
            std::iter::once(String::from("'__diff_target")).chain(
                used_generics
                    .iter()
                    .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::ident_only(gen)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_bounds = serde_bound
    )
    .parse()
    .unwrap()
}

pub(crate) fn derive_struct_diff_enum(enum_: &Enum) -> TokenStream {
    let owned_derives: String = vec![
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

    let ref_derives: String = vec![
        #[cfg(feature = "debug_diffs")]
        "core::fmt::Debug",
        "Clone",
        #[cfg(feature = "nanoserde")]
        "nanoserde::SerBin",
        #[cfg(feature = "serde")]
        "serde::Serialize",
    ]
    .join(", ");

    let mut replace_enum_body = String::new();
    #[cfg(unused)]
    let mut diff_enum_body = String::new();
    let mut diff_body = String::new();
    let mut diff_body_ref = String::new();
    let mut apply_single_body = String::new();
    #[allow(unused_mut)]
    let mut type_aliases = String::new();
    let mut used_generics: Vec<&Generic> = Vec::new();

    let enum_name = String::from("__".to_owned() + enum_.name.as_str() + "StructDiffEnum");
    let ref_into_owned_body = format!(
        "Self::Replace(variant) => {}::Replace(variant.clone()),",
        &enum_name
    );
    let struct_generics_names_hash: HashSet<String> =
        enum_.generics.iter().map(|x| x.full()).collect();

    if enum_.variants.iter().any(|x| attrs_skip(&x.attributes)) {
        panic!("Enum variants may not be skipped");
    };

    enum_.variants.iter().enumerate().for_each(|(_, field)| {
        let field_name = field.field_name.as_ref().unwrap();
        let ty = &field.ty;
        used_generics.extend(
            enum_
                .generics
                .iter()
                .filter(|x| x.full() == ty.ident.path(&ty, false)),
        );

        let to_add = enum_.generics.iter().filter(|x| {
            ty.wraps()
                .iter()
                .find(|&wrapped_type| &x.full() == wrapped_type)
                .is_some()
        });
        used_generics.extend(to_add);

        used_generics.extend(get_used_lifetimes(&ty).into_iter().filter_map(|x| {
            match struct_generics_names_hash.contains(&x) {
                true => Some(
                    enum_
                        .generics
                        .iter()
                        .find(|generic| generic.full() == x)
                        .unwrap(),
                ),
                false => None,
            }
        }));

        for val in get_array_lens(&ty) {
            if let Some(const_gen) = enum_.generics.iter().find(|x| x.full() == val) {
                used_generics.push(const_gen)
            }
        }
        if !matches!(ty.ident, Category::None) {
            match (
                attrs_recurse(&field.attributes),
                attrs_collection_type(&field.attributes),
                ty.base() == "Option",
            ) {
                (false, None, false) => {
                    // The default case
                    l!(replace_enum_body, " {}({}),", field_name, ty.full());

                    if matches!(ty.ident, Category::AnonymousStruct { .. }) {
                        l!(
                            apply_single_body,
                            "variant @ Self::{}{{..}} => *self = variant,",
                            field_name
                        );

                        l!(
                            diff_body,
                            "variant @ Self::{}{{..}} => Self::Diff::Replace(variant),",
                            field_name
                        );

                        l!(
                            diff_body_ref,
                            "variant @ Self::{}{{..}} => Self::DiffRef::Replace(&variant),",
                            field_name
                        );
                    } else {
                        l!(
                            apply_single_body,
                            "variant @ Self::{}(..) => *self = variant,",
                            field_name
                        );

                        l!(
                            diff_body,
                            "variant @ Self::{}(..) => Self::Diff::Replace(variant),",
                            field_name
                        );

                        l!(
                            diff_body_ref,
                            "\nvariant @ Self::{}{{..}} => Self::DiffRef::Replace(&variant),",
                            field_name
                        );
                    }
                }
                #[allow(unreachable_patterns)]
                _ => {
                    panic!("this combination of options is not yet supported, please file an issue")
                }
            }
        } else {
            //empty variant
            l!(replace_enum_body, " {},", field_name);

            l!(
                apply_single_body,
                "variant @ Self::{} => *self = variant,",
                field_name
            );

            l!(
                diff_body,
                "variant @ Self::{} => Self::Diff::Replace(variant),",
                field_name
            );
            l!(
                diff_body_ref,
                "variant @ Self::{} => Self::DiffRef::Replace(&variant),",
                field_name
            );
        };
    });

    #[allow(unused)]
    let nanoserde_hack = String::new();
    #[cfg(feature = "nanoserde")]
    let nanoserde_hack = String::from("\nuse nanoserde::*;");

    #[cfg(unused)]
    let used_generics = {
        let mut added: HashSet<String> = HashSet::new();
        let mut ret = Vec::new();
        for maybe_used in enum_.generics.iter() {
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

    #[inline]
    fn get_used_generic_bounds_ref() -> &'static [&'static str] {
        REF_BOUNDS
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
            #[derive({owned_derives})]{serde_bounds}
            #[allow(non_camel_case_types)]
            pub enum {enum_name}{owned_enum_def_generics} 
            where
            {enum_where_bounds}
            {{
                Replace({struct_name}{struct_generics})
            }}

            #[allow(non_camel_case_types)]
            /// Generated type from StructDiff
            #[derive({ref_derives})]
            pub enum {enum_name}Ref{ref_enum_def_generics} 
            where
            {ref_enum_where_bounds}
            {{
                Replace(&'__diff_target {struct_name}{struct_generics})
            }}

            impl{ref_enum_def_generics} Into<{enum_name}{enum_impl_generics}> for {enum_name}Ref{ref_enum_impl_generics}
            where
            {into_impl_where_bounds}
            {{
                fn into(self) -> {enum_name}{enum_impl_generics} {{
                    match self {{
                        {ref_into_owned_body}
                    }}
                }}
            }}
            
            impl{impl_generics} StructDiff for {struct_name}{struct_generics} 
            where 
            {struct_where_bounds}
            {{
                type Diff = {enum_name}{enum_impl_generics};
                type DiffRef<'__diff_target> = {enum_name}Ref{ref_enum_impl_generics} where
                    {diff_ref_type_where_bounds};

                fn diff(&self, updated: &Self) -> Vec<Self::Diff> {{
                    if self == updated {{
                        vec![]
                    }} else {{
                        vec![match updated.clone() {{
                            {diff_body}
                        }}]
                    }}
                }}

                fn diff_ref<'__diff_target>(&'__diff_target self, updated: &'__diff_target Self) -> Vec<Self::DiffRef<'__diff_target>> {{
                    if self == updated {{
                        vec![]
                    }} else {{
                        vec![match updated {{
                            {ref_diff_body}
                        }}]
                    }}
                }}

                #[inline(always)]
                fn apply_single(&mut self, diff: Self::Diff) {{
                    match diff {{
                        Self::Diff::Replace(diff) => match diff {{
                            {apply_single_body}
                        }}
                    }}
                }}
            }}
        }};",
        type_aliases = type_aliases,
        nanoserde_hack = nanoserde_hack,
        owned_derives = owned_derives,
        ref_derives = ref_derives,
        struct_name = enum_.name,
        diff_body = diff_body,
        ref_diff_body = diff_body_ref,
        ref_into_owned_body = ref_into_owned_body,
        enum_name = enum_name,
        apply_single_body = apply_single_body,
        owned_enum_def_generics = format!(
            "<{}>",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_with_const(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ref_enum_def_generics = format!(
            "<{}>",
            std::iter::once(String::from("'__diff_target")).chain(
                enum_
                .generics
                    .iter()
                    .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::ident_with_const(gen)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        enum_where_bounds = format!(
            "{}",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                     Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, false, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), &[], true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        ref_enum_where_bounds = format!(
            "{}",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds_ref(), &["\'__diff_target"], true))
                .chain(std::iter::once(String::from("Self: '__diff_target")))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        into_impl_where_bounds = format!(
            "{}",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds_ref(), &["\'__diff_target"], true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        diff_ref_type_where_bounds = format!(
            "{}",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(
                    gen,
                    Generic::WhereBounded { .. } | Generic::ConstGeneric { .. }
                ))
                .filter(|g| Generic::has_where_bounds(g, true, true))
                .map(|gen| Generic::full_with_const(gen, &[], &["\'__diff_target"], true))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
        impl_generics = format!(
            "<{}>",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_with_const(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        struct_generics = format!(
            "<{}>",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_only(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        struct_where_bounds = format!(
            "{}",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::ConstGeneric { .. } | Generic::WhereBounded { .. }))
                .map(|gen| Generic::full_with_const(gen, get_used_generic_bounds(), &[],true))
                .collect::<Vec<_>>().into_iter().chain(enum_
                    .generics
                    .iter()
                    .filter(|gen| matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::full_with_const(gen, &[], &[], true)).collect::<Vec<_>>().into_iter()).collect::<Vec<_>>()
                .join(",\n")
        ),
        enum_impl_generics = format!(
            "<{}>",
            enum_
                .generics
                .iter()
                .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                .map(|gen| Generic::ident_only(gen))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ref_enum_impl_generics = format!(
            "<{}>",
            std::iter::once(String::from("'__diff_target")).chain(
                enum_
                .generics
                    .iter()
                    .filter(|gen| !matches!(gen, Generic::WhereBounded { .. }))
                    .map(|gen| Generic::ident_only(gen)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_bounds = serde_bound
    )
    .parse()
    .unwrap()
}
