//! Implementation of the [`SerializeDisplay`](super::serialize_display()),
//! [`DeserializeFromStr`](super::deserialize_from_str()), and
//! [`DeserializeTryFromString`](super::deserialize_try_from_string()) derive macros.

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, DeriveInput, Generics, Ident, Lifetime, LifetimeParam, LitStr, Result};

/// [`SerializeDisplay`](super::serialize_display()) derive macro input.
pub struct Input {
    expecting: Option<LitStr>,
    ident: Ident,
    generics: Generics,
}

impl Input {
    /// Create an [`Input`] from [`DeriveInput`].
    pub fn from_syn(
        DeriveInput {
            attrs,
            ident,
            generics,
            ..
        }: DeriveInput,
    ) -> Result<Self> {
        Ok(Self {
            expecting: parse_expecting(&attrs)?,
            ident,
            generics,
        })
    }

    /// Generate a [`Serialize`] implementation for the input type using its
    /// [`Display`](std::fmt::Display) implementation.
    ///
    /// [`Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
    pub fn impl_serialize_display(self) -> TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;

        quote! {
            impl #impl_generics ::serde::Serialize for #ident #type_generics
            #where_clause
            {
                fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    serializer.collect_str(self)
                }
            }
        }
    }

    /// Generate a [`Deserialize`] implementation for the input type using its
    /// [`FromStr`](std::str::FromStr) implementation.
    ///
    /// [`Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
    pub fn impl_deserialize_from_str(self) -> TokenStream {
        self.impl_deserialize(quote!(crate::serde::FromStrVisitor))
    }

    /// Generate a [`Deserialize`] implementation for the input type using its [`TryFrom<String>`]
    /// implementation.
    ///
    /// [`Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
    pub fn impl_deserialize_try_from_string(self) -> TokenStream {
        self.impl_deserialize(quote!(crate::serde::TryFromStringVisitor))
    }

    /// Generate a [`Deserialize`] implementation for the input type by creating a [`Visitor`] of
    /// the given `visitor_type`.
    ///
    /// [`Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
    /// [`Visitor`]: https://docs.rs/serde/latest/serde/de/trait.Visitor.html
    fn impl_deserialize(self, visitor_type: impl ToTokens) -> TokenStream {
        let ident = self.ident;

        let lifetime = Lifetime::new("'_de", Span::call_site());

        let mut impl_generics = self.generics.clone();
        impl_generics
            .params
            .push(LifetimeParam::new(lifetime.clone()).into());
        let (impl_generics, _, _) = impl_generics.split_for_impl();

        let (_, type_generics, where_clause) = self.generics.split_for_impl();

        let visitor = self.expecting.map_or_else(
            || quote!(#visitor_type::default()),
            |expecting| quote!(#visitor_type::new(#expecting)),
        );

        quote! {
            impl #impl_generics ::serde::Deserialize<#lifetime> for #ident #type_generics
            #where_clause
            {
                fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<#lifetime>,
                {
                    #visitor.deserialize(deserializer)
                }
            }
        }
    }
}

/// Parse the `#[serde(expecting = "...")]` container attribute.
fn parse_expecting(attributes: &[Attribute]) -> Result<Option<LitStr>> {
    let mut expecting = None;

    for attribute in attributes {
        if attribute.path().is_ident("serde") {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("expecting") {
                    if expecting.is_some() {
                        Err(meta.error("duplicate `expecting`"))
                    } else {
                        expecting = meta.value()?.parse().map(Some)?;
                        Ok(())
                    }
                } else if meta.path.is_ident("transparent") {
                    Ok(())
                } else {
                    Err(meta.error("unsupported serde attribute"))
                }
            })?;
        }
    }

    Ok(expecting)
}
