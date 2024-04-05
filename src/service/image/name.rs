//! Provides [`Name`] for validating the name portion of an [`Image`](super::Image).

use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
};

use thiserror::Error;

use super::char_is_alnum;

/// Validated name for a container [`Image`](super::Image).
///
/// A name may or may not contain a registry. Some container engines, like
/// docker, use a default registry (e.g. "docker.io") or can be configured with one. It is often
/// recommended to use a full name with a registry for both performance reasons and clarity.
///
/// When validating a name, it is split into parts by splitting on slash (/) characters, then each
/// part is validated. If the name contains more than one part, and first part contains a dot (.)
/// character, it is treated as the registry.
///
/// Image name parts must:
///
/// - Not have more than one separator (., _, __, any number of -) in a row.
/// - Only contain lowercase ASCII letters (a-z), digits (0-9), dashes (-), dots (.),
///   or underscores (_).
/// - Not be empty.
/// - Start and end with a lowercase ASCII letter (a-z) or digit (0-9).
#[derive(Debug, Clone, Copy, Eq)]
pub struct Name<'a> {
    /// Inner string slice.
    inner: &'a str,

    /// Byte position of `inner` where the registry ends, if the image name has a registry part.
    registry_end: Option<usize>,
}

impl<'a> Name<'a> {
    /// Validate a [`Name`].
    ///
    /// The name is split into parts by splitting on slash (/) characters, then each part is
    /// validated. If the name contains more than one part, and first part contains a dot (.)
    /// character, it is treated as the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The part has more than one separator (., _, __, any number of -) in a row.
    /// - The part contains a character other than a lowercase ASCII letter (a-z), digit (0-9),
    ///   dash (-), dot (.), or underscore (_).
    /// - The part is empty.
    /// - The part starts or ends with a character other than a lowercase ASCII letter (a-z)
    ///   or digit (0-9).
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::Name;
    ///
    /// let name = Name::new("library/postgres").unwrap();
    /// assert_eq!(name, "library/postgres");
    /// assert!(name.registry().is_none());
    ///
    /// let name = Name::new("quay.io/podman/hello").unwrap();
    /// assert_eq!(name.registry().unwrap(), "quay.io");
    ///
    /// // Non-ASCII characters are not allowed in image names.
    /// assert!(Name::new("cliché").is_err());
    /// ```
    pub fn new(name: &'a str) -> Result<Self, InvalidNamePartError> {
        let mut split = name.split('/');

        let mut registry_end = None;
        if let Some(first) = split.next() {
            validate_part(first)?;
            if let Some(second) = split.next() {
                validate_part(second)?;
                if first.contains('.') {
                    registry_end = Some(first.len());
                }
            }
        }

        for part in split {
            validate_part(part)?;
        }

        Ok(Self {
            inner: name,
            registry_end,
        })
    }

    /// Create a [`Name`] without checking the validity.
    pub(super) const fn new_unchecked(name: &'a str, registry_end: Option<usize>) -> Self {
        Self {
            inner: name,
            registry_end,
        }
    }

    /// Byte position of `inner` where the registry ends, if the image name has a registry part.
    pub(super) const fn registry_end(&self) -> Option<usize> {
        self.registry_end
    }

    /// Return the registry part of the inner string slice if the name has a registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::Name;
    ///
    /// let name = Name::new("quay.io/podman/hello").unwrap();
    /// assert_eq!(name.registry().unwrap(), "quay.io");
    /// ```
    #[must_use]
    pub fn registry(&self) -> Option<&str> {
        self.registry_end.map(|end| {
            // PANIC_SAFETY:
            // `registry_end` is always within `inner`.
            // `inner` only contains ASCII.
            // Checked with `registry()` test.
            #[allow(clippy::indexing_slicing, clippy::string_slice)]
            &self.inner[..end]
        })
    }

    /// Return the inner string slice.
    #[must_use]
    pub const fn into_inner(self) -> &'a str {
        self.inner
    }
}

/// Validate a `part` of an [`Image`](super::Image) name.
///
/// Image names (image without tag or digest) can contain one or more parts separated by '/'.
///
/// See the [OCI distribution spec](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests)
/// for details.
///
/// # Errors
///
/// Returns an error if:
///
/// - The part has more than one separator (., _, __, any number of -) in a row.
/// - The part contains a character other than a lowercase ASCII letter (a-z), digit (0-9),
///   dash (-), dot (.), or underscore (_).
/// - The part is empty.
/// - The part starts or ends with a character other than a lowercase ASCII letter (a-z)
///   or digit (0-9).
fn validate_part(part: &str) -> Result<(), InvalidNamePartError> {
    let mut dots: u8 = 0;
    let mut underscores: u8 = 0;
    let mut prev_char_dash = false;
    part.chars().try_for_each(|char| match char {
        'a'..='z' | '0'..='9' => {
            dots = 0;
            underscores = 0;
            prev_char_dash = false;
            Ok(())
        }
        '-' => {
            if dots == 0 && underscores == 0 {
                prev_char_dash = true;
                Ok(())
            } else {
                Err(InvalidNamePartError::MultipleSeparators)
            }
        }
        '.' => {
            dots += 1;
            if dots == 1 && underscores == 0 && !prev_char_dash {
                prev_char_dash = false;
                Ok(())
            } else {
                Err(InvalidNamePartError::MultipleSeparators)
            }
        }
        '_' => {
            underscores += 1;
            if dots == 0 && underscores <= 2 && !prev_char_dash {
                prev_char_dash = false;
                Ok(())
            } else {
                Err(InvalidNamePartError::MultipleSeparators)
            }
        }
        char => Err(InvalidNamePartError::Character(char)),
    })?;

    if part.is_empty() {
        // empty means multiple '/' in a row
        Err(InvalidNamePartError::Empty)
    } else if !part.starts_with(char_is_alnum) {
        Err(InvalidNamePartError::Start)
    } else if !part.ends_with(char_is_alnum) {
        Err(InvalidNamePartError::End)
    } else {
        Ok(())
    }
}

/// Error returned when validating a part of an [`Image`](super::Image) [`Name`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidNamePartError {
    /// Part had more than one separator (., _, __, any number of -) in a row.
    #[error("image name parts may only have one separator (., _, __, any number of -) in a row")]
    MultipleSeparators,

    /// Name parts must only contain lowercase ASCII letters (a-z), digits (0-9), dashes (-),
    /// dots (.), and underscores (_).
    #[error(
        "invalid character in image name '{0}', name parts must only contain \
        lowercase ASCII letters (a-z), digits (0-9), dashes (-), dots (.), and underscores (_)"
    )]
    Character(char),

    /// Name part was empty.
    #[error(
        "a part of an image name cannot be empty, i.e. there were two slashes (/) in row, \
        or the image name was completely empty"
    )]
    Empty,

    /// Name parts must start with a lowercase ASCII letter (a-z) or a digit (0-9).
    #[error("image name parts must start with a lowercase ASCII letter (a-z) or a digit (0-9)")]
    Start,

    /// Name parts must end with a lowercase ASCII letter (a-z) or a digit (0-9).
    #[error("image name parts must end with a lowercase ASCII letter (a-z) or a digit (0-9)")]
    End,
}

impl<'a> AsRef<str> for Name<'a> {
    fn as_ref(&self) -> &str {
        self.inner
    }
}

impl<'a> Borrow<str> for Name<'a> {
    fn borrow(&self) -> &str {
        self.inner
    }
}

impl<'a> TryFrom<&'a str> for Name<'a> {
    type Error = InvalidNamePartError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> PartialEq for Name<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(other.inner)
    }
}

impl<'a> PartialEq<str> for Name<'a> {
    fn eq(&self, other: &str) -> bool {
        self.inner == other
    }
}

impl<'a> PartialEq<&str> for Name<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.inner == *other
    }
}

impl<'a> PartialOrd for Name<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Name<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(other.inner)
    }
}

impl<'a> Hash for Name<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<'a> Display for Name<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use pomsky_macro::pomsky;
    use proptest::{prop_assert_eq, proptest};

    use super::*;

    const NAME: &str = pomsky! {
        let end = [ascii_lower ascii_digit]+;
        let separator = '.' | '_' | "__" | '-'+;
        let part = end (separator end)*;

        part ('/' part)*
    };

    const REGISTRY: &str = pomsky! {
        let end = [ascii_lower ascii_digit]+;
        let separator = '.' | '_' | "__" | '-'+;
        let part = end (separator end)*;

        part '.' part
    };

    proptest! {
        #[test]
        fn no_panic(name: String) {
            let _ = Name::new(&name);
        }

        /// Test [`Name`] creation for all possible valid names.
        ///
        /// Regex is from the
        /// [OCI distribution spec](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests).
        #[test]
        #[ignore]
        fn new(name in NAME) {
            Name::new(&name)?;
        }

        /// Test `registry_end` is accurately parsed.
        #[test]
        #[ignore]
        fn registry(registry in REGISTRY, rest in NAME) {
            let name = format!("{registry}/{rest}");
            let name = Name::new(&name)?;
            prop_assert_eq!(name.registry(), Some(registry.as_str()));
        }
    }
}
