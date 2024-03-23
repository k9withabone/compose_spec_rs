//! Provides [`Network`] for the top-level `networks` field of a [`Compose`](super::Compose) file.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    net::IpAddr,
    ops::Not,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexMap;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::{
    impl_from_str, service::Hostname, Extensions, ListOrMap, MapKey, Resource, StringOrNumber,
};

impl Resource<Network> {
    /// Custom network name, if set.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#name)
    #[must_use]
    pub fn name(&self) -> Option<&String> {
        match self {
            Self::External { name } => name.as_ref(),
            Self::Compose(network) => network.name.as_ref(),
        }
    }
}

impl From<Network> for Resource<Network> {
    fn from(value: Network) -> Self {
        Self::Compose(value)
    }
}

/// A named network which allows for [`Service`](super::Service)s to communicate with each other.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Network {
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

    /// Custom IPAM configuration.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#ipam)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipam: Option<Ipam>,

    /// Whether to isolate this network from external connectivity.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#internal)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub internal: bool,

    /// Add metadata to the network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#labels)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Custom name for the network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

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

/// IP address management (IPAM) options for a [`Network`] [`Config`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#ipam)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Ipam {
    /// Custom IPAM driver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// IPAM configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<IpamConfig>,

    /// Driver-specific options.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub options: IndexMap<MapKey, String>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// [`Ipam`] configuration.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#ipam)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct IpamConfig {
    /// Network subnet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subnet: Option<IpNet>,

    /// Range of IPs from which to allocate container IPs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_range: Option<IpNet>,

    /// IPv4 or IPv6 gateway for the subnet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway: Option<IpAddr>,

    /// Auxiliary IPv4 or IPv6 addresses used by [`Network`] driver, as a mapping from hostnames to
    /// IP addresses.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub aux_addresses: IndexMap<Hostname, IpAddr>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
