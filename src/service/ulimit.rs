//! Provides [`Ulimits`] for the `ulimits` field of [`Service`](super::Service) and the long
//! [`Build`](super::Build) syntax.

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{common::key_impls, AsShort, Extensions, ShortOrLong};

/// Override the default ulimits for a [`Service`](super::Service) container.
///
/// Ulimits are defined as map from a [`Resource`] to either a singe limit ([`u64`]) or a mapping
/// of a soft and hard limit ([`Ulimit`]).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ulimits)
pub type Ulimits = IndexMap<Resource, ShortOrLong<u64, Ulimit>>;

/// [`Ulimit`] resource name (e.g. "nofile").
///
/// Resource names must only contain lowercase ASCII letters (a-z) and cannot be empty.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ulimits)
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Resource(Box<str>);

impl Resource {
    /// Create a new [`Resource`], validating the given string.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid [`Resource`].
    /// Resources must only contain lowercase ASCII letters (a-z) and cannot be empty.
    pub fn new<T>(resource: T) -> Result<Self, InvalidResourceError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let resource_str = resource.as_ref();

        if resource_str.is_empty() {
            return Err(InvalidResourceError::Empty);
        }

        for char in resource_str.chars() {
            if !char.is_ascii_lowercase() {
                return Err(InvalidResourceError::Character(char));
            }
        }

        Ok(Self(resource.into()))
    }
}

/// Error returned when creating a [`Resource`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidResourceError {
    /// Resource was empty.
    #[error("ulimit resources cannot be empty")]
    Empty,

    /// Resource contained an invalid character.
    ///
    /// Ulimit resources can only contain lowercase ASCII letters (a-z).
    #[error(
        "invalid character '{0}', ulimit resources can only contain lowercase ASCII letters (a-z)"
    )]
    Character(char),
}

key_impls!(Resource => InvalidResourceError);

/// Ulimit long syntax, defines a soft and hard limit for a [`Resource`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ulimits)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Ulimit {
    /// Soft limit.
    pub soft: u64,

    /// Hard limit.
    pub hard: u64,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl AsShort for Ulimit {
    type Short = u64;

    fn as_short(&self) -> Option<&Self::Short> {
        let Self {
            soft,
            hard,
            extensions,
        } = self;

        (*soft == *hard && extensions.is_empty()).then_some(soft)
    }
}

impl From<u64> for Ulimit {
    fn from(value: u64) -> Self {
        Self {
            soft: value,
            hard: value,
            extensions: Extensions::default(),
        }
    }
}
