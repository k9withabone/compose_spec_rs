//! Provides [`Tag`] for validating an [`Image`](super::Image)'s tag.

use std::{
    borrow::Borrow,
    fmt::{self, Display, Formatter},
};

use thiserror::Error;

/// Validated [`Image`](super::Image) tag.
///
/// Image tags must:
///
/// - Not be empty.
/// - Only contain ASCII letters (a-z, A-Z), digits (0-9), dots (.), underscores (_), or dashes (-).
/// - Be less than or equal to 128 characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag<'a>(&'a str);

impl<'a> Tag<'a> {
    /// Create an [`Image`](super::Image) [`Tag`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The tag contains a character other than an ASCII letter (a-z, A-Z), digit (0-9), dot (.),
    ///   underscore (_), or dash (-).
    /// - The tag is empty.
    /// - The tag was longer than 128 characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::Tag;
    ///
    /// let tag = Tag::new("latest").unwrap();
    /// assert_eq!(tag, "latest");
    ///
    /// // Non-ASCII characters are not allowed in tags.
    /// assert!(Tag::new("cliché").is_err());
    /// ```
    pub fn new(tag: &'a str) -> Result<Self, InvalidTagError> {
        // See the OCI distribution spec for details:
        // https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests

        for char in tag.chars() {
            if !matches!(char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-') {
                return Err(InvalidTagError::Character(char));
            }
        }

        if tag.is_empty() {
            Err(InvalidTagError::Empty)
        } else if tag.len() > 128 {
            // tag.len() == tag.chars().count() because ASCII only
            Err(InvalidTagError::Length)
        } else {
            Ok(Self(tag))
        }
    }

    /// Create a [`Tag`] without checking the validity.
    pub(super) const fn new_unchecked(tag: &'a str) -> Self {
        Self(tag)
    }

    /// Return the inner string slice.
    #[must_use]
    pub fn into_inner(self) -> &'a str {
        self.0
    }
}

/// Error returned when validating an [`Image`](super::Image) [`Tag`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidTagError {
    /// Tags must only contain ASCII letters (a-z, A-Z), digits (0-9), dots (.), underscores (_),
    /// and dashes (-).
    #[error(
        "image tag contains invalid character '{0}', tags must only contain \
        ASCII letters (a-z, A-Z), digits (0-9), dots (.), underscores (_), and dashes (-)"
    )]
    Character(char),

    /// Tag was empty.
    #[error("image tag cannot be empty")]
    Empty,

    /// Tag was longer than 128 characters.
    #[error("image tags can only be up to 128 characters long")]
    Length,
}

impl<'a> AsRef<str> for Tag<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> Borrow<str> for Tag<'a> {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl<'a> TryFrom<&'a str> for Tag<'a> {
    type Error = InvalidTagError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> PartialEq<str> for Tag<'a> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<'a> PartialEq<&str> for Tag<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl<'a> Display for Tag<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use proptest::proptest;

    use super::*;

    proptest! {
        #[test]
        fn no_panic(tag: String) {
            let _ = Tag::new(&tag);
        }

        /// Test [`Tag`] creation for all possible valid tags.
        ///
        /// Regex is from the
        /// [OCI distribution spec](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests).
        #[test]
        fn new(tag in "[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}") {
            Tag::new(&tag)?;
        }
    }
}
