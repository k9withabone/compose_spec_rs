//! Provides [`NetworkConfig`] and [`MacAddress`] for the `network_config` and `mac_address` fields
//! of [`Service`](super::Service).

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter, LowerHex},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    num::ParseIntError,
    str::{FromStr, Split},
};

use compose_spec_macros::{DeserializeFromStr, DeserializeTryFromString, SerializeDisplay};
use indexmap::{map::Keys, IndexMap, IndexSet};
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{
    impl_from_str, AsShortIter, Extensions, Identifier, InvalidIdentifierError, ShortOrLong,
};

use super::Hostname;

/// [`Network`](crate::Network)s that a [`Service`](super::Service) container is attached to.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#networks)
pub type Networks = ShortOrLong<IndexSet<Identifier>, IndexMap<Identifier, Option<Network>>>;

/// Network configuration for a [`Service`] container.
///
/// (De)serializes from/to a struct with either a `network_mode` or `networks` field, which is
/// flattened into [`Service`].
///
/// [`Service`]: super::Service
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkConfig {
    /// [`Service`](super::Service) container's network mode.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#network_mode)
    NetworkMode(NetworkMode),

    /// [`Network`](crate::Network)s that a [`Service`](super::Service) container is attached to.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#networks)
    Networks(Networks),
}

impl NetworkConfig {
    /// Struct name for (de)serializing.
    const NAME: &'static str = "NetworkConfig";
}

impl From<NetworkMode> for NetworkConfig {
    fn from(value: NetworkMode) -> Self {
        Self::NetworkMode(value)
    }
}

impl From<Networks> for NetworkConfig {
    fn from(value: Networks) -> Self {
        Self::Networks(value)
    }
}

/// Possible [`NetworkConfig`] fields.
#[derive(Debug, Clone, Copy)]
enum Field {
    /// [`NetworkConfig::NetworkMode`] / `network_mode`
    NetworkMode,

    /// [`NetworkConfig::Networks`] / `networks`
    Networks,
}

impl Field {
    /// Field identifier as a static string slice.
    const fn as_str(self) -> &'static str {
        match self {
            Self::NetworkMode => "network_mode",
            Self::Networks => "networks",
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// [`format_args`] with all [`Field`]s.
macro_rules! format_fields {
    ($args:literal) => {
        format_args!($args, Field::NetworkMode, Field::Networks)
    };
}

impl Serialize for NetworkConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(Self::NAME, 1)?;

        match self {
            Self::NetworkMode(network_mode) => {
                state.serialize_field(Field::NetworkMode.as_str(), network_mode)?;
            }
            Self::Networks(networks) => {
                state.serialize_field(Field::Networks.as_str(), networks)?;
            }
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for NetworkConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        option::deserialize(deserializer)?
            .ok_or_else(|| de::Error::custom(format_fields!("missing required field `{}` or `{}`")))
    }
}

/// (De)serialize [`Option<NetworkConfig>`], for use in `#[serde(with = "option")]`.
///
/// For deserialization, the following is returned:
///
/// - `Ok(Some(NetworkConfig::NetworkMode(_)))`, if given a struct/map with a `network_mode` field.
/// - `Ok(Some(NetworkConfig::Networks(_)))`, if given a struct/map with a `networks` field.
/// - `Ok(None)`, if neither the `network_mode` or `networks` fields are present.
/// - `Err(_)`, if both fields are present.
/// - `Err(_)`, if there is an error deserializing either field value.
pub(super) mod option {
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    use super::{Field, NetworkConfig, NetworkMode, Networks};

    /// Serialize [`Option<NetworkConfig>`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `serializer` does while serializing.
    pub(in super::super) fn serialize<S: Serializer>(
        value: &Option<NetworkConfig>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }

    /// Deserialize [`Option<NetworkConfig>`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `deserializer` does, there is an error deserializing either
    /// [`NetworkConfig`] variant, or both fields are present.
    pub(in super::super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<NetworkConfig>, D::Error> {
        let NetworkConfigFlat {
            network_mode,
            networks,
        } = NetworkConfigFlat::deserialize(deserializer)?;

        match (network_mode, networks) {
            (Some(network_mode), None) => Ok(Some(network_mode.into())),
            (None, Some(networks)) => Ok(Some(networks.into())),
            (None, None) => Ok(None),
            (Some(_), Some(_)) => Err(de::Error::custom(format_fields!(
                "cannot set both `{}` and `{}`"
            ))),
        }
    }

    /// Flattened version of [`NetworkConfig`].
    #[derive(Deserialize)]
    #[serde(
        rename = "NetworkConfig",
        expecting = "a struct with either a `network_mode` or `networks` field"
    )]
    struct NetworkConfigFlat {
        #[serde(default)]
        network_mode: Option<NetworkMode>,
        #[serde(default)]
        networks: Option<Networks>,
    }
}

/// [`Service`](super::Service) container's network mode.
///
/// Available values are platform specific, but some options are defined in the Compose
/// specification.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#network_mode)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum NetworkMode {
    /// Turns off all container networking.
    None,

    /// Gives the container raw access to the host's network interface.
    Host,

    /// Gives the container access to the specified service only.
    Service(Identifier),

    /// Other network mode.
    Other(String),
}

impl NetworkMode {
    /// [`Self::None`] string value.
    const NONE: &'static str = "none";

    /// [`Self::Host`] string value.
    const HOST: &'static str = "host";

    /// [`Self::Service`] string prefix.
    const SERVICE_PREFIX: &'static str = "service:";

    /// Parse a [`NetworkMode`] from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the service in the service network mode is not a valid [`Identifier`].
    pub fn parse<T>(network_mode: T) -> Result<Self, ParseNetworkModeError>
    where
        T: AsRef<str> + Into<String>,
    {
        let s = network_mode.as_ref();

        if s == Self::NONE {
            Ok(Self::None)
        } else if s == Self::HOST {
            Ok(Self::Host)
        } else if let Some(service) = s.strip_prefix(Self::SERVICE_PREFIX) {
            service.parse().map(Self::Service).map_err(Into::into)
        } else {
            Ok(Self::Other(network_mode.into()))
        }
    }

    /// Returns `true` if the network mode is [`None`].
    ///
    /// [`None`]: NetworkMode::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if the network mode is [`Host`].
    ///
    /// [`Host`]: NetworkMode::Host
    #[must_use]
    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host)
    }

    /// Returns `true` if the network mode is [`Service`].
    ///
    /// [`Service`]: NetworkMode::Service
    #[must_use]
    pub fn is_service(&self) -> bool {
        matches!(self, Self::Service(..))
    }

    /// Returns [`Some`] if the network mode is [`Service`].
    ///
    /// [`Service`]: NetworkMode::Service
    #[must_use]
    pub fn as_service(&self) -> Option<&Identifier> {
        if let Self::Service(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the network mode is [`Other`].
    ///
    /// [`Other`]: NetworkMode::Other
    #[must_use]
    pub fn is_other(&self) -> bool {
        matches!(self, Self::Other(..))
    }

    /// Returns [`Some`] if the network mode is [`Other`].
    ///
    /// [`Other`]: NetworkMode::Other
    #[must_use]
    pub fn as_other(&self) -> Option<&String> {
        if let Self::Other(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Error returned when [parsing](NetworkMode::parse()) a [`NetworkMode`] from a string.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("error parsing service network mode")]
pub struct ParseNetworkModeError(#[from] InvalidIdentifierError);

impl_from_str!(NetworkMode => ParseNetworkModeError);

impl Display for NetworkMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::None => f.write_str(Self::NONE),
            Self::Host => f.write_str(Self::HOST),
            Self::Service(service) => write!(f, "{}{service}", Self::SERVICE_PREFIX),
            Self::Other(other) => f.write_str(other),
        }
    }
}

impl From<NetworkMode> for String {
    fn from(value: NetworkMode) -> Self {
        if let NetworkMode::Other(other) = value {
            other
        } else {
            value.to_string()
        }
    }
}

impl From<NetworkMode> for Cow<'static, str> {
    fn from(value: NetworkMode) -> Self {
        match value {
            NetworkMode::None => Cow::Borrowed(NetworkMode::NONE),
            NetworkMode::Host => Cow::Borrowed(NetworkMode::HOST),
            value => Cow::Owned(value.to_string()),
        }
    }
}

/// How a [`Service`](super::Service) container should connect to a [`Network`](crate::Network).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#networks)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Network {
    /// Alternative hostnames for the service on the network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#aliases)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub aliases: IndexSet<Hostname>,

    /// Static IPv4 address for the service container when joining the network.
    ///
    /// The corresponding network configuration in must have ipam set with subnet configurations
    /// covering the address.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ipv4_address-ipv6_address)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipv4_address: Option<Ipv4Addr>,

    /// Static IPv6 address for the service container when joining the network.
    ///
    /// The corresponding network configuration in must have ipam set with subnet configurations
    /// covering the address.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ipv4_address-ipv6_address)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<Ipv6Addr>,

    /// A list of link-local IPs.
    ///
    /// Link-local IPs are special IPs which belong to a well known subnet and are purely managed by
    /// the operator, usually dependent on the architecture where they are deployed. Implementation
    /// is platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#link_local_ips)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub link_local_ips: IndexSet<IpAddr>,

    /// MAC address used by the service container when connecting to this particular network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#mac_address)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<MacAddress>,

    /// Indicates the order in which the service container will connect to the network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#priority)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u64>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Network {
    /// Returns `true` if the network configuration is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            aliases,
            ipv4_address,
            ipv6_address,
            link_local_ips,
            mac_address,
            priority,
            extensions,
        } = self;

        aliases.is_empty()
            && ipv4_address.is_none()
            && ipv6_address.is_none()
            && link_local_ips.is_empty()
            && mac_address.is_none()
            && priority.is_none()
            && priority.is_none()
            && extensions.is_empty()
    }
}

impl<'a> AsShortIter<'a> for IndexMap<Identifier, Option<Network>> {
    type Iter = Keys<'a, Identifier, Option<Network>>;

    fn as_short_iter(&'a self) -> Option<Self::Iter> {
        self.values()
            .all(|network| network.as_ref().map_or(true, Network::is_empty))
            .then(|| self.keys())
    }
}

/// A MAC address.
///
/// (De)serializes from/to a string of six integers in hex format separated by colons (:), e.g.
/// `92:d0:c6:0a:29:33`.
#[derive(
    SerializeDisplay, DeserializeFromStr, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(expecting = "a MAC address string in hex format, e.g. \"92:d0:c6:0a:29:33\"")]
pub struct MacAddress(pub [u8; 6]);

impl From<[u8; 6]> for MacAddress {
    fn from(value: [u8; 6]) -> Self {
        Self(value)
    }
}

impl From<MacAddress> for [u8; 6] {
    fn from(value: MacAddress) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for MacAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl FromStr for MacAddress {
    type Err = ParseMacAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');

        let mac_address = Self([
            parse_next_hex(&mut split)?,
            parse_next_hex(&mut split)?,
            parse_next_hex(&mut split)?,
            parse_next_hex(&mut split)?,
            parse_next_hex(&mut split)?,
            parse_next_hex(&mut split)?,
        ]);

        if split.next().is_some() {
            Err(ParseMacAddressError::Length)
        } else {
            Ok(mac_address)
        }
    }
}

/// Parse the next string slice in the `split` iterator as [`u8`] in hex format.
///
/// # Errors
///
/// Returns an error if the iterator is finished or the value is not a valid hex integer.
fn parse_next_hex(split: &mut Split<char>) -> Result<u8, ParseMacAddressError> {
    let value = split.next().ok_or(ParseMacAddressError::Length)?;
    u8::from_str_radix(value, 16).map_err(|source| ParseMacAddressError::Int {
        source,
        value: value.to_owned(),
    })
}

/// Error returned when parsing a [`MacAddress`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseMacAddressError {
    /// MAC address was not the correct length.
    #[error("MAC addresses must be exactly six integers")]
    Length,

    /// Error parsing value as an integer in hex format.
    #[error("error parsing `{value}` as an integer in hex format")]
    Int {
        /// Source of the error.
        source: ParseIntError,

        /// Value attempted to parse.
        value: String,
    },
}

impl TryFrom<&str> for MacAddress {
    type Error = ParseMacAddressError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for MacAddress {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(byte) = iter.next() {
            LowerHex::fmt(byte, f)?;
        }

        for byte in iter {
            write!(f, ":{byte:02x}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indexmap::indexset;

    use super::*;

    #[test]
    fn network_mode() {
        let config = NetworkConfig::NetworkMode(NetworkMode::None);
        let string = "network_mode: none\n";
        assert_eq!(config, serde_yaml::from_str(string).unwrap());
        assert_eq!(serde_yaml::to_string(&config).unwrap(), string);
    }

    #[test]
    fn networks() {
        let config = NetworkConfig::Networks(indexset! { Identifier::new("test").unwrap() }.into());
        let string = "networks:\n- test\n";
        assert_eq!(config, serde_yaml::from_str(string).unwrap());
        assert_eq!(serde_yaml::to_string(&config).unwrap(), string);
    }

    #[test]
    fn missing_err() {
        assert!(serde_yaml::from_str::<NetworkConfig>("{}")
            .unwrap_err()
            .to_string()
            .contains("missing"));
    }

    #[test]
    fn both_err() {
        assert!(
            serde_yaml::from_str::<NetworkConfig>("{ network_mode: none, networks: [test] }")
                .unwrap_err()
                .to_string()
                .contains("both")
        );
    }

    #[derive(Deserialize, Debug)]
    struct Test {
        #[serde(flatten, with = "option")]
        network_config: Option<NetworkConfig>,
    }

    #[test]
    fn flatten_option_none() {
        assert_eq!(
            serde_yaml::from_str::<Test>("{}").unwrap().network_config,
            None,
        );
    }

    #[test]
    fn flatten_option_both_err() {
        assert!(
            serde_yaml::from_str::<Test>("{ network_mode: none, networks: [test] }")
                .unwrap_err()
                .to_string()
                .contains("both")
        );
    }

    mod mac_address {
        use proptest::{prop_assert_eq, proptest};

        use super::*;

        #[test]
        fn from_str() {
            assert_eq!(
                MacAddress([0x92, 0xd0, 0xc6, 0x0a, 0x29, 0x33]),
                "92:d0:c6:0a:29:33".parse().unwrap(),
            );
        }

        #[test]
        fn display() {
            assert_eq!(
                MacAddress([0x92, 0xd0, 0xc6, 0x0a, 0x29, 0x33]).to_string(),
                "92:d0:c6:0a:29:33",
            );
        }

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<MacAddress>();
            }

            #[test]
            fn to_string_no_panic(mac_address: [u8; 6]) {
                MacAddress(mac_address).to_string();
            }

            #[test]
            fn round_trip(mac_address: [u8; 6]) {
                let mac_address = MacAddress(mac_address);
                prop_assert_eq!(mac_address, mac_address.to_string().parse()?);
            }
        }
    }
}
