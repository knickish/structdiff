extern crate alloc;
extern crate proc_macro;

#[macro_use]
mod shared;

mod difference;
#[cfg(feature = "syn")]
mod syn_backend;

#[cfg(not(feature = "syn"))]
use difference::derive_struct_diff_enum;

#[cfg(not(feature = "syn"))]
use crate::difference::derive_struct_diff_struct;

#[cfg_attr(feature = "syn", allow(dead_code))]
mod parse;

/// Derive macro generating an impl of the trait `StructDiff`
#[proc_macro_derive(Difference, attributes(difference))]
pub fn derive_struct_diff(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_struct_diff_impl(input)
}

#[cfg(feature = "syn")]
fn derive_struct_diff_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn_backend::derive_struct_diff(input)
}

#[cfg(not(feature = "syn"))]
fn derive_struct_diff_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse::parse_data(input);

    match &input {
        parse::Data::Struct(struct_) if struct_.named => derive_struct_diff_struct(struct_),
        parse::Data::Enum(enum_) => derive_struct_diff_enum(enum_),
        _ => unimplemented!("Only structs and enums are supported"),
    }
}
