//! Implementation of the [`platforms`](super::platforms()) macro.

mod arch;
mod os;
mod prefix;

/// Keywords
mod kw {
    use syn::custom_keyword;

    custom_keyword!(arch);
    custom_keyword!(ParseError);
    custom_keyword!(TryFromArchError);
    custom_keyword!(variants);
}

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Bracket,
    AttrStyle, Attribute, Error, Ident, LitChar, Result, Token, Type, Visibility,
};

use self::{
    arch::{Arch, Map as ArchMap},
    os::Os,
    prefix::Prefix,
};

/// Input for the [`platforms!`](super::platforms()) macro.
///
/// `#![apply_to_all(#(#apply_to_all),*)] #platform #os #arch #parse_error_type #try_from_arch_error_type`
pub struct Input {
    apply_to_all: Vec<Attribute>,
    platform: Platform,
    os: Os,
    arch: Arch,
    parse_error_type: ParseErrorType,
    try_from_arch_error_type: TryFromArchErrorType,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let apply_to_all = parse_apply_to_all(input)?;
        let mut platform = None;
        let mut os = None;
        let mut arch = None;
        let mut parse_error_type = None;
        let mut try_from_arch_error_type = None;

        while !input.is_empty() {
            if input.peek(Token![type]) {
                if input.peek2(kw::ParseError) {
                    if parse_error_type.is_some() {
                        return Err(input.error("duplicate `ParseError`"));
                    }
                    parse_error_type = input.parse().map(Some)?;
                } else if input.peek2(kw::TryFromArchError) {
                    if try_from_arch_error_type.is_some() {
                        return Err(input.error("duplicate `TryFromArchError`"));
                    }
                    try_from_arch_error_type = input.parse().map(Some)?;
                } else {
                    input.parse::<Token![type]>()?;
                    return Err(input.error("expected `ParseError` or `TryFromArchError`"));
                }
                continue;
            }

            let prefix = Prefix::parse(input)?;
            let mut ident = prefix.peek_ident();
            if ident.is("Platform") {
                if platform.is_some() {
                    return Err(ident.duplicate());
                }
                platform = prefix.parse_platform(input).map(Some)?;
            } else if ident.is("Os") {
                if os.is_some() {
                    return Err(ident.duplicate());
                }
                os = prefix.parse_os(input).map(Some)?;
            } else if ident.is("Arch") {
                if arch.is_some() {
                    return Err(ident.duplicate());
                }
                arch = prefix.parse_arch(input).map(Some)?;
            } else {
                return Err(ident.error());
            }
        }

        Ok(Self {
            apply_to_all,
            platform: platform.ok_or_else(|| input.error("missing `Platform`"))?,
            os: os.ok_or_else(|| input.error("missing `Os`"))?,
            arch: arch.ok_or_else(|| input.error("missing `Arch`"))?,
            parse_error_type: parse_error_type
                .ok_or_else(|| input.error("missing `ParseError`"))?,
            try_from_arch_error_type: try_from_arch_error_type
                .ok_or_else(|| input.error("missing `TryFromArchError`"))?,
        })
    }
}

impl Input {
    /// Expand into `Platform`, `Os`, `Arch`, and `{Os}Arch` enums.
    pub fn expand(&self) -> Result<TokenStream> {
        let Self {
            apply_to_all,
            platform,
            os,
            arch,
            parse_error_type,
            try_from_arch_error_type,
        } = self;

        let arch_map = arch.to_map(os.arch_list())?;

        let from_str_error = &parse_error_type.ty;
        let try_from_arch_error = &try_from_arch_error_type.ty;

        let platform = os.expand_platform(
            platform,
            apply_to_all,
            &arch_map,
            from_str_error,
            try_from_arch_error,
        );
        let os = os.expand(apply_to_all, from_str_error);
        let arch = arch.expand(apply_to_all, from_str_error);

        Ok([platform, os, arch].into_iter().collect())
    }
}

/// Parse `apply_to_all` inner attributes.
///
/// e.g. `#![apply_to_all(derive(Clone, Copy))]`
fn parse_apply_to_all(input: ParseStream) -> Result<Vec<Attribute>> {
    let mut apply_to_all = Vec::with_capacity(1);

    for attribute in Attribute::parse_inner(input)? {
        if !attribute.path().is_ident("apply_to_all") {
            return Err(Error::new(attribute.span(), "unsupported inner attribute"));
        }

        let attributes = attribute
            .parse_args_with(Punctuated::<_, Token![,]>::parse_terminated)?
            .into_iter()
            .map(|meta| Attribute {
                pound_token: <Token![#]>::default(),
                style: AttrStyle::Outer,
                bracket_token: Bracket::default(),
                meta,
            });

        apply_to_all.extend(attributes);
    }

    Ok(apply_to_all)
}

/// Platform enum definition.
///
/// Used to as an attribute target and to define its visibility.
///
/// `pub enum Platform;`
struct Platform {
    attributes: Vec<Attribute>,
    visibility: Visibility,
    _enum: Token![enum],
    ident: Ident,
    _semicolon: Token![;],
}

impl Prefix {
    /// Continue parsing the `input` into a [`Platform`].
    fn parse_platform(self, input: ParseStream) -> Result<Platform> {
        let Self {
            attributes,
            visibility,
            enum_token,
            ident,
        } = self;

        Ok(Platform {
            attributes,
            visibility,
            _enum: enum_token,
            ident,
            _semicolon: input.parse()?,
        })
    }
}

/// Type used as error for [`FromStr`](std::str::FromStr) implementations.
///
/// `type ParseError = #ty;`
struct ParseErrorType {
    _type_token: Token![type],
    _ident: kw::ParseError,
    _eq: Token![=],
    ty: Type,
    _semicolon: Token![;],
}

impl Parse for ParseErrorType {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            _type_token: input.parse()?,
            _ident: input.parse()?,
            _eq: input.parse()?,
            ty: input.parse()?,
            _semicolon: input.parse()?,
        })
    }
}

/// Type used as error for [`TryFrom<Arch>`] implementations for `{Os}Arch` types.
///
/// `type TryFromArchError = #ty;`
struct TryFromArchErrorType {
    _type_token: Token![type],
    _ident: kw::TryFromArchError,
    _eq: Token![=],
    ty: Type,
    _semicolon: Token![;],
}

impl Parse for TryFromArchErrorType {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            _type_token: input.parse()?,
            _ident: input.parse()?,
            _eq: input.parse()?,
            ty: input.parse()?,
            _semicolon: input.parse()?,
        })
    }
}

/// Implement traits for the given `name`.
///
/// Assumes the `name` type has an `as_str()` method. Also, `from_str_error` must be a newtype
/// newtype struct wrapping a [`String`].
///
/// The following traits are implemented:
///
/// - [`AsRef<str>`]
/// - [`Display`](std::fmt::Display)
/// - [`FromStr`](std::str::FromStr)
/// - [`TryFrom<&str>`]
///
/// The `FromStr` and `TryFromStr` implementations use `from_str_error` as the error type.
fn impl_traits<I>(name: &Ident, from_str_error: &Type, from_str_arms: I) -> TokenStream
where
    I: Iterator,
    I::Item: ToTokens,
{
    quote! {
        impl ::std::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = #from_str_error;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms)*
                    s => ::std::result::Result::Err(#from_str_error(s.to_owned())),
                }
            }
        }

        impl ::std::convert::TryFrom<&str> for #name {
            type Error = #from_str_error;

            fn try_from(value: &str) -> ::std::result::Result<Self, Self::Error> {
                value.parse()
            }
        }

    }
}

/// Concatenate literals, separated by `separator`, together into a single string literal.
fn concat<I>(strings: I, separator: char) -> TokenStream
where
    I: IntoIterator,
    I::Item: ToTokens,
{
    let separator = LitChar::new(separator, Span::call_site());
    let mut literals = TokenStream::new();

    let mut strings = strings.into_iter();

    if let Some(first) = strings.next() {
        literals.append_all(quote!(#first,));
    }

    for string in strings {
        literals.append_all(quote!(#separator, #string,));
    }

    quote!(::std::concat!(#literals))
}

#[cfg(test)]
mod tests {
    use syn::LitStr;

    use super::*;

    /// Test [`concat()`] with string literals.
    #[test]
    fn concat_string_literals() {
        let hello = LitStr::new("hello", Span::call_site());
        let world = LitStr::new("world", Span::call_site());
        let tokens = concat([&hello, &world], ' ');
        assert_eq!(
            tokens.to_string(),
            quote!(::std::concat!("hello", ' ', "world",)).to_string(),
        );
    }
}
