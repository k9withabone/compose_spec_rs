//! RFC 1123 compliant [`Hostname`].

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

use crate::common::key_impls;

/// An RFC 1123 compliant hostname.
///
/// Hostnames must only contain contain ASCII letters (a-z, A-Z), digits (0-9), dots (.), and
/// dashes (-). Hostnames are split on dots (.) into labels. Each label must not be empty and cannot
/// start or end with a dash (-).
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Hostname(Box<str>);

impl Hostname {
    /// Create a new [`Hostname`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the hostname contains a character other than ASCII letters (a-z, A-Z),
    /// digits (0-9), dots (.), and dashes (-), a label is empty, or a label starts or ends with a
    /// dash (-).
    pub fn new<T>(hostname: T) -> Result<Self, InvalidHostnameError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let hostname_str = hostname.as_ref();

        for label in hostname_str.split('.') {
            if label.is_empty() {
                return Err(InvalidHostnameError::LabelEmpty);
            }
            for char in label.chars() {
                if !char.is_ascii_alphanumeric() && char != '-' {
                    return Err(InvalidHostnameError::Character(char));
                }
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(InvalidHostnameError::LabelStartEnd);
            }
        }

        Ok(Self(hostname.into()))
    }
}

/// Error returned when creating a [`Hostname`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidHostnameError {
    /// One of the hostname's labels was empty.
    #[error("hostname label was empty")]
    LabelEmpty,

    /// Hostnames can only contain ASCII letters (a-z, A-Z), digits (0-9), dots (.), and dashes (-).
    #[error(
        "invalid hostname character `{0}`, hostnames can only contain ASCII letters (a-z, A-Z), \
            digits (0-9), dots (.), and dashes (-)"
    )]
    Character(char),

    /// Hostname labels cannot start or end with a dash (-).
    #[error("hostname labels cannot start or end with dashes (-)")]
    LabelStartEnd,
}

key_impls!(Hostname => InvalidHostnameError);

#[cfg(test)]
mod tests {
    use pomsky_macro::pomsky;
    use proptest::proptest;

    use super::*;

    const HOSTNAME: &str = pomsky! {
        let end = [ascii_alnum];
        let middle = [ascii_alnum '-']*;
        let label = end (middle end)?;

        label ('.' label)*
    };

    proptest! {
        #[test]
        fn no_panic(string: String) {
            let _ = Hostname::new(string);
        }

        #[test]
        fn valid(string in HOSTNAME) {
            Hostname::new(string)?;
        }
    }
}
