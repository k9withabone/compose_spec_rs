//! Parsing and expansion of the `Arch` enum.

use std::{collections::HashMap, iter};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket},
    Attribute, Error, Ident, LitStr, Result, Token, Type, Visibility,
};

use super::{concat, impl_traits, kw, prefix::Prefix};

/// Definition of platform architectures.
///
/// `pub enum Arch { #(#items),* }`
pub(super) struct Arch {
    attributes: Vec<Attribute>,
    visibility: Visibility,
    _enum: Token![enum],
    ident: Ident,
    _brace: Brace,
    items: Punctuated<Item, Token![,]>,
}

#[allow(clippy::multiple_inherent_impl)]
impl Prefix {
    /// Continue parsing the `input` into an [`Arch`].
    pub(super) fn parse_arch(self, input: ParseStream) -> Result<Arch> {
        let Self {
            attributes,
            visibility,
            enum_token,
            ident,
        } = self;

        let items;
        Ok(Arch {
            attributes,
            visibility,
            _enum: enum_token,
            ident,
            _brace: braced!(items in input),
            items: Punctuated::parse_terminated(&items)?,
        })
    }
}

impl Arch {
    /// Create an [`ArchMap`](Map).
    ///
    /// # Errors
    ///
    /// Checks if each OS arch in `os_arch_names` is defined and that no arch is unused.
    pub(super) fn to_map<'a>(
        &self,
        os_arch_names: impl IntoIterator<Item = &'a LitStr>,
    ) -> Result<Map> {
        let map = Map {
            inner: self
                .items
                .iter()
                .map(|item| (item.name.value(), item))
                .collect(),
        };

        let mut visited: HashMap<_, _> = map
            .inner
            .iter()
            .map(|(key, item)| (key, (false, item.ident.span())))
            .collect();
        let mut error: Option<Error> = None;

        for os_arch_name in os_arch_names {
            if let Some((visited, _)) = visited.get_mut(&os_arch_name.value()) {
                *visited = true;
            } else {
                let new_error = Error::new(os_arch_name.span(), "undefined arch");
                if let Some(error) = error.as_mut() {
                    error.combine(new_error);
                } else {
                    error = Some(new_error);
                }
            }
        }

        for (visited, span) in visited.into_values() {
            if !visited {
                let new_error = Error::new(span, "unused arch");
                if let Some(error) = error.as_mut() {
                    error.combine(new_error);
                } else {
                    error = Some(new_error);
                }
            }
        }

        error.map_or(Ok(map), Err)
    }

    /// Expand into an `Arch` enum and `{Arch}Variant` enums with a `as_str()` function and
    /// implementations for:
    ///
    /// - [`AsRef<str>`]
    /// - [`Display`](std::fmt::Display)
    /// - [`FromStr`](std::str::FromStr)
    /// - [`TryFrom<&str>`]
    pub(super) fn expand(&self, apply_to_all: &[Attribute], from_str_error: &Type) -> TokenStream {
        let Self {
            attributes,
            visibility,
            _enum,
            ident,
            _brace,
            items,
        } = self;

        let enum_variants = items.iter().map(Item::arch_enum_variant);
        let as_str_arms = items.iter().map(Item::arch_as_str_arms);
        let from_str_arms = items.iter().map(Item::arch_from_str_arms);

        let traits = impl_traits(ident, from_str_error, from_str_arms);

        let arch_variants = self
            .items
            .iter()
            .filter_map(|item| item.expand_arch_variant(apply_to_all, visibility, from_str_error));

        quote! {
            #(#attributes)*
            #(#apply_to_all)*
            #visibility enum #ident {
                #(#enum_variants)*
            }

            impl #ident {
                /// Static string slice of the architecture name as used in a [`Platform`] string.
                #visibility const fn as_str(&self) -> &'static str {
                    match self {
                        #(#as_str_arms)*
                    }
                }
            }

            #traits

            #(#arch_variants)*
        }
    }
}

/// Map of architecture names to [`Arch`] values.
///
/// Created with [`Arch::to_map()`].
pub(super) struct Map<'a> {
    inner: HashMap<String, &'a Item>,
}

impl<'a> Map<'a> {
    /// Get the [`Item`] matching the given `arch`.
    ///
    /// # Panics
    ///
    /// Panics if given a non-validated arch.
    fn get(&self, arch: &str) -> &Item {
        self.inner.get(arch).expect("map and arch validated")
    }

    /// Match arm(s) for use in `Platform::as_str()`.
    pub(super) fn platform_as_str_arms(
        &self,
        arch: &str,
        os: &Ident,
        os_arch: &Ident,
        os_str: &LitStr,
    ) -> TokenStream {
        self.get(arch).platform_as_str_arms(os, os_arch, os_str)
    }

    /// Match arm(s) for use in [`FromStr`](std::str::FromStr) for `Platform`.
    pub(super) fn platform_from_str_arms(
        &self,
        arch: &str,
        os: &Ident,
        os_arch: &Ident,
        os_str: &LitStr,
    ) -> TokenStream {
        self.get(arch).platform_from_str_arms(os, os_arch, os_str)
    }

    /// Variant for use in defining `Arch` and `{Os}Arch` enums.
    pub(super) fn arch_enum_variant(&self, arch: &str) -> TokenStream {
        self.get(arch).arch_enum_variant()
    }

    /// Match arm(s) for use in `Arch::as_str()` or `{Os}Arch::as_str()`.
    pub(super) fn arch_as_str_arms(&self, arch: &str) -> TokenStream {
        self.get(arch).arch_as_str_arms()
    }

    /// Match arm(s) for use in [`FromStr`](std::str::FromStr).
    pub(super) fn arch_from_str_arms(&self, arch: &str) -> TokenStream {
        self.get(arch).arch_from_str_arms()
    }

    /// Match arm for use in [`TryFrom<Arch>`] for `{Os}Arch`.
    pub(super) fn os_arch_try_from_arch_arm(&self, arch: &str) -> TokenStream {
        self.get(arch).os_arch_try_from_arch_arm()
    }

    /// Match arm for use in [`From<{Os}Arch>`] for `Arch`.
    pub(super) fn arch_from_os_arch_arm(&self, arch: &str, os_arch: &Ident) -> TokenStream {
        self.get(arch).arch_from_os_arch_arm(os_arch)
    }
}

/// Mapping of an `Arch` variant ident to a string literal and optional variants.
///
/// `#(#attributes)* #ident => #name #(#fields)?`
struct Item {
    attributes: Vec<Attribute>,
    ident: Ident,
    _arrow: Token![=>],
    name: LitStr,
    fields: Option<Fields>,
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            ident: input.parse()?,
            _arrow: input.parse()?,
            name: input.parse()?,
            fields: input.peek(Brace).then(|| input.parse()).transpose()?,
        })
    }
}

impl Item {
    /// `{Arch}Variant` [`Ident`](struct@Ident) and arch [`Variants`], if any.
    fn variants(&self) -> Option<(Ident, &Variants)> {
        self.fields.as_ref().map(|fields| {
            let ident = format_ident!("{}Variant", self.ident);
            (ident, &fields.variants)
        })
    }

    /// Match arm(s) for use in `Platform::as_str()`.
    fn platform_as_str_arms(&self, os: &Ident, os_arch: &Ident, os_str: &LitStr) -> TokenStream {
        let os_str = os_str.to_token_stream();
        let some = quote!(::std::option::Option::Some);

        self.str_arms(|arch, str| {
            let str = concat([&os_str, &str], '/');
            quote! {
                Self::#os(#some(#os_arch::#arch)) => #str,
            }
        })
    }

    /// Match arm(s) for use in [`FromStr`](std::str::FromStr) for `Platform`.
    fn platform_from_str_arms(&self, os: &Ident, os_arch: &Ident, os_str: &LitStr) -> TokenStream {
        let os_str = os_str.to_token_stream();
        let ok = quote!(::std::result::Result::Ok);
        let some = quote!(::std::option::Option::Some);

        self.str_arms(|arch, str| {
            let str = concat([&os_str, &str], '/');
            quote! {
                #str => #ok(Self::#os(#some(#os_arch::#arch))),
            }
        })
    }

    /// Variant for use in defining the `Arch` and `{Os}Arch` enums.
    fn arch_enum_variant(&self) -> TokenStream {
        let attributes = &self.attributes;
        let ident = &self.ident;

        if let Some((arch_variant, _)) = self.variants() {
            quote! {
                #(#attributes)*
                #ident(::std::option::Option<#arch_variant>),
            }
        } else {
            quote! {
                #(#attributes)*
                #ident,
            }
        }
    }

    /// Match arm(s) for use in `Arch::as_str()` or `{Os}Arch::as_str()`.
    fn arch_as_str_arms(&self) -> TokenStream {
        self.str_arms(|arch, str| quote!(Self::#arch => #str,))
    }

    /// Match arm(s) for use in [`FromStr`](std::str::FromStr).
    fn arch_from_str_arms(&self) -> TokenStream {
        self.str_arms(|arch, str| {
            quote!(
                #str => ::std::result::Result::Ok(Self::#arch),
            )
        })
    }

    /// Match arm(s) for implementations of [`FromStr`](std::str::FromStr) and `as_str()`.
    ///
    /// `arm_fn` is called with the arch (including possible variants) and string literal to match
    /// against or map to.
    fn str_arms<F>(&self, mut arm_fn: F) -> TokenStream
    where
        F: FnMut(TokenStream, TokenStream) -> TokenStream,
    {
        let arch = &self.ident;
        let str = self.name.to_token_stream();

        if let Some((arch_variant, variants)) = self.variants() {
            let option = quote!(::std::option::Option);

            let no_variant = arm_fn(quote!(#arch(#option::None)), str);
            let variants = variants.list.iter().map(|variant| {
                let str = concat([&self.name, &variant.name], '/');
                let variant = &variant.ident;
                arm_fn(quote!(#arch(#option::Some(#arch_variant::#variant))), str)
            });

            iter::once(no_variant).chain(variants).collect()
        } else {
            arm_fn(arch.to_token_stream(), str)
        }
    }

    /// Match arm for use in [`TryFrom<Arch>`] for `{Os}Arch`.
    fn os_arch_try_from_arch_arm(&self) -> TokenStream {
        let ident = &self.ident;

        if self.fields.is_some() {
            quote!(Arch::#ident(variant) => ::std::result::Result::Ok(Self::#ident(variant)),)
        } else {
            quote!(Arch::#ident => ::std::result::Result::Ok(Self::#ident),)
        }
    }

    /// Match arm for use in [`From<{Os}Arch>`] for `Arch`.
    fn arch_from_os_arch_arm(&self, os_arch: &Ident) -> TokenStream {
        let ident = &self.ident;

        if self.fields.is_some() {
            quote!(#os_arch::#ident(variant) => Self::#ident(variant),)
        } else {
            quote!(#os_arch::#ident => Self::#ident,)
        }
    }

    /// Expand into an `{Arch}Variant` enum with a `as_str()` function and implementations for:
    ///
    /// - [`AsRef<str>`]
    /// - [`Display`](std::fmt::Display)
    /// - [`FromStr`](std::str::FromStr)
    /// - [`TryFrom<&str>`]
    fn expand_arch_variant(
        &self,
        apply_to_all: &[Attribute],
        visibility: &Visibility,
        from_str_error: &Type,
    ) -> Option<TokenStream> {
        self.variants().map(|(ident, variants)| {
            let attributes = &variants.attributes;
            let enum_variants = variants.list.iter().map(Variant::enum_variant);
            let as_str_arms = variants.list.iter().map(Variant::enum_as_str_arm);
            let from_str_arms = variants.list.iter().map(Variant::enum_from_str_arm);

            let traits = impl_traits(&ident, from_str_error, from_str_arms);

            quote! {
                #(#attributes)*
                #(#apply_to_all)*
                #visibility enum #ident {
                    #(#enum_variants)*
                }

                impl #ident {
                    /// Static string slice of the architecture variant name as used in a [`Platform`] string.
                    #visibility const fn as_str(&self) -> &'static str {
                        match self {
                            #(#as_str_arms)*
                        }
                    }
                }

                #traits
            }
        })
    }
}

/// [`Item`] fields.
///
/// `{ #variants }`
struct Fields {
    _brace: Brace,
    variants: Variants,
}

impl Parse for Fields {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let brace = braced!(content in input);
        let variants = content.parse()?;

        Ok(Self {
            _brace: brace,
            variants,
        })
    }
}

/// [`Item`] variants.
///
/// `#(#attributes)* variants: [#(#list),*],`
struct Variants {
    attributes: Vec<Attribute>,
    _token: kw::variants,
    _colon: Token![:],
    _bracket: Bracket,
    list: Punctuated<Variant, Token![,]>,
    _comma: Token![,],
}

impl Parse for Variants {
    fn parse(input: ParseStream) -> Result<Self> {
        let list;
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            _token: input.parse()?,
            _colon: input.parse()?,
            _bracket: bracketed!(list in input),
            list: Punctuated::parse_terminated(&list)?,
            _comma: input.parse()?,
        })
    }
}

/// [`Item`] variant.
///
/// `#(#attributes)* #ident => #name`
struct Variant {
    attributes: Vec<Attribute>,
    ident: Ident,
    _arrow: Token![=>],
    name: LitStr,
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            ident: input.parse()?,
            _arrow: input.parse()?,
            name: input.parse()?,
        })
    }
}

impl Variant {
    /// Variant for use in defining the `{Arch}Variant` enum.
    fn enum_variant(&self) -> TokenStream {
        let attributes = &self.attributes;
        let ident = &self.ident;

        quote! {
            #(#attributes)*
            #ident,
        }
    }

    /// Match arm for use in `{Arch}Variant::as_str()`.
    fn enum_as_str_arm(&self) -> TokenStream {
        let ident = &self.ident;
        let str = &self.name;

        quote!(Self::#ident => #str,)
    }

    /// Match arm for use in [`FromStr`](std::str::FromStr) implementation for `{Arch}Variant`.
    fn enum_from_str_arm(&self) -> TokenStream {
        let str = &self.name;
        let ident = &self.ident;

        quote!(#str => ::std::result::Result::Ok(Self::#ident),)
    }
}
