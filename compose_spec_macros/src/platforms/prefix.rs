//! [`Prefix`] parsing helper.

use syn::{
    parse::{Parse, ParseStream},
    Attribute, Error, Ident, Result, Token, Visibility,
};

/// Enum definition prefix, e.g. `#[doc = "Platform docs"] pub enum Platform`.
pub struct Prefix {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub enum_token: Token![enum],
    pub ident: Ident,
}

impl Parse for Prefix {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attributes: Attribute::parse_outer(input)?,
            visibility: input.parse()?,
            enum_token: input.parse()?,
            ident: input.parse()?,
        })
    }
}

impl Prefix {
    /// Construct a helper for checking that the ident is an expected value.
    pub fn peek_ident(&self) -> PeekIdent {
        PeekIdent {
            ident: &self.ident,
            comparisons: Vec::with_capacity(3),
        }
    }
}

/// Support for checking if the [`Prefix`] ident is an expected value.
///
/// Created with [`Prefix::peek_ident()`].
pub struct PeekIdent<'a> {
    ident: &'a Ident,
    comparisons: Vec<&'static str>,
}

impl<'a> PeekIdent<'a> {
    /// Whether the [`Prefix`] ident matches an expected value.
    pub fn is(&mut self, ident: &'static str) -> bool {
        self.comparisons.push(ident);
        self.ident == ident
    }

    /// Trigger an error at the [`Prefix`] ident location, with a message that the last checked
    /// ident was a duplicate.
    pub fn duplicate(&self) -> Error {
        let duplicate = self.comparisons.last().copied().unwrap_or_default();
        Error::new(self.ident.span(), format_args!("duplicate `{duplicate}`"))
    }

    /// Trigger an error at the [`Prefix`] ident location, with a message containing a list of
    /// checked idents.
    pub fn error(self) -> Error {
        let mut message = String::from("expected");
        for comparison in self.comparisons {
            let prefix = if message.is_empty() {
                "expected `"
            } else {
                " or `"
            };
            message.push_str(prefix);
            message.push_str(comparison);
            message.push('`');
        }
        Error::new(self.ident.span(), message)
    }
}
