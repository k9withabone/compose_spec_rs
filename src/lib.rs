//! Types for (de)serializing from/to the
//! [compose-spec](https://github.com/compose-spec/compose-spec). The types are validated while they
//! are deserialized when possible.
//!
//! Note that the [`Deserialize`] implementations of many types make use of
//! [`Deserializer::deserialize_any()`](::serde::de::Deserializer::deserialize_any). This means that
//! you should only attempt to deserialize them from self-describing formats like YAML or JSON.
//!
//! Lists that must contain unique values use [`IndexSet`](indexmap::IndexSet) otherwise they are
//! [`Vec`]s.

mod common;
pub mod config;
pub mod duration;
mod include;
mod name;
pub mod network;
pub mod secret;
mod serde;
pub mod service;
mod volume;

use std::path::PathBuf;

use ::serde::{Deserialize, Serialize};
use indexmap::IndexMap;

pub use self::{
    common::{
        AsShort, AsShortIter, ExtensionKey, Extensions, Identifier, InvalidExtensionKeyError,
        InvalidIdentifierError, InvalidMapKeyError, ItemOrList, ListOrMap, Map, MapKey, Number,
        ParseNumberError, Resource, ShortOrLong, StringOrNumber, TryFromNumberError,
        TryFromValueError, Value, YamlValue,
    },
    config::Config,
    include::Include,
    name::{InvalidNameError, Name},
    network::Network,
    secret::Secret,
    service::Service,
    volume::Volume,
};

/// Named networks which allow for [`Service`]s to communicate with each other.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
pub type Networks = IndexMap<Identifier, Option<Resource<Network>>>;

/// Named volumes which can be reused across multiple [`Service`]s.
///
/// Volumes are persistent data stores implemented by the container engine.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md)
pub type Volumes = IndexMap<Identifier, Option<Resource<Volume>>>;

/// Configs allow [`Service`]s to adapt their behavior without needing to rebuild the container
/// image.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
pub type Configs = IndexMap<Identifier, Resource<Config>>;

/// Sensitive data that a [`Service`] may be granted access to.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/09-secrets.md)
pub type Secrets = IndexMap<Identifier, Resource<Secret>>;

/// The Compose file is a YAML file defining a containers based application.
///
/// Note that the [`Deserialize`] implementations of many types within `Compose` make use of
/// [`Deserializer::deserialize_any()`](::serde::de::Deserializer::deserialize_any). This means that
/// you should only attempt to deserialize from self-describing formats like YAML or JSON.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/03-compose-file.md)
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Compose {
    /// Declared for backward compatibility, ignored.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/04-version-and-name.md#version-top-level-element)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Define the Compose project name, until user defines one explicitly.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/04-version-and-name.md#name-top-level-element)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,

    /// Compose sub-projects to be included.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<ShortOrLong<PathBuf, Include>>,

    /// The [`Service`]s (containerized computing components) of the application.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md)
    pub services: IndexMap<Identifier, Service>,

    /// Named networks for [`Service`]s to communicate with each other.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
    #[serde(default, skip_serializing_if = "Networks::is_empty")]
    pub networks: Networks,

    /// Named volumes which can be reused across multiple [`Service`]s.
    ///
    /// Volumes are persistent data stores implemented by the container engine.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md)
    #[serde(default, skip_serializing_if = "Volumes::is_empty")]
    pub volumes: Volumes,

    /// Configs allow [`Service`]s to adapt their behavior without needing to rebuild the container
    /// image.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
    #[serde(default, skip_serializing_if = "Configs::is_empty")]
    pub configs: Configs,

    /// Sensitive data that a [`Service`] may be granted access to.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/09-secrets.md)
    #[serde(default, skip_serializing_if = "Secrets::is_empty")]
    pub secrets: Secrets,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Implement [`From`] for `Ty` using `f`.
macro_rules! impl_from {
    ($Ty:ident::$f:ident, $($From:ty),+ $(,)?) => {
        $(
            impl From<$From> for $Ty {
                fn from(value: $From) -> Self {
                    Self::$f(value)
                }
            }
        )+
    };
}

use impl_from;

/// Implement [`TryFrom`] for `Ty` using `f` which returns [`Result<Ty, Error>`].
macro_rules! impl_try_from {
    ($Ty:ident::$f:ident -> $Error:ty, $($From:ty),+ $(,)?) => {
        $(
            impl TryFrom<$From> for $Ty {
                type Error = $Error;

                fn try_from(value: $From) -> Result<Self, Self::Error> {
                    Self::$f(value)
                }
            }
        )+
    };
}

use impl_try_from;

/// Implement string conversion traits for types which have a `parse` method.
///
/// For types with an error, the macro creates implementations of:
///
/// - [`FromStr`]
/// - [`TryFrom<&str>`]
/// - [`TryFrom<String>`]
/// - [`TryFrom<Box<str>>`]
/// - [`TryFrom<Cow<str>>`]
///
/// For types without an error, the macro creates implementations of:
///
/// - [`FromStr`], where `Err` is [`Infallible`](std::convert::Infallible)
/// - [`From<&str>`]
/// - [`From<String>`]
/// - [`From<Box<str>>`]
/// - [`From<Cow<str>>`]
///
/// [`FromStr`]: std::str::FromStr
macro_rules! impl_from_str {
    ($($Ty:ident => $Error:ty),* $(,)?) => {
        $(
            impl std::str::FromStr for $Ty {
                type Err = $Error;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Self::parse(s)
                }
            }

            crate::impl_try_from! {
                $Ty::parse -> $Error,
                &str,
                String,
                Box<str>,
                std::borrow::Cow<'_, str>,
            }
        )*
    };
    ($($Ty:ident),* $(,)?) => {
        $(
            impl std::str::FromStr for $Ty {
                type Err = std::convert::Infallible;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self::parse(s))
                }
            }

            crate::impl_from!($Ty::parse, &str, String, Box<str>, std::borrow::Cow<'_, str>);
        )*
    };
}

use impl_from_str;
