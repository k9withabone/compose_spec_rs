//! Provides [`EndpointMode`] for the `endpoint_mode` field of [`Deploy`](super::Deploy).

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};

use crate::impl_from_str;

/// [`Service`](crate::Service) discovery method for external clients connecting to a service.
///
/// Default and available values are platform specific, however, the specification defines two
/// canonical values: [`vip`](Self::VIp) and [`dnsrr`](Self::DnsRR).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#endpoint_mode)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum EndpointMode {
    /// Assigns the service a virtual IP (VIP) that acts as the front end for clients to reach the
    /// service on a network. The platform routes requests between the client and nodes running the
    /// service, without the client knowing how many nodes are participating in the service or their
    /// IP addresses or ports.
    VIp,

    /// The platform sets up DNS entries for the service such that a DNS query for the service name
    /// returns a list of IP addresses (DNS round-robin), and the client connects directly to one of
    /// these.
    DnsRR,

    /// Some other endpoint mode.
    Other(String),
}

impl EndpointMode {
    /// [`Self::VIp`] string value.
    const VIP: &'static str = "vip";

    /// [`Self::DnsRR`] string value.
    const DNS_RR: &'static str = "dnsrr";

    /// Parse an [`EndpointMode`] from a string.
    pub fn parse<T>(endpoint_mode: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match endpoint_mode.as_ref() {
            Self::VIP => Self::VIp,
            Self::DNS_RR => Self::DnsRR,
            _ => Self::Other(endpoint_mode.into()),
        }
    }

    /// Returns `true` if the endpoint mode is [`VIp`].
    ///
    /// [`VIp`]: EndpointMode::VIp
    #[must_use]
    pub const fn is_vip(&self) -> bool {
        matches!(self, Self::VIp)
    }

    /// Returns `true` if the endpoint mode is [`DnsRR`].
    ///
    /// [`DnsRR`]: EndpointMode::DnsRR
    #[must_use]
    pub const fn is_dnsrr(&self) -> bool {
        matches!(self, Self::DnsRR)
    }

    /// Endpoint mode as a string slice.
    ///
    /// Convenience method for `as_ref()` to a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::VIp => Self::VIP,
            Self::DnsRR => Self::DNS_RR,
            Self::Other(other) => other,
        }
    }
}

impl_from_str!(EndpointMode);

impl AsRef<str> for EndpointMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for EndpointMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<EndpointMode> for String {
    fn from(value: EndpointMode) -> Self {
        match value {
            EndpointMode::VIp | EndpointMode::DnsRR => value.as_str().to_owned(),
            EndpointMode::Other(string) => string,
        }
    }
}

impl From<EndpointMode> for Cow<'static, str> {
    fn from(value: EndpointMode) -> Self {
        match value {
            EndpointMode::VIp => Self::Borrowed(EndpointMode::VIP),
            EndpointMode::DnsRR => Self::Borrowed(EndpointMode::DNS_RR),
            EndpointMode::Other(other) => Self::Owned(other),
        }
    }
}
