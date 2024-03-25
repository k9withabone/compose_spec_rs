//! Expansion of `Platform` enum, parsing and expansion of `Os` and `{Os}Arch` enums.

use std::iter;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket},
    Attribute, Ident, LitStr, Result, Token, Type, Visibility,
};

use super::{impl_traits, kw, prefix::Prefix, ArchMap, Platform};

/// Definition of platform operating systems and their architectures.
///
/// `pub enum Os { #(#items),* }`
pub(super) struct Os {
    attributes: Vec<Attribute>,
    visibility: Visibility,
    _enum: Token![enum],
    ident: Ident,
    _brace: Brace,
    items: Punctuated<Item, Token![,]>,
}

impl Prefix {
    /// Continue parsing the `input` into an [`Os`].
    pub(super) fn parse_os(self, input: ParseStream) -> Result<Os> {
        let Self {
            attributes,
            visibility,
            enum_token,
            ident,
        } = self;

        let items;
        Ok(Os {
            attributes,
            visibility,
            _enum: enum_token,
            ident,
            _brace: braced!(items in input),
            items: Punctuated::parse_terminated(&items)?,
        })
    }
}

impl Os {
    /// Iterator of string literals in all OS arch lists.
    pub(super) fn arch_list(&self) -> impl Iterator<Item = &LitStr> {
        self.items.iter().flat_map(Item::arch_list)
    }

    /// Expand into a `Platform` enum with functions:
    ///
    /// - `const fn as_str(&self) -> &'static str`
    /// - `const fn os(&self) -> Os`
    /// - `fn arch(&self) -> Option<Arch>`
    ///
    /// And implementations for:
    ///
    /// - [`AsRef<str>`]
    /// - [`Display`](std::fmt::Display)
    /// - [`FromStr`](std::str::FromStr)
    /// - [`TryFrom<&str>`]
    /// - [`From<Os>`]
    ///
    /// Additionally, `{Os}Arch` enums are generated.
    pub(super) fn expand_platform(
        &self,
        platform: &Platform,
        apply_to_all: &[Attribute],
        arch_map: &ArchMap,
        from_str_error: &Type,
        try_from_arch_error: &Type,
    ) -> TokenStream {
        let Platform {
            attributes,
            visibility,
            _enum,
            ident,
            _semicolon,
        } = platform;

        let enum_variants = self.items.iter().map(Item::platform_variant);
        let as_str_arms = self
            .items
            .iter()
            .map(|item| item.platform_as_str_arms(arch_map));
        let os_arms = self.items.iter().map(Item::platform_os_arm);
        let arch_arms = self.items.iter().map(Item::platform_arch_arm);
        let from_str_arms = self
            .items
            .iter()
            .map(|item| item.platform_from_str_arms(arch_map));
        let os = &self.ident;
        let from_os_arms = self.items.iter().map(|item| item.platform_from_os_arm(os));
        let os_archs = self.items.iter().map(|item| {
            item.expand_os_arch(
                apply_to_all,
                visibility,
                arch_map,
                from_str_error,
                try_from_arch_error,
            )
        });

        let traits = impl_traits(ident, from_str_error, from_str_arms);

        quote! {
            #(#attributes)*
            #(#apply_to_all)*
            #visibility enum #ident {
                #(#enum_variants)*
            }

            impl #ident {
                /// Static string slice in the format `{os}[/{arch}[/{variant}]]`.
                #visibility const fn as_str(&self) -> &'static str {
                    match self {
                        #(#as_str_arms)*
                    }
                }

                /// Platform operating system, without architecture information.
                #visibility const fn os(&self) -> Os {
                    match self {
                        #(#os_arms)*
                    }
                }

                /// Platform architecture, if set.
                #visibility fn arch(&self) -> ::std::option::Option<Arch> {
                    match self {
                        #(#arch_arms)*
                    }
                }
            }

            #traits

            impl ::std::convert::From<#os> for #ident {
                fn from(value: #os) -> Self {
                    match value {
                        #(#from_os_arms)*
                    }
                }
            }

            #(#os_archs)*
        }
    }

    /// Expand into an `Os` enum with a `as_str()` function and implementations for:
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

        let variants = items.iter().map(Item::os_variant);
        let as_str_arms = items.iter().map(Item::os_as_str_arm);
        let from_str_arms = items.iter().map(Item::os_from_str_arm);

        let traits = impl_traits(ident, from_str_error, from_str_arms);

        quote! {
            #(#attributes)*
            #(#apply_to_all)*
            #visibility enum #ident {
                #(#variants)*
            }

            impl #ident {
                /// Static string slice of the OS name as used in a [`Platform`] string.
                #visibility const fn as_str(&self) -> &'static str {
                    match self {
                        #(#as_str_arms)*
                    }
                }
            }

            #traits
        }
    }
}

/// Mapping of an `Os` variant ident to a string literal and architectures.
///
/// `#(#attributes)* #ident => #name #fields`
struct Item {
    attributes: Vec<Attribute>,
    ident: Ident,
    _arrow: Token![=>],
    name: LitStr,
    fields: Fields,
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            ident: input.parse()?,
            _arrow: input.parse()?,
            name: input.parse()?,
            fields: input.parse()?,
        })
    }
}

impl Item {
    /// [`Iterator`] of string literals in this OS's `arch` list.
    fn arch_list(&self) -> impl Iterator<Item = &LitStr> {
        self.fields.arch.list.iter()
    }

    /// Ident used for the `{Os}Arch` enum.
    fn arch_enum_ident(&self) -> Ident {
        format_ident!("{}Arch", self.ident)
    }

    /// Variant for use in defining the `Platform` enum.
    fn platform_variant(&self) -> TokenStream {
        let attributes = &self.attributes;
        let variant = &self.ident;
        let os_arch = self.arch_enum_ident();
        quote! {
            #(#attributes)*
            #variant(::std::option::Option<#os_arch>),
        }
    }

    /// Match arms for use in `Platform::as_str()`.
    fn platform_as_str_arms(&self, arch_map: &ArchMap) -> TokenStream {
        let os = &self.ident;
        let os_str = &self.name;

        let none = quote!(::std::option::Option::None);

        let no_arch = quote!(Self::#os(#none) => #os_str,);

        iter::once(no_arch)
            .chain(self.fields.arch.platform_as_str_arms(arch_map, self))
            .collect()
    }

    /// Match arm for use in `Platform::os()`.
    fn platform_os_arm(&self) -> TokenStream {
        let os = &self.ident;
        quote!(Self::#os(_) => Os::#os,)
    }

    /// Match arm for use in `Platform::arch()`.
    fn platform_arch_arm(&self) -> TokenStream {
        let os = &self.ident;
        quote!(Self::#os(arch) => arch.clone().map(::std::convert::Into::into),)
    }

    /// Match arms for use in [`FromStr`](std::str::FromStr) for `Platform`.
    fn platform_from_str_arms(&self, arch_map: &ArchMap) -> TokenStream {
        let os = &self.ident;
        let os_str = &self.name;

        let ok = quote!(::std::result::Result::Ok);
        let none = quote!(::std::option::Option::None);

        let no_arch = quote!(#os_str => #ok(Self::#os(#none)),);

        iter::once(no_arch)
            .chain(self.fields.arch.platform_from_str_arms(arch_map, self))
            .collect()
    }

    /// Match arm for use in `impl From<Os> for Platform`.
    fn platform_from_os_arm(&self, os: &Ident) -> TokenStream {
        let variant = &self.ident;
        quote!(#os::#variant => Self::#variant(None),)
    }

    /// Variant for use in defining the `Os` enum.
    fn os_variant(&self) -> TokenStream {
        let attributes = &self.attributes;
        let variant = &self.ident;
        quote! {
            #(#attributes)*
            #variant,
        }
    }

    /// Match arm for use in `Os::as_str()`.
    fn os_as_str_arm(&self) -> TokenStream {
        let variant = &self.ident;
        let str = &self.name;
        quote! (Self::#variant => #str,)
    }

    /// Match arm for use in [`FromStr`](std::str::FromStr).
    fn os_from_str_arm(&self) -> TokenStream {
        let str = &self.name;
        let variant = &self.ident;
        quote!(#str => ::std::result::Result::Ok(Self::#variant),)
    }

    /// Expand into an `{Os}Arch` enum with an `OS` associated constant, `as_str()` function, and
    /// implementations for:
    ///
    /// - [`AsRef<str>`]
    /// - [`Display`](std::fmt::Display)
    /// - [`FromStr`](std::str::FromStr)
    /// - [`TryFrom<&str>`]
    /// - [`TryFrom<Arch>`]
    /// - [`From<{Os}Arch>`] for `Arch`
    fn expand_os_arch(
        &self,
        apply_to_all: &[Attribute],
        visibility: &Visibility,
        arch_map: &ArchMap,
        from_str_error: &Type,
        try_from_arch_error: &Type,
    ) -> TokenStream {
        let attributes = &self.fields.arch.attributes;
        let ident = self.arch_enum_ident();
        let enum_variants = self.fields.arch.os_arch_enum_variants(arch_map);
        let os = &self.ident;
        let as_str_arms = self.fields.arch.os_arch_as_str_arms(arch_map);
        let from_str_arms = self.fields.arch.os_arch_from_str_arms(arch_map);
        let try_from_arch_arms = self.fields.arch.os_arch_try_from_arch_arms(arch_map);
        let arch_from_os_arch_arms = self.fields.arch.arch_from_os_arch_arms(arch_map, &ident);

        let traits = impl_traits(&ident, from_str_error, from_str_arms);

        quote! {
            #(#attributes)*
            #(#apply_to_all)*
            #visibility enum #ident {
                #(#enum_variants)*
            }

            impl #ident {
                /// The [`Os`] which this set of architectures corresponds to.
                #visibility const OS: Os = Os::#os;

                /// Static string slice of the architecture name as used in a [`Platform`] string.
                #visibility const fn as_str(&self) -> &'static str {
                    match self {
                        #(#as_str_arms)*
                    }
                }
            }

            #traits

            impl ::std::convert::TryFrom<Arch> for #ident {
                type Error = #try_from_arch_error;

                fn try_from(arch: Arch) -> ::std::result::Result<Self, Self::Error> {
                    match arch {
                        #(#try_from_arch_arms)*
                        arch => Err(#try_from_arch_error { arch, os: Self::OS }),
                    }
                }
            }

            impl ::std::convert::From<#ident> for Arch {
                fn from(value: #ident) -> Arch {
                    match value {
                        #(#arch_from_os_arch_arms)*
                    }
                }
            }
        }
    }
}

/// [`Item`] fields.
///
/// `{ #arch }`
struct Fields {
    _brace: Brace,
    arch: Arch,
}

impl Parse for Fields {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            _brace: braced!(content in input),
            arch: content.parse()?,
        })
    }
}

/// [`Item`] architectures.
///
/// `arch: [#(#list),*],`
struct Arch {
    attributes: Vec<Attribute>,
    _token: kw::arch,
    _colon: Token![:],
    _bracket: Bracket,
    list: Punctuated<LitStr, Token![,]>,
    /// Precomputed `list` values, used when generating `{Os}Arch` enums.
    list_values: Vec<String>,
    _comma: Token![,],
}

impl Parse for Arch {
    fn parse(input: ParseStream) -> Result<Self> {
        let list;
        let attributes = Attribute::parse_outer(input)?;
        let arch_token = input.parse()?;
        let colon = input.parse()?;
        let bracket = bracketed!(list in input);
        let list = Punctuated::parse_terminated(&list)?;
        let list_values = list.iter().map(LitStr::value).collect();

        Ok(Self {
            attributes,
            _token: arch_token,
            _colon: colon,
            _bracket: bracket,
            list,
            list_values,
            _comma: input.parse()?,
        })
    }
}

impl Arch {
    /// Match arm(s) for use in `Platform::as_str()`.
    fn platform_as_str_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
        os: &'a Item,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        let os_arch = os.arch_enum_ident();
        let os_str = &os.name;
        let os = &os.ident;
        self.list_values
            .iter()
            .map(move |arch| arch_map.platform_as_str_arms(arch, os, &os_arch, os_str))
    }

    /// Match arm(s) for use in [`FromStr`](std::str::FromStr) for `Platform`.
    fn platform_from_str_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
        os: &'a Item,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        let os_arch = os.arch_enum_ident();
        let os_str = &os.name;
        let os = &os.ident;
        self.list_values
            .iter()
            .map(move |arch| arch_map.platform_from_str_arms(arch, os, &os_arch, os_str))
    }

    /// Variants for use in defining `{Os}Arch` enums.
    fn os_arch_enum_variants<'a>(
        &'a self,
        arch_map: &'a ArchMap,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        self.list_values
            .iter()
            .map(|arch| arch_map.arch_enum_variant(arch))
    }

    /// Match arms for use in `{Os}Arch::as_str()`.
    fn os_arch_as_str_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        self.list_values
            .iter()
            .map(|arch| arch_map.arch_as_str_arms(arch))
    }

    /// Match arms for use in [`FromStr`](std::str::FromStr).
    fn os_arch_from_str_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        self.list_values
            .iter()
            .map(|arch| arch_map.arch_from_str_arms(arch))
    }

    /// Match arms for use in [`TryFrom<Arch>`] for `{Os}Arch`.
    fn os_arch_try_from_arch_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        self.list_values
            .iter()
            .map(|arch| arch_map.os_arch_try_from_arch_arm(arch))
    }

    /// Match arms for use in [`From<{Os}Arch>`] for `Arch`.
    fn arch_from_os_arch_arms<'a>(
        &'a self,
        arch_map: &'a ArchMap,
        os_arch: &'a Ident,
    ) -> impl Iterator<Item = TokenStream> + 'a {
        self.list_values
            .iter()
            .map(|arch| arch_map.arch_from_os_arch_arm(arch, os_arch))
    }
}
