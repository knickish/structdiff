extern crate alloc;
extern crate proc_macro;

#[macro_use]
mod shared;

mod difference;
use crate::difference::derive_struct_diff_struct;

mod parse;

/// Derive macro for StructDiff
#[proc_macro_derive(Difference, attributes(difference))]
pub fn derive_struct_diff(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse::parse_data(input);

    // ok we have an ident, hopefully it's a struct
    let ts = match &input {
        parse::Data::Struct(struct_) if struct_.named => derive_struct_diff_struct(struct_),
        _ => unimplemented!("Only structs are supported"),
    };

    ts
}
