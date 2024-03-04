//! Provides [`Digest`] for validating an [`Image`](super::Image)'s digest.

use std::{
    borrow::Borrow,
    fmt::{self, Display, Formatter},
};

use thiserror::Error;

use super::char_is_alnum;

/// Validated [`Image`](super::Image) digest.
///
/// Image digests have an algorithm and encoded data.
///
/// The algorithm must:
///
/// - Not contain more than one separator (+._-) in a row.
/// - Only contain lowercase ASCII letters (a-z), digits (0-9), or separators (+._-).
/// - Start and end with a lowercase ASCII letter (a-z) or digit (0-9).
///
/// The encoded data must:
///
/// - Not be empty.
/// - Only contain ASCII letters (a-z, A-Z), digits (0-9), equals (=), underscores (_),
///   or dashes (-).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest<'a>(&'a str);

impl<'a> Digest<'a> {
    /// Create an [`Image`](super::Image) [`Digest`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The algorithm is missing or empty.
    /// - The algorithm contains more than one separators (+._-) in a row.
    /// - The algorithm contains a character other than a lowercase ASCII letter (a-z), digit (0-9),
    ///   or separator (+._-).
    /// - The algorithm starts or ends with a character other than a lowercase ASCII letter (a-z) or
    ///   digit (0-9).
    /// - The encoded data contains a character other than an ASCII letter (a-z, A-Z), digit (0-9),
    ///   equals (=), underscore (_), or dash (-).
    /// - The encoded data is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::Digest;
    ///
    /// let string = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// let digest = Digest::new(string).unwrap();
    /// assert_eq!(digest, string);
    /// ```
    pub fn new(digest: &'a str) -> Result<Self, InvalidDigestError> {
        // See the OCI image spec for details:
        // https://github.com/opencontainers/image-spec/blob/main/descriptor.md#digests

        let (algorithm, encoded) = digest
            .split_once(':')
            .ok_or(InvalidDigestError::MissingAlgorithm)?;

        let mut separators: u8 = 0;
        for char in algorithm.chars() {
            match char {
                'a'..='z' | '0'..='9' => {
                    separators = 0;
                    Ok(())
                }
                '+' | '.' | '_' | '-' => {
                    separators += 1;
                    if separators <= 1 {
                        Ok(())
                    } else {
                        Err(InvalidDigestError::AlgorithmSeparators)
                    }
                }
                char => Err(InvalidDigestError::AlgorithmCharacter(char)),
            }?;
        }

        for char in encoded.chars() {
            if !matches!(char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '=' | '_' | '-') {
                return Err(InvalidDigestError::EncodeCharacter(char));
            }
        }

        if algorithm.is_empty() {
            Err(InvalidDigestError::MissingAlgorithm)
        } else if !algorithm.starts_with(char_is_alnum) {
            Err(InvalidDigestError::AlgorithmStart)
        } else if !algorithm.ends_with(char_is_alnum) {
            Err(InvalidDigestError::AlgorithmEnd)
        } else if encoded.is_empty() {
            Err(InvalidDigestError::EncodeEmpty)
        } else {
            Ok(Self(digest))
        }
    }

    /// Create a [`Digest`] without checking the validity.
    pub(super) const fn new_unchecked(digest: &'a str) -> Self {
        Self(digest)
    }

    /// Return the inner string slice.
    #[must_use]
    pub fn into_inner(self) -> &'a str {
        self.0
    }
}

/// Error returned when validating an [`Image`](super::Image) [`Digest`].
#[derive(Error, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidDigestError {
    /// Digest was missing its algorithm part.
    #[error("image digest is missing its algorithm")]
    MissingAlgorithm,

    /// Digest algorithm had multiple separators (+._-) in a row.
    #[error("image digest algorithms may only have one separator (+._-) in a row")]
    AlgorithmSeparators,

    /// Digests may only contain lowercase ASCII letters (a-z), digits (0-9), or separators (+._-).
    #[error(
        "invalid character '{0}' in image digest algorithm,
        digests may only contain lowercase ASCII letters (a-z), digits (0-9), or separators (+._-)"
    )]
    AlgorithmCharacter(char),

    /// Data encoded in digest may only contain ASCII letters (a-z, A-Z), digits (0-9), equals (=),
    /// underscores (_), and dashes (-).
    #[error(
        "invalid character '{0}' in image digest encode, data encoded in digest may only contain \
        ASCII letters (a-z, A-Z), digits (0-9), equals (=), underscores (_), and dashes (-)"
    )]
    EncodeCharacter(char),

    /// Digest algorithms must start with a lowercase ASCII letter (a-z) or a digit (0-9).
    #[error(
        "image digest algorithm must start with a lowercase ASCII letter (a-z) or a digit (0-9)"
    )]
    AlgorithmStart,

    /// Digest algorithms must end with a lowercase ASCII letter (a-z) or a digit (0-9).
    #[error(
        "image digest algorithm must end with a lowercase ASCII letter (a-z) or a digit (0-9)"
    )]
    AlgorithmEnd,

    /// Data encoded in digest was missing.
    #[error("image digest encode is empty")]
    EncodeEmpty,
}

impl<'a> AsRef<str> for Digest<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> Borrow<str> for Digest<'a> {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl<'a> TryFrom<&'a str> for Digest<'a> {
    type Error = InvalidDigestError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> PartialEq<str> for Digest<'a> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<'a> PartialEq<&str> for Digest<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl<'a> Display for Digest<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use pomsky_macro::pomsky;
    use proptest::proptest;

    use super::*;

    const DIGEST: &str = pomsky! {
        let component = [ascii_lower ascii_digit]+;
        let separator = ['+' '.' '_' '-'];
        let algorithm = component (separator component)*;

        let encoded = [ascii_alnum '=' '_' '-']+;

        algorithm ":" encoded
    };

    proptest! {
        #[test]
        fn no_panic(digest: String) {
            let _ = Digest::new(&digest);
        }

        /// Test [`Digest`] creation for all possible valid digests.
        ///
        /// Regex is from the
        /// [OCI image spec](https://github.com/opencontainers/image-spec/blob/main/descriptor.md#digests).
        #[test]
        fn new(digest in DIGEST) {
            Digest::new(&digest)?;
        }
    }
}
