use alloc::format;
use alloc::string::String;

use crate::parse::Struct;
use crate::shared::{attrs_recurse, attrs_skip, attrs_collection};

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
    let mut use_collections = false;
    let enum_name = String::from("__".to_owned() + struct_.name.as_str() + "StructDiffEnum");

    struct_
        .fields
        .iter()
        .filter(|x| !attrs_skip(&x.attributes))
        .enumerate()
        .for_each(|(index, field)| {
            let field_name = field.field_name.as_ref().unwrap();

            match (attrs_recurse(&field.attributes), attrs_collection(&field.attributes)) {
                (true, None)  => { // Recurse inwards and generate a Vec<SubStructDiff> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(type_aliases, "///Generated aliases fron StructDiff\n type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.path);

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
                (false, None) => {
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
                (true, Some(_)) => panic!("Recursion inside of collections is not yet supported"),
                (false, Some(_)) => {
                    l!(diff_enum_body, " {}(Vec<structdiff::collections::UnorderedItemChange<{}>>),", field_name, field.ty.wraps.clone().expect("Using collection strategy on a non-collection"));

                    l!(
                        apply_single_body,
                        "Self::Diff::{}(__{}) => self.{} = structdiff::collections::apply_unordered_hashdiffs(std::mem::take(&mut self.{}).into_iter(), __{}).collect(),",
                        field_name,
                        index,
                        field_name,
                        field_name,
                        index
                    );

                    l!(
                        diff_body,
                        "match structdiff::collections::unordered_hashcmp(self.{}.iter(), updated.{}.iter()) {{
                            list_diffs if !list_diffs.is_empty() => diffs.push(Self::Diff::{}(list_diffs)),
                            _ => ()
                        }};"
                        ,
                        field_name,
                        field_name,
                        field_name
                    );
                    use_collections = true;
                },
                
            }
        });

    #[allow(unused)]
    let nanoserde_hack = String::new();
    #[cfg(feature = "nanoserde")]
    let nanoserde_hack = String::from("\nuse nanoserde::*;");

    format!(
        "{type_aliases}
        {nanoserde_hack}
        
        /// Generated type from StructDiff
        #[derive({derives})]
        pub enum {enum_name} {{
            {enum_body}
        }}
        
        impl StructDiff for {struct_name} {{
            type Diff = {enum_name};

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
        }}",
        type_aliases = type_aliases,
        nanoserde_hack = nanoserde_hack,
        derives = derives,
        struct_name = struct_.name,
        diff_body = diff_body,
        enum_name = enum_name,
        enum_body = diff_enum_body,
        apply_single_body = apply_single_body
    )
    .parse()
    .unwrap()
}
