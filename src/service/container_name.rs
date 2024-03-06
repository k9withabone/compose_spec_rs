//! Provides [`ContainerName`] for the `container_name` field of [`Service`](super::Service).

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

use crate::{common::key_impls, Identifier};

/// A custom container name.
///
/// Container names must be at least 2 characters long, start with an ASCII letter (a-z, A-Z) or
/// digit (0-9), and only contain ASCII letters, digits, underscores (_), dots (.), and dashes (-).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#container_name)
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ContainerName(Box<str>);

impl ContainerName {
    /// Create a new [`ContainerName`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is less than 2 characters long, does not start with an
    /// ASCII letter (a-z, A-Z) or digit (0-9), or contains a character other than an ASCII letter,
    /// digit, underscore (_), dot (.), or dash (-).
    pub fn new<T>(name: T) -> Result<Self, InvalidContainerNameError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let name_str = name.as_ref();
        let mut chars = name_str.chars();

        // Pattern from compose-spec: [a-zA-Z0-9][a-zA-Z0-9_.-]+
        let first = chars.next().ok_or(InvalidContainerNameError::Length)?;
        if !first.is_ascii_alphanumeric() {
            return Err(InvalidContainerNameError::StartCharacter(first));
        }
        for char in chars {
            if !(char.is_ascii_alphanumeric() || matches!(char, '_' | '.' | '-')) {
                return Err(InvalidContainerNameError::Character(char));
            }
        }
        if name_str.len() < 2 {
            // name_str.len() == name_str.chars().count() because it only contains ASCII characters.
            return Err(InvalidContainerNameError::Length);
        }

        Ok(Self(name.into()))
    }
}

/// Error returned when creating a [`ContainerName`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InvalidContainerNameError {
    /// Container names must be at least 2 characters long.
    #[error("container names must be at least 2 characters long")]
    Length,

    /// Container name started with an invalid character.
    ///
    /// Container names must start with an ASCII letter (a-z, A-Z) or a digit (0-9).
    #[error(
        "first character '{0}' invalid, container names must start with \
            an ASCII letter (a-z, A-Z) or a digit (0-9)"
    )]
    StartCharacter(char),

    /// Container name contained an invalid character.
    ///
    /// Container names can only contain ASCII letters (a-z, A-Z), digits (0-9), underscores (_),
    /// dots (.), and dashes (-).
    #[error(
        "character '{0}' invalid, container names can only contain ASCII letters (a-z, A-Z), \
            digits (0-9), underscores (_), dots (.), and dashes (-)"
    )]
    Character(char),
}

impl From<ContainerName> for Identifier {
    fn from(value: ContainerName) -> Self {
        // A `ContainerName` is always a valid `Identifier`.
        Self::new_unchecked(value.0)
    }
}

key_impls!(ContainerName => InvalidContainerNameError);

#[cfg(test)]
mod tests {
    use proptest::proptest;

    use super::*;

    proptest! {
        #[test]
        fn no_panic(name: String) {
            let _ = ContainerName::new(name);
        }

        #[test]
        fn valid(name in "[a-zA-Z0-9][a-zA-Z0-9_.-]+") {
            ContainerName::new(name)?;
        }
    }
}
