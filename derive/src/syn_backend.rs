use proc_macro::TokenStream;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, Attribute, ConstParam, Data, DeriveInput, Expr, Field, Fields, GenericParam,
    LifetimeParam, Lit, Meta, PathArguments, Type, TypeParam, TypeParamBound, WherePredicate,
};

use crate::difference::{derive_struct_diff_enum, derive_struct_diff_struct};
use crate::parse::{
    Attribute as ParsedAttribute, Category, ConstValType, Data as ParsedData, Enum as ParsedEnum,
    Field as ParsedField, FnType, Generic as ParsedGeneric, Lifetime, Struct as ParsedStruct,
    Type as ParsedType, Visibility,
};

pub(crate) fn derive_struct_diff(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match lower_input(input) {
        ParsedData::Struct(struct_) if struct_.named => derive_struct_diff_struct(&struct_),
        ParsedData::Enum(enum_) => derive_struct_diff_enum(&enum_),
        _ => unimplemented!("Only structs and enums are supported"),
    }
}

fn lower_input(input: DeriveInput) -> ParsedData {
    let attributes = lower_attrs(&input.attrs);
    let generics = lower_generics(&input.generics);
    let name = input.ident.to_string();

    match input.data {
        Data::Struct(data) => ParsedData::Struct(ParsedStruct {
            name: Some(name),
            named: matches!(data.fields, Fields::Named(_)),
            fields: lower_fields(data.fields),
            attributes,
            generics,
        }),
        Data::Enum(data) => ParsedData::Enum(ParsedEnum {
            name,
            variants: data
                .variants
                .into_iter()
                .map(|variant| {
                    let ty = match variant.fields {
                        Fields::Unit => empty_type(),
                        Fields::Unnamed(fields) => tuple_type(
                            fields
                                .unnamed
                                .into_iter()
                                .map(|field| lower_type(&field.ty))
                                .collect(),
                        ),
                        Fields::Named(fields) => {
                            let fields =
                                fields.named.into_iter().map(lower_field).collect::<Vec<_>>();
                            let contents = ParsedStruct {
                                name: Some(variant.ident.to_string()),
                                named: true,
                                fields: fields.clone(),
                                attributes: Vec::new(),
                                generics: Vec::new(),
                            };
                            ParsedType {
                                wraps: Some(fields.into_iter().map(|field| field.ty).collect()),
                                ident: Category::AnonymousStruct { contents },
                                ref_type: None,
                                as_other: None,
                            }
                        }
                    };

                    ParsedField {
                        attributes: lower_attrs(&variant.attrs),
                        vis: Visibility::Public,
                        field_name: Some(variant.ident.to_string()),
                        ty,
                    }
                })
                .collect(),
            attributes,
            generics,
        }),
        Data::Union(_) => ParsedData::Union(()),
    }
}

fn lower_fields(fields: Fields) -> Vec<ParsedField> {
    match fields {
        Fields::Named(fields) => fields.named.into_iter().map(lower_field).collect(),
        Fields::Unnamed(fields) => fields.unnamed.into_iter().map(lower_field).collect(),
        Fields::Unit => Vec::new(),
    }
}

fn lower_field(field: Field) -> ParsedField {
    ParsedField {
        attributes: lower_attrs(&field.attrs),
        vis: lower_visibility(&field.vis),
        field_name: field.ident.map(|ident| ident.to_string()),
        ty: lower_type(&field.ty),
    }
}

fn lower_visibility(vis: &syn::Visibility) -> Visibility {
    match vis {
        syn::Visibility::Public(_) => Visibility::Public,
        syn::Visibility::Restricted(restricted)
            if restricted.path.is_ident("crate") || restricted.path.is_ident("self") =>
        {
            Visibility::Crate
        }
        syn::Visibility::Restricted(_) => Visibility::Restricted,
        syn::Visibility::Inherited => Visibility::Private,
    }
}

fn lower_attrs(attrs: &[Attribute]) -> Vec<ParsedAttribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("difference"))
        .flat_map(|attr| {
            attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                .unwrap_or_default()
                .into_iter()
                .filter_map(lower_meta)
        })
        .collect()
}

fn lower_meta(meta: Meta) -> Option<ParsedAttribute> {
    match meta {
        Meta::Path(path) => Some(ParsedAttribute {
            name: "difference".to_owned(),
            tokens: vec![path_to_string(&path)],
        }),
        Meta::NameValue(name_value) => {
            let value = match name_value.value {
                Expr::Lit(expr_lit) => match expr_lit.lit {
                    Lit::Str(lit) => lit.value(),
                    other => other.to_token_stream().to_string(),
                },
                other => other.to_token_stream().to_string(),
            };
            Some(ParsedAttribute {
                name: "difference".to_owned(),
                tokens: vec![path_to_string(&name_value.path), value],
            })
        }
        Meta::List(_) => None,
    }
}

fn lower_generics(generics: &syn::Generics) -> Vec<ParsedGeneric> {
    let mut lowered = Vec::new();

    for generic in generics.params.iter().map(lower_generic_param) {
        push_or_merge_generic(&mut lowered, generic);
    }

    if let Some(where_clause) = &generics.where_clause {
        for generic in where_clause
            .predicates
            .iter()
            .filter_map(lower_where_predicate)
        {
            push_or_merge_generic(&mut lowered, generic);
        }
    }

    lowered
}

fn push_or_merge_generic(generics: &mut Vec<ParsedGeneric>, generic: ParsedGeneric) {
    let Some(existing) = generics
        .iter_mut()
        .find(|existing| existing.full() == generic.full())
    else {
        generics.push(generic);
        return;
    };

    match (existing, generic) {
        (
            ParsedGeneric::Regular { bounds, .. },
            ParsedGeneric::Regular {
                bounds: other_bounds,
                ..
            }
            | ParsedGeneric::WhereBounded {
                bounds: other_bounds,
                ..
            },
        )
        | (
            ParsedGeneric::WhereBounded { bounds, .. },
            ParsedGeneric::Regular {
                bounds: other_bounds,
                ..
            }
            | ParsedGeneric::WhereBounded {
                bounds: other_bounds,
                ..
            },
        ) => bounds.extend(other_bounds),
        (
            ParsedGeneric::Lifetime { bounds, .. },
            ParsedGeneric::Lifetime {
                bounds: other_bounds,
                ..
            },
        ) => bounds.extend(other_bounds),
        _ => (),
    }
}

fn lower_generic_param(param: &GenericParam) -> ParsedGeneric {
    match param {
        GenericParam::Type(param) => lower_type_param(param),
        GenericParam::Lifetime(param) => lower_lifetime_param(param),
        GenericParam::Const(param) => lower_const_param(param),
    }
}

fn lower_type_param(param: &TypeParam) -> ParsedGeneric {
    ParsedGeneric::Regular {
        name: param.ident.to_string(),
        default: param.default.as_ref().map(lower_type),
        bounds: lower_type_bounds(&param.bounds),
    }
}

fn lower_lifetime_param(param: &LifetimeParam) -> ParsedGeneric {
    ParsedGeneric::Lifetime {
        name: lifetime_name(&param.lifetime),
        bounds: param
            .bounds
            .iter()
            .map(|lifetime| Lifetime {
                ident: lifetime_name(lifetime),
            })
            .collect(),
    }
}

fn lower_const_param(param: &ConstParam) -> ParsedGeneric {
    ParsedGeneric::Const {
        name: param.ident.to_string(),
        _type: lower_type(&param.ty),
        default: param.default.as_ref().map(lower_const_value),
    }
}

fn lower_where_predicate(predicate: &WherePredicate) -> Option<ParsedGeneric> {
    match predicate {
        WherePredicate::Type(predicate) => Some(ParsedGeneric::WhereBounded {
            name: lower_type(&predicate.bounded_ty).full(),
            bounds: lower_type_bounds(&predicate.bounds),
        }),
        WherePredicate::Lifetime(predicate) => Some(ParsedGeneric::Lifetime {
            name: lifetime_name(&predicate.lifetime),
            bounds: predicate
                .bounds
                .iter()
                .map(|lifetime| Lifetime {
                    ident: lifetime_name(lifetime),
                })
                .collect(),
        }),
        _ => None,
    }
}

fn lower_type_bounds(bounds: &Punctuated<TypeParamBound, syn::Token![+]>) -> Vec<ParsedType> {
    bounds
        .iter()
        .filter_map(|bound| match bound {
            TypeParamBound::Trait(bound) => Some(path_type(bound.to_token_stream().to_string(), None)),
            TypeParamBound::Lifetime(lifetime) => Some(ParsedType {
                ident: Category::Lifetime {
                    path: lifetime_name(lifetime),
                },
                wraps: None,
                ref_type: None,
                as_other: None,
            }),
            TypeParamBound::Verbatim(tokens) => Some(path_type(tokens.to_string(), None)),
            _ => None,
        })
        .collect()
}

fn lower_const_value(expr: &Expr) -> ConstValType {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit) => lit.base10_parse::<isize>().map(ConstValType::Value).unwrap_or_else(
                |_| ConstValType::Named(Box::new(path_type(lit.to_string(), None))),
            ),
            _ => ConstValType::Named(Box::new(path_type(expr.to_token_stream().to_string(), None))),
        },
        _ => ConstValType::Named(Box::new(path_type(expr.to_token_stream().to_string(), None))),
    }
}

fn lower_type(ty: &Type) -> ParsedType {
    match ty {
        Type::Array(array) => {
            let content_type = lower_type(&array.elem);
            let len = lower_array_len(&array.len);
            ParsedType {
                ident: Category::Array {
                    content_type: Box::new(content_type.clone()),
                    len,
                },
                wraps: Some(vec![content_type]),
                ref_type: None,
                as_other: None,
            }
        }
        Type::BareFn(bare) => ParsedType {
            ident: Category::Fn {
                category: FnType::Bare,
                args: Some(Box::new(tuple_type(
                    bare.inputs
                        .iter()
                        .map(|input| lower_type(&input.ty))
                        .collect(),
                ))),
                return_type: lower_return_type(&bare.output).map(Box::new),
            },
            wraps: None,
            ref_type: None,
            as_other: None,
        },
        Type::Group(group) => lower_type(&group.elem),
        Type::ImplTrait(bounds) => object_type(false, &bounds.bounds),
        Type::Never(_) => ParsedType {
            ident: Category::Never,
            wraps: None,
            ref_type: None,
            as_other: None,
        },
        Type::Paren(paren) => tuple_type(vec![lower_type(&paren.elem)]),
        Type::Path(path) => lower_type_path(path),
        Type::Ptr(ptr) => path_type(ptr.to_token_stream().to_string(), None),
        Type::Reference(reference) => {
            let mut ty = lower_type(&reference.elem);
            ty.ref_type = Some(reference.lifetime.as_ref().map(|lifetime| Lifetime {
                ident: lifetime_name(lifetime),
            }));
            ty
        }
        Type::Slice(slice) => {
            let content_type = lower_type(&slice.elem);
            ParsedType {
                ident: Category::Array {
                    content_type: Box::new(content_type.clone()),
                    len: None,
                },
                wraps: Some(vec![content_type]),
                ref_type: None,
                as_other: None,
            }
        }
        Type::TraitObject(object) => object_type(true, &object.bounds),
        Type::Tuple(tuple) => tuple_type(tuple.elems.iter().map(lower_type).collect()),
        Type::Verbatim(tokens) => path_type(tokens.to_string(), None),
        _ => path_type(ty.to_token_stream().to_string(), None),
    }
}

fn lower_type_path(path: &syn::TypePath) -> ParsedType {
    if let Some(qself) = &path.qself {
        let base = lower_type(&qself.ty);
        let as_trait = path
            .path
            .segments
            .iter()
            .take(qself.position)
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let associated = path
            .path
            .segments
            .iter()
            .skip(qself.position)
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        return ParsedType {
            wraps: Some(vec![base.clone()]),
            ident: Category::Associated {
                base: Box::new(base),
                as_trait: Box::new(path_type(as_trait, None)),
                associated: Box::new(path_type(associated, None)),
            },
            ref_type: None,
            as_other: None,
        };
    }

    let Some(last) = path.path.segments.last() else {
        return path_type(path_to_string(&path.path), None);
    };

    let wraps = match &last.arguments {
        PathArguments::AngleBracketed(args) => {
            let args = args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => Some(lower_type(ty)),
                    syn::GenericArgument::Lifetime(lifetime) => Some(ParsedType {
                        ident: Category::Lifetime {
                            path: lifetime_name(lifetime),
                        },
                        wraps: None,
                        ref_type: None,
                        as_other: None,
                    }),
                    syn::GenericArgument::Const(expr) => {
                        Some(path_type(expr.to_token_stream().to_string(), None))
                    }
                    syn::GenericArgument::AssocType(assoc) => Some(ParsedType {
                        ident: Category::AssociatedBound {
                            associated: assoc.ident.to_string(),
                            is: Box::new(lower_type(&assoc.ty)),
                        },
                        wraps: None,
                        ref_type: None,
                        as_other: None,
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (!args.is_empty()).then_some(args)
        }
        _ => None,
    };

    let path_without_args = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");

    path_type(path_without_args, wraps)
}

fn lower_array_len(expr: &Expr) -> Option<ConstValType> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit) => lit.base10_parse::<isize>().ok().map(ConstValType::Value),
            _ => Some(ConstValType::Named(Box::new(path_type(
                expr.to_token_stream().to_string(),
                None,
            )))),
        },
        _ => Some(ConstValType::Named(Box::new(path_type(
            expr.to_token_stream().to_string(),
            None,
        )))),
    }
}

fn lower_return_type(return_type: &syn::ReturnType) -> Option<ParsedType> {
    match return_type {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some(lower_type(ty)),
    }
}

fn object_type(is_dyn: bool, bounds: &Punctuated<TypeParamBound, syn::Token![+]>) -> ParsedType {
    ParsedType {
        ident: Category::Object {
            is_dyn,
            trait_names: lower_type_bounds(bounds),
        },
        wraps: None,
        ref_type: None,
        as_other: None,
    }
}

fn tuple_type(contents: Vec<ParsedType>) -> ParsedType {
    ParsedType {
        ident: Category::Tuple {
            contents: contents.clone(),
        },
        wraps: Some(contents),
        ref_type: None,
        as_other: None,
    }
}

fn path_type(path: String, wraps: Option<Vec<ParsedType>>) -> ParsedType {
    ParsedType {
        ident: Category::Named { path },
        wraps,
        ref_type: None,
        as_other: None,
    }
}

fn empty_type() -> ParsedType {
    ParsedType {
        ident: Category::None,
        wraps: None,
        ref_type: None,
        as_other: None,
    }
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn lifetime_name(lifetime: &syn::Lifetime) -> String {
    lifetime.ident.to_string()
}
