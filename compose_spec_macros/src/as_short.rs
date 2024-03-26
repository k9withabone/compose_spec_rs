//! Implementation of the [`AsShort`](super::as_short()) and [`FromShort`](super::from_short())
//! derive macros.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::ParseStream, spanned::Spanned, AngleBracketedGenericArguments, Data, DataStruct,
    DeriveInput, Error, Expr, Fields, FieldsNamed, GenericArgument, Generics, LitStr,
    PathArguments, Result, Type, TypePath,
};

/// [`AsShort`](super::as_short()) and [`FromShort`](super::from_short()) derive macro input.
///
/// Created with [`Input::from_syn()`]
pub(super) struct Input<'a> {
    /// Name of the input struct.
    ident: &'a Ident,

    /// Input struct generics.
    generics: &'a Generics,

    /// The field marked with `#[as_short(short)]`.
    short_field: ShortField<'a>,

    /// [`Field`]s not marked with `#[as_short(short)]`.
    other_fields: Vec<Field<'a>>,
}

impl<'a> Input<'a> {
    /// Creates an [`Input`] from [`DeriveInput`].
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a struct with named fields, a `as_short` attribute has
    /// an incorrect format or duplicates, or the `#[as_short(short)]` attribute is missing.
    pub(super) fn from_syn(
        DeriveInput {
            ident,
            generics,
            data,
            ..
        }: &'a DeriveInput,
    ) -> Result<Self> {
        let Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named: fields, .. }),
            ..
        }) = data
        else {
            return Err(Error::new(
                ident.span(),
                "must be used on structs with named fields",
            ));
        };

        let mut short_field = None;
        let other_fields = fields
            .iter()
            .filter_map(|field| {
                let ident = field.ident.as_ref().expect("field is named");
                let attribute = match Attribute::from_syn(&field.attrs) {
                    Ok(attribute) => attribute,
                    Err(error) => return Some(Err(error)),
                };
                match attribute {
                    Attribute::Short(span) => {
                        if short_field.is_some() {
                            return Some(Err(Error::new(span, "duplicate `short`")));
                        }
                        short_field = Some(ShortField::new(ident, &field.ty));
                        None
                    }
                    Attribute::Other { if_fn, default_fn } => Some(Ok(Field {
                        if_fn,
                        default_fn,
                        ident,
                        ty: &field.ty,
                    })),
                }
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            ident,
            generics,
            short_field: short_field
                .ok_or_else(|| Error::new(ident.span(), "missing `short` field"))?,
            other_fields,
        })
    }

    /// Generate a `AsShort` implementation for the input type.
    pub(super) fn impl_as_short(&self) -> TokenStream {
        let Self {
            ident,
            generics,
            short_field:
                ShortField {
                    ident: short,
                    ty: short_ty,
                    optional,
                },
            ref other_fields,
        } = *self;

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let if_expressions = other_fields.iter().map(|field| {
            let ident = field.ident;
            let ty = field.ty;
            let if_fn = field.if_fn.as_ref().map_or_else(
                || {
                    if option_type(ty).is_some() {
                        quote!(::std::option::Option::is_none)
                    } else if is_bool(ty) {
                        quote!(::std::ops::Not::not)
                    } else {
                        quote!(<#ty>::is_empty)
                    }
                },
                ToTokens::to_token_stream,
            );
            quote!((#if_fn)(&self.#ident))
        });

        let short = if optional {
            quote!(self.#short.as_ref())
        } else {
            quote!(::std::option::Option::Some(&self.#short))
        };

        quote! {
            impl #impl_generics crate::AsShort for #ident #ty_generics #where_clause {
                type Short = #short_ty;

                fn as_short(&self) -> ::std::option::Option<&Self::Short> {
                    if #(#if_expressions)&&* {
                        #short
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Generate a [`From<Short>`] implementation for the input type.
    pub(super) fn impl_from_short(&self) -> TokenStream {
        let Self {
            ident,
            generics,
            short_field:
                ShortField {
                    ident: short,
                    ty: short_ty,
                    optional,
                },
            ref other_fields,
        } = *self;

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let short_field = if optional {
            quote!(#short: ::std::option::Option::Some(#short))
        } else {
            short.to_token_stream()
        };

        let other_fields = other_fields.iter().map(|field| {
            let ident = field.ident;
            let default_fn = field.default_fn.as_ref().map_or_else(
                || quote!(::std::default::Default::default),
                ToTokens::to_token_stream,
            );
            quote!(#ident: (#default_fn)())
        });

        quote! {
            impl #impl_generics ::std::convert::From<#short_ty> for #ident #ty_generics
            #where_clause
            {
                fn from(#short: #short_ty) -> Self {
                    Self {
                        #short_field,
                        #(#other_fields,)*
                    }
                }
            }
        }
    }
}

/// Returns the inner [`Type`] if `ty` is [`Option<Type>`].
fn option_type(ty: &Type) -> Option<&Type> {
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return None;
    };

    if path.segments.len() != 1 {
        return None;
    }
    let segment = &path.segments[0];

    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };

    if args.len() != 1 || segment.ident != "Option" {
        return None;
    }

    if let GenericArgument::Type(ty) = &args[0] {
        Some(ty)
    } else {
        None
    }
}

/// Returns `true` if the given [`Type`] is a [`bool`].
fn is_bool(ty: &Type) -> bool {
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return false;
    };

    path.is_ident("bool")
}

/// Field marked with `#[as_short(short)]`.
struct ShortField<'a> {
    /// Field name.
    ident: &'a Ident,

    /// Field type, or inner [`Option`] type if `optional` is true.
    ty: &'a Type,

    /// Whether the field is optional.
    optional: bool,
}

impl<'a> ShortField<'a> {
    /// Create a [`ShortField`].
    fn new(ident: &'a Ident, ty: &'a Type) -> Self {
        option_type(ty).map_or(
            Self {
                ident,
                ty,
                optional: false,
            },
            |ty| Self {
                ident,
                ty,
                optional: true,
            },
        )
    }
}

/// Field with an optional `#[as_short([default = {default_fn},][if_fn = {if_fn}])]` attribute.
struct Field<'a> {
    /// Function used to determine if the [`Input`] struct can be represented as its short form.
    ///
    /// Should implement `FnOnce(&<ty>) -> bool`.
    if_fn: Option<Expr>,

    /// Function used to create a default `ty`.
    ///
    /// Should implement `FnOnce() -> <ty>`.
    default_fn: Option<Expr>,

    /// Field name.
    ident: &'a Ident,

    /// Field type.
    ty: &'a Type,
}

/// `as_short` helper attribute.
enum Attribute {
    /// `#[as_short(short)]`
    Short(Span),

    /// `#[as_short([default = {default_fn},][if_fn = {if_fn}])]`
    Other {
        /// Function used to determine if the [`Input`] struct can be represented as its short form.
        ///
        /// Should implement `FnOnce(&<ty>) -> bool`.
        if_fn: Option<Expr>,

        /// Function used to create a default `ty`.
        ///
        /// Should implement `FnOnce() -> <ty>`.
        default_fn: Option<Expr>,
    },
}

impl Attribute {
    /// Create an [`Attribute`] from [`syn::Attribute`]s.
    ///
    /// # Errors
    ///
    /// Returns an error a `as_short` attribute has an incorrect format or duplicates.
    fn from_syn(attributes: &[syn::Attribute]) -> Result<Self> {
        let mut short = None;
        let mut if_fn = None;
        let mut default_fn = None;

        for attribute in attributes {
            if attribute.path().is_ident("as_short") {
                attribute.parse_nested_meta(|meta| {
                    if meta.path.is_ident("short") {
                        if short.is_some() {
                            return Err(meta.error("duplicate `short`"));
                        }
                        short = Some(meta.path.span());
                        Ok(())
                    } else if meta.path.is_ident("if_fn") {
                        if if_fn.is_some() {
                            return Err(meta.error("duplicate `if_fn`"));
                        }
                        if_fn = meta.value().and_then(parse_expr).map(Some)?;
                        Ok(())
                    } else if meta.path.is_ident("default") {
                        if default_fn.is_some() {
                            return Err(meta.error("duplicate `default`"));
                        }
                        default_fn = meta.value().and_then(parse_expr).map(Some)?;
                        Ok(())
                    } else {
                        Err(meta.error("unknown attribute"))
                    }
                })?;
            }
        }

        if let Some(span) = short {
            if if_fn.is_none() && default_fn.is_none() {
                Ok(Self::Short(span))
            } else {
                Err(Error::new(
                    span,
                    "cannot mark a field as the `short` type and have `if_fn` or `default`",
                ))
            }
        } else {
            Ok(Self::Other { if_fn, default_fn })
        }
    }
}

/// Parse an [`Expr`] from a string literal or as is.
fn parse_expr(input: ParseStream) -> Result<Expr> {
    if input.peek(LitStr) {
        input.parse::<LitStr>()?.parse()
    } else {
        input.parse()
    }
}
