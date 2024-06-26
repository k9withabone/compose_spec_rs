//! Types for use as keys in maps.

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

/// Validated identifier for use as a map key in a [`Compose`](crate::Compose) file.
///
/// Used to identify top-level items like `services`, `networks`, and `volumes`.
///
/// Identifiers must not be empty, start with an ASCII letter (a-z, A-Z) or digit (0-9), and only
/// contain ASCII letters (a-z, A-Z), digits (0-9), dots (.), underscores (_), or dashes (-).
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Identifier(Box<str>);

impl Identifier {
    /// Create a new [`Identifier`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid [`Identifier`]. Identifiers must not be
    /// empty, start with an ASCII letter (a-z, A-Z) or digit (0-9), and only contain ASCII letters
    /// (a-z, A-Z), digits (0-9), dots (.), underscores (_), or dashes (-).
    pub fn new<T>(identifier: T) -> Result<Self, InvalidIdentifierError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        // Valid identifier pattern: "[a-zA-Z0-9][a-zA-Z0-9._-]*"
        let mut chars = identifier.as_ref().chars();

        let first = chars.next().ok_or(InvalidIdentifierError::Empty)?;
        if !first.is_ascii_alphanumeric() {
            return Err(InvalidIdentifierError::Start(first));
        }

        for char in chars {
            if !(char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-')) {
                return Err(InvalidIdentifierError::Character(char));
            }
        }

        Ok(Self(identifier.into()))
    }
}

/// Error returned when attempting to create a [`Identifier`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidIdentifierError {
    /// Empty identifier
    #[error("identifier cannot be empty")]
    Empty,

    /// Invalid start character.
    ///
    /// Identifiers must start with an ASCII letter (a-z, A-Z) or digit (0-9).
    #[error(
        "invalid start character `{0}`, identifiers must start with an ASCII letter (a-z, A-Z) \
            or digit (0-9)"
    )]
    Start(char),

    /// Invalid character.
    ///
    /// Identifiers must contain only ASCII letters (a-z, A-Z), digits (0-9),
    /// dots (.), underscores (_), or dashes (-).
    #[error(
        "invalid character `{0}`, identifiers must contain only ASCII letters (a-z, A-Z), \
            digits (0-9), dots (.), underscores (_), or dashes (-)"
    )]
    Character(char),
}

/// Valid map key string.
///
/// Map keys cannot be empty or contain multiple lines.
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct MapKey(Box<str>);

impl MapKey {
    /// Create a new [`MapKey`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid [`MapKey`].
    /// Map keys cannot be empty or have multiple lines (i.e. contain the newline `\n` character).
    pub fn new<T>(key: T) -> Result<Self, InvalidMapKeyError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let key_str = key.as_ref();

        // pattern from schema: "^.+$"
        if key_str.is_empty() {
            Err(InvalidMapKeyError::Empty)
        } else if key_str.contains('\n') {
            Err(InvalidMapKeyError::MultipleLines)
        } else {
            Ok(Self(key.into()))
        }
    }
}

/// Error returned when attempting to create a [`MapKey`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidMapKeyError {
    /// Empty map key
    #[error("map key cannot be empty")]
    Empty,

    /// Map key has multiple lines
    #[error("map key cannot have multiple lines (newline character `\\n` found)")]
    MultipleLines,
}

/// Valid extension key string.
///
/// Extension keys must start with "x-" and cannot contain multiple lines.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ExtensionKey(Box<str>);

impl ExtensionKey {
    /// Create a new [`ExtensionKey`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid [`ExtensionKey`].
    /// Extension keys must start with "x-" and cannot have multiple lines (i.e. contain the newline
    /// `\n` character).
    pub fn new<T>(key: T) -> Result<Self, InvalidExtensionKeyError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let key_str = key.as_ref();

        // pattern from schema: "^x-"
        if !key_str.starts_with("x-") {
            Err(InvalidExtensionKeyError::MissingPrefix(key.into()))
        } else if key_str.contains('\n') {
            Err(InvalidExtensionKeyError::MultipleLines)
        } else {
            Ok(Self(key.into()))
        }
    }

    /// Returns the underlying string slice with the "x-" prefix removed.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn strip_prefix(&self) -> &str {
        self.as_str()
            .strip_prefix("x-")
            .expect("`ExtensionKey`s always start with \"x-\"")
    }
}

/// Error returned when attempting to create a [`ExtensionKey`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InvalidExtensionKeyError {
    /// The "x-" prefix was missing from the extension key.
    #[error("extension key `{0}` does not start with \"x-\"")]
    MissingPrefix(Box<str>),

    /// Extension key has multiple lines.
    #[error("map key cannot have multiple lines (newline character `\\n` found)")]
    MultipleLines,
}

/// Implement a number of traits for a newtype of a [`String`] or [`Box<str>`] which will be used
/// as a map key.
///
/// The type must have a `new()` function which returns a [`Result<Self, Error>`].
macro_rules! key_impls {
    ($($Ty:ident => $Error:ty),* $(,)?) => {
        $(
            impl $Ty {
                /// A string slice of the inner value.
                ///
                /// Convenience method for `as_ref()` to a `&str`.
                #[must_use]
                pub fn as_str(&self) -> &str {
                    self.0.as_ref()
                }
            }

            crate::impl_try_from! {
                $Ty::new -> $Error,
                String,
                Box<str>,
                &str,
                ::std::borrow::Cow<'_, str>,
            }

            impl ::std::str::FromStr for $Ty {
                type Err = $Error;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    s.try_into()
                }
            }

            impl AsRef<str> for $Ty {
                fn as_ref(&self) -> &str {
                    self.as_str()
                }
            }

            impl ::std::borrow::Borrow<str> for $Ty {
                fn borrow(&self) -> &str {
                    self.as_str()
                }
            }

            impl ::std::cmp::PartialEq<str> for $Ty {
                fn eq(&self, other: &str) -> bool {
                    self.as_str().eq(other)
                }
            }

            impl ::std::cmp::PartialEq<&str> for $Ty {
                fn eq(&self, other: &&str) -> bool {
                    self.as_str().eq(*other)
                }
            }

            impl ::std::fmt::Display for $Ty {
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    f.write_str(self.as_str())
                }
            }

            impl From<$Ty> for Box<str> {
                fn from(value: $Ty) -> Self {
                    value.0.into()
                }
            }

            impl From<$Ty> for String {
                fn from(value: $Ty) -> Self {
                    value.0.into()
                }
            }
        )*
    };
}

pub(crate) use key_impls;

key_impls! {
    Identifier => InvalidIdentifierError,
    MapKey => InvalidMapKeyError,
    ExtensionKey => InvalidExtensionKeyError,
}

#[cfg(test)]
mod tests {
    use proptest::{
        arbitrary::Arbitrary,
        strategy::{BoxedStrategy, Strategy},
    };

    use super::*;

    impl Arbitrary for Identifier {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            "[a-zA-Z0-9][a-zA-Z0-9._-]*"
                .prop_map_into()
                .prop_map(Self)
                .boxed()
        }
    }
}
