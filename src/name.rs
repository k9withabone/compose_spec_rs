//! Provides a validated [`Name`] for [`Compose`](super::Compose) files' top-level `name` field.

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

/// Validated [`Compose`](super::Compose) project name.
///
/// Names cannot be empty, they must start with a lowercase ASCII letter (a-z) or digit (0-9), and
/// must only contain lowercase ASCII letters (a-z), digits (0-9), underscores (_), or dashes (-).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/04-version-and-name.md#name-top-level-element)
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Name(Box<str>);

impl Name {
    /// Create a new [`Name`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid [`Name`]. Names cannot be empty, they
    /// must start with a lowercase ASCII letter (a-z) or digit (0-9), and must only contain
    /// lowercase ASCII letters (a-z), digits (0-9), underscores (_), or dashes (-).
    pub fn new<T>(name: T) -> Result<Self, InvalidNameError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let mut chars = name.as_ref().chars();

        let first = chars.next().ok_or(InvalidNameError::Empty)?;

        // pattern from schema: "^[a-z0-9][a-z0-9_-]*$"
        if !matches!(first, 'a'..='z' | '0'..='9') {
            return Err(InvalidNameError::Start(first));
        }
        for char in chars {
            if !matches!(char, 'a'..='z' | '0'..='9' | '_' | '-') {
                return Err(InvalidNameError::Character(char));
            }
        }

        Ok(Self(name.into()))
    }

    /// [`Name`] as a string slice.
    ///
    /// Convenience method for `as_ref()` to a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// Error returned when attempting to create a [`Name`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidNameError {
    /// Empty name.
    #[error("name cannot be empty")]
    Empty,

    /// Invalid start character.
    ///
    /// Names must start with a lowercase ASCII letter (a-z) or digit (0-9).
    #[error(
        "invalid character `{0}`, names must start with \
            a lowercase ASCII letter (a-z) or digit (0-9)"
    )]
    Start(char),

    /// Invalid character.
    ///
    /// Names must contain only lowercase ASCII letters (a-z), digits (0-9), underscores (_), or
    /// dashes (-).
    #[error(
        "invalid character `{0}`, names must contain only \
            lowercase ASCII letters (a-z), digits (0-9), underscores (_), or dashes (-)"
    )]
    Character(char),
}

impl TryFrom<String> for Name {
    type Error = InvalidNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<Box<str>> for Name {
    type Error = InvalidNameError;

    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Name {
    type Error = InvalidNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for Name {
    type Err = InvalidNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Name> for Box<str> {
    fn from(value: Name) -> Self {
        value.0
    }
}

impl From<Name> for String {
    fn from(value: Name) -> Self {
        value.0.into_string()
    }
}
