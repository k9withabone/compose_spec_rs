//! Provides [`Network`] for the top-level `networks` field of a [`Compose`](super::Compose) file.

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Not,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexMap;
use serde::{
    de::{self, IntoDeserializer},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{impl_from_str, Extensions, MapKey, StringOrNumber};

/// A named network which allows for [`Service`](super::Service)s to communicate with each other.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
#[derive(Debug, Clone, PartialEq)]
pub enum Network {
    /// Externally managed network.
    ///
    /// (De)serializes from/to the mapping `external: true`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#external)
    External,

    /// Network configuration.
    Config(Config),
}

impl Network {
    /// [`Self::External`] field name.
    const EXTERNAL: &'static str = "external";

    /// Returns `true` if the network is [`External`].
    ///
    /// [`External`]: Network::External
    #[must_use]
    pub fn is_external(&self) -> bool {
        matches!(self, Self::External)
    }

    /// Returns [`Some`] if the network is [`Config`].
    ///
    /// [`Config`]: Network::Config
    #[must_use]
    pub fn as_config(&self) -> Option<&Config> {
        if let Self::Config(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<Config> for Network {
    fn from(value: Config) -> Self {
        Self::Config(value)
    }
}

impl Serialize for Network {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Network::External => {
                let mut state = serializer.serialize_struct("Network", 1)?;
                state.serialize_field(Self::EXTERNAL, &true)?;
                state.end()
            }
            Network::Config(config) => config.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = HashMap::<String, serde_yaml::Value>::deserialize(deserializer)?;

        let external = map
            .remove(Self::EXTERNAL)
            .map(bool::deserialize)
            .transpose()
            .map_err(de::Error::custom)?
            .unwrap_or_default();

        if external {
            if map.is_empty() {
                Ok(Self::External)
            } else {
                Err(de::Error::custom("cannot set `external` and other fields"))
            }
        } else {
            Config::deserialize(map.into_deserializer())
                .map(Self::Config)
                .map_err(de::Error::custom)
        }
    }
}

/// [`Network`] configuration.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename = "Network")]
pub struct Config {
    /// Which driver to use for this network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#driver)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<Driver>,

    /// Driver-dependent options.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#driver_opts)
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub driver_opts: IndexMap<MapKey, StringOrNumber>,

    /// Whether externally managed containers may attach to this network, in addition to
    /// [`Service`](super::Service)s.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#attachable)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub attachable: bool,

    /// Whether to enable IPv6 networking.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#enable_ipv6)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub enable_ipv6: bool,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// [`Network`] driver.
///
/// Default and available values are platform specific.
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum Driver {
    /// Use the host's networking stack.
    Host,
    /// Turn off networking.
    None,
    /// Other network driver.
    Other(String),
}

impl Driver {
    /// [`Self::Host`] string value.
    const HOST: &'static str = "host";

    /// [`Self::None`] string value.
    const NONE: &'static str = "none";

    /// Parse a [`Driver`] from a string.
    pub fn parse<T>(driver: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match driver.as_ref() {
            Self::HOST => Self::Host,
            Self::NONE => Self::None,
            _ => Self::Other(driver.into()),
        }
    }

    /// Network driver as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Host => Self::HOST,
            Self::None => Self::NONE,
            Self::Other(other) => other,
        }
    }
}

impl_from_str!(Driver);

impl AsRef<str> for Driver {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Driver {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Driver> for Cow<'static, str> {
    fn from(value: Driver) -> Self {
        match value {
            Driver::Host => Driver::HOST.into(),
            Driver::None => Driver::NONE.into(),
            Driver::Other(other) => other.into(),
        }
    }
}

impl From<Driver> for String {
    fn from(value: Driver) -> Self {
        Cow::from(value).into_owned()
    }
}
