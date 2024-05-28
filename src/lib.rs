//! `compose_spec` is a library for (de)serializing from/to the [Compose specification].
//!
//! This library attempts to make interacting with and creating Compose files as idiomatic and
//! correct as possible.
//!
//! - [`PathBuf`]s are used for fields which denote a path.
//! - Enums are used for fields which conflict with each other.
//! - Values are fully parsed and validated when they have a defined format.
//! - Lists that must contain unique values use [`IndexSet`](indexmap::IndexSet), otherwise they are
//!   [`Vec`]s.
//! - Strings which represent a span of time are converted to/from
//!   [`Duration`](std::time::Duration)s, see the [`duration`] module.
//!
//! Note that the [`Deserialize`] implementations of many types make use of
//! [`Deserializer::deserialize_any()`](::serde::de::Deserializer::deserialize_any). This means that
//! you should only attempt to deserialize them from self-describing formats like YAML or JSON.
//!
//! # Examples
//!
//! ```
//! use compose_spec::{Compose, Service, service::Image};
//!
//! let yaml = "\
//! services:
//!   caddy:
//!     image: docker.io/library/caddy:latest
//!     ports:
//!       - 8000:80
//!       - 8443:443
//!     volumes:
//!       - ./Caddyfile:/etc/caddy/Caddyfile
//!       - caddy-data:/data
//! volumes:
//!   caddy-data:
//! ";
//!
//! // Deserialize `Compose`
//! let compose: Compose = serde_yaml::from_str(yaml)?;
//!
//! // Serialize `Compose`
//! let value = serde_yaml::to_value(&compose)?;
//! # let yaml: serde_yaml::Value = serde_yaml::from_str(yaml)?;
//! # assert_eq!(value, yaml);
//!
//! // Get the `Image` of the "caddy" service
//! let caddy: Option<&Service> = compose.services.get("caddy");
//! let image: &Option<Image> = &caddy.unwrap().image;
//! let image: &Image = image.as_ref().unwrap();
//!
//! assert_eq!(image, "docker.io/library/caddy:latest");
//! assert_eq!(image.name(), "docker.io/library/caddy");
//! assert_eq!(image.tag(), Some("latest"));
//! # Ok::<(), serde_yaml::Error>(())
//! ```
//!
//! # Short or Long Syntax Values
//!
//! Many values within the [Compose specification] can be represented in either a short or long
//! syntax. The enum [`ShortOrLong`] is used to for these values. Conversion from the [`Short`]
//! syntax to the [`Long`] syntax is always possible. The [`AsShort`] trait is used for [`Long`]
//! syntax types which may be represented directly as the [`Short`] syntax type if additional
//! options are not set.
//!
//! [Compose specification]: https://github.com/compose-spec/compose-spec
//! [`Short`]: ShortOrLong::Short
//! [`Long`]: ShortOrLong::Long

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

use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

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

impl Compose {
    /// Ensure that the networks used in each [`Service`] are defined in the `networks` field.
    ///
    /// # Errors
    ///
    /// Returns an error if a [`Service`] uses an [`Identifier`] for a [`Network`] not defined in
    /// the `networks` field.
    ///
    /// Only the first undefined network is listed in the error's [`Display`] output.
    pub fn validate_networks(&self) -> Result<(), ValidationError> {
        for (name, service) in &self.services {
            service
                .validate_networks(&self.networks)
                .map_err(|resource| ValidationError {
                    service: Some(name.clone()),
                    resource,
                    kind: ResourceKind::Network,
                })?;
        }

        Ok(())
    }

    /// Ensure that named volumes used across multiple [`Service`]s are defined in the `volumes`
    /// field.
    ///
    /// # Errors
    ///
    /// Returns an  error if a named volume [`Identifier`] is used across multiple [`Service`]s is
    /// not defined in the `volumes` field.
    ///
    /// Only the first undefined named volume is listed in the error's [`Display`] output.
    pub fn validate_volumes(&self) -> Result<(), ValidationError> {
        let volumes = self
            .services
            .values()
            .flat_map(|service| service::volumes::named_volumes_iter(&service.volumes));

        let mut seen_volumes = HashMap::new();
        for volume in volumes {
            match seen_volumes.entry(volume) {
                Entry::Occupied(mut entry) => {
                    if !entry.get() && !self.volumes.contains_key(volume) {
                        return Err(ValidationError {
                            service: None,
                            resource: volume.clone(),
                            kind: ResourceKind::Volume,
                        });
                    }
                    *entry.get_mut() = true;
                }
                Entry::Vacant(entry) => {
                    entry.insert(false);
                }
            }
        }

        Ok(())
    }
}

/// Error returned when validation of a [`Compose`] file fails.
///
/// Occurs when a [`Service`] uses a [`Resource`] which is not defined in the corresponding
/// field in the [`Compose`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Name of the [`Service`] which uses the invalid `resource`.
    service: Option<Identifier>,
    /// Name of the resource which is not defined by the [`Compose`] file.
    resource: Identifier,
    /// The kind of the `resource`.
    kind: ResourceKind,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            service,
            resource,
            kind,
        } = self;

        write!(f, "{kind} `{resource}` ")?;

        if let Some(service) = service {
            write!(f, "(used in the `{service}` service) ")?;
        }

        if matches!(kind, ResourceKind::Volume) {
            write!(f, "is used across multiple services and ")?;
        }

        write!(f, "is not defined in the top-level `{kind}s` field")
    }
}

impl Error for ValidationError {}

/// Kinds of [`Resource`]s that may be used in a [`ValidationError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    /// [`Network`] resource kind.
    Network,
    /// [`Volume`] resource kind.
    Volume,
}

impl ResourceKind {
    /// Resource kind as a static string slice.
    #[must_use]
    const fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Volume => "volume",
        }
    }
}

impl Display for ResourceKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
            impl ::std::str::FromStr for $Ty {
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
                ::std::borrow::Cow<'_, str>,
            }
        )*
    };
    ($($Ty:ident),* $(,)?) => {
        $(
            impl ::std::str::FromStr for $Ty {
                type Err = std::convert::Infallible;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self::parse(s))
                }
            }

            crate::impl_from!($Ty::parse, &str, String, Box<str>, ::std::borrow::Cow<'_, str>);
        )*
    };
}

use impl_from_str;

#[cfg(test)]
mod tests {
    use indexmap::{indexmap, indexset};

    use self::service::volumes::{ShortOptions, ShortVolume};

    use super::*;

    #[test]
    fn full_round_trip() -> serde_yaml::Result<()> {
        let yaml = include_str!("test-full.yaml");

        let compose: Compose = serde_yaml::from_str(yaml)?;

        assert_eq!(
            serde_yaml::from_str::<serde_yaml::Value>(yaml)?,
            serde_yaml::to_value(compose)?,
        );

        Ok(())
    }

    #[test]
    fn validate_networks() -> Result<(), InvalidIdentifierError> {
        let test = Identifier::new("test")?;
        let network = Identifier::new("network")?;

        let service = Service {
            network_config: Some(service::NetworkConfig::Networks(
                indexset![network.clone()].into(),
            )),
            ..Service::default()
        };

        let mut compose = Compose {
            services: indexmap! {
                test.clone() => service,
            },
            ..Compose::default()
        };
        assert_eq!(
            compose.validate_networks(),
            Err(ValidationError {
                service: Some(test),
                resource: network.clone(),
                kind: ResourceKind::Network
            })
        );

        compose.networks.insert(network, None);
        assert_eq!(compose.validate_networks(), Ok(()));

        Ok(())
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    fn validate_volumes() {
        let volume_id = Identifier::new("volume").unwrap();
        let volume = ShortVolume {
            container_path: PathBuf::from("/container").try_into().unwrap(),
            options: Some(ShortOptions::new(volume_id.clone().into())),
        };
        let service = Service {
            volumes: indexset![volume.into()],
            ..Service::default()
        };

        let mut compose = Compose {
            services: indexmap! {
                Identifier::new("one").unwrap() => service.clone(),
            },
            ..Compose::default()
        };

        assert_eq!(compose.validate_volumes(), Ok(()));

        compose
            .services
            .insert(Identifier::new("two").unwrap(), service);
        let error = Err(ValidationError {
            service: None,
            resource: volume_id.clone(),
            kind: ResourceKind::Volume,
        });
        assert_eq!(compose.validate_volumes(), error);

        let volume = compose.services[1].volumes.pop().unwrap();
        compose.services[1]
            .volumes
            .insert(volume.into_long().into());
        assert_eq!(compose.validate_volumes(), error);

        compose.volumes.insert(volume_id, None);
        assert_eq!(compose.validate_volumes(), Ok(()));
    }
}
