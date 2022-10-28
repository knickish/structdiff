use alloc::format;
use alloc::string::String;

use crate::parse::Struct;
use crate::shared::{attrs_skip, attrs_recurse};

use proc_macro::TokenStream;

pub(crate) fn derive_struct_diff_struct(struct_: &Struct) -> TokenStream {
    let derives: String = vec![
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
    let enum_name = String::from("__".to_owned() + struct_.name.as_str() + "StructDiffEnum");

    struct_
        .fields
        .iter()
        .filter(|x| !attrs_skip(&x.attributes))
        .enumerate()
        .for_each(|(index, field)| {
            let field_name = field.field_name.as_ref().unwrap();

            match attrs_recurse(&field.attributes) {
                true => { // Recurse inwards and generate a Vec<SubStructDiff> instead of cloning the entire thing
                    let typename = format!("__{}StructDiffVec", field_name);
                    l!(type_aliases, "type {} = Vec<<{} as StructDiff>::Diff>;", typename, field.ty.path);

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
                        "if &self.{} != &prev.{} {{diffs.push(Self::Diff::{}(self.{}.diff(&prev.{})))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                },
                false => {
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
                        "if self.{} != prev.{} {{diffs.push(Self::Diff::{}(self.{}.clone()))}};",
                        field_name,
                        field_name,
                        field_name,
                        field_name
                    );
                }
            }

            
            
        });

    #[allow(unused)]
    let nanoserde_hack = String::new();
    #[cfg(feature = "nanoserde")]
    let nanoserde_hack = String::from("\nuse nanoserde::*;");

    format!(
        "///Generated aliases fron StructDiff
        {type_aliases}
        
        /// Generated type from StructDiff
        {nanoserde_hack}
        #[derive({derives})]
        pub enum {enum_name} {{
            {enum_body}
        }}
        
        impl StructDiff for {struct_name} {{
            type Diff = {enum_name};

            fn diff(&self, prev: &Self) -> Vec<Self::Diff> {{
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
