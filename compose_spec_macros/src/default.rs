//! Implementation of the [`Default`](super::default()) derive macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Data, DataStruct, DeriveInput, Error, Expr, Fields, FieldsNamed, Generics,
    Ident, Result,
};

/// [`Default`](super::default()) derive macro input.
///
/// Created with [`Input::from_syn()`].
pub(super) struct Input<'a> {
    /// Name of the input struct.
    ident: &'a Ident,

    /// Input struct generics.
    generics: &'a Generics,

    /// Fields of the input struct.
    fields: Vec<Field<'a>>,
}

impl<'a> Input<'a> {
    /// Create an [`Input`] from [`DeriveInput`].
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a struct with named fields or a `default` attribute has
    /// an incorrect format or duplicates.
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

        let fields = fields.iter().map(Field::from_syn).collect::<Result<_>>()?;

        Ok(Self {
            ident,
            generics,
            fields,
        })
    }

    /// Expand the input into a [`Default`] implementation.
    pub(super) fn expand(self) -> TokenStream {
        let Self {
            ident,
            generics,
            fields,
        } = self;

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

        let fields = fields.into_iter().map(Field::expand);

        quote! {
            impl #impl_generics ::std::default::Default for #ident #type_generics #where_clause {
                fn default() -> Self {
                    Self {
                        #(#fields,)*
                    }
                }
            }
        }
    }
}

/// Field with an optional `#[default = {default}]` attribute.
struct Field<'a> {
    /// Expression to use as the default value for the field.
    default: Option<&'a Expr>,

    /// Field name.
    ident: &'a Ident,
}

impl<'a> Field<'a> {
    /// Create a [`Field`] from a [`syn::Field`].
    ///
    /// # Errors
    ///
    /// Returns an error if  a `default` attribute has an incorrect format or duplicates.
    fn from_syn(syn::Field { attrs, ident, .. }: &'a syn::Field) -> Result<Self> {
        let mut default = None;

        for attribute in attrs {
            if attribute.path().is_ident("default") {
                if default.is_some() {
                    return Err(Error::new(
                        attribute.span(),
                        "duplicate `default` attribute",
                    ));
                }

                default = Some(&attribute.meta.require_name_value()?.value);
            }
        }

        Ok(Self {
            default,
            ident: ident.as_ref().expect("named field"),
        })
    }

    /// Expand into a struct field for a [`Default`] implementation.
    fn expand(self) -> TokenStream {
        let Self { default, ident } = self;

        default.map_or_else(
            || quote!(#ident: ::std::default::Default::default()),
            |default| quote!(#ident: #default),
        )
    }
}
