use alloc::format;
use alloc::string::String;

use crate::parse::Struct;

use proc_macro::TokenStream;

pub(crate) fn derive_struct_diff_struct(struct_: &Struct) -> TokenStream {
    let mut diff_enum_body = String::new();
    let mut diff_body = String::new();
    let mut apply_single_body = String::new();
    let enum_name = String::from("__".to_owned() + struct_.name.as_str() + "StructDiffEnum");

    struct_
        .fields
        .iter()
        .filter(|x| {
            !x.attributes
                .iter()
                .any(|y| y.name == "difference".to_owned() && y.tokens.contains(&"skip".to_owned()))
        })
        .enumerate()
        .for_each(|(index, field)| {
            let field_name = field.field_name.as_ref().unwrap();
            l!(diff_enum_body, " {}({}),", field_name, field.ty.path);
            l!(
                diff_body,
                "if self.{} != prev.{} {{diffs.push(Self::Diff::{}(self.{}.clone()))}};",
                field_name,
                field_name,
                field_name,
                field_name
            );

            l!(
                apply_single_body,
                "Self::Diff::{}(__{}) => self.{} = __{},",
                field_name,
                index,
                field_name,
                index
            );
        });
    format!(
        "/// Generated type from StructDiff
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
        struct_name = struct_.name,
        diff_body = diff_body,
        enum_name = enum_name,
        enum_body = diff_enum_body,
        apply_single_body = apply_single_body
    )
    .parse()
    .unwrap()
}
