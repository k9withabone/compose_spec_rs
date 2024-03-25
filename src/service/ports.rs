//! Provides [`Ports`] for the `ports` field of [`Service`](super::Service).
//!
//! Each port may be in the [`ShortPort`] syntax or the long [`Port`] syntax.

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{self, Display, Formatter, Write},
    hash::{Hash, Hasher},
    net::{AddrParseError, IpAddr},
    num::ParseIntError,
    ops::{Add, AddAssign, RangeInclusive, Sub, SubAssign},
    str::FromStr,
};

use compose_spec_macros::{AsShort, DeserializeTryFromString, FromShort, SerializeDisplay};
use indexmap::IndexSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{impl_from_str, serde::FromStrOrU16Visitor, Extensions, ShortOrLong};

/// [`Service`](super::Service) container ports to publish to the host.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ports)
pub type Ports = IndexSet<ShortOrLong<ShortPort, Port>>;

/// Convert [`Ports`] into an [`Iterator`] of ports in the [`ShortPort`] syntax.
///
/// If a [`Port`] cannot be represented in the [`ShortPort`] syntax, it is returned in the [`Err`]
/// variant of the item.
pub fn into_short_iter(ports: Ports) -> impl Iterator<Item = Result<ShortPort, Port>> {
    ports.into_iter().map(|port| match port {
        ShortOrLong::Short(port) => Ok(port),
        ShortOrLong::Long(port) => port.into_short(),
    })
}

/// Convert [`Ports`] into an [`Iterator`] of long syntax [`Port`]s.
///
/// One [`ShortPort`] may represent multiple [`Port`]s as a [`Port`] may only have one target and a
/// [`ShortPort`] may have a range of container ports.
pub fn into_long_iter(ports: Ports) -> impl Iterator<Item = Port> {
    ports.into_iter().flat_map(|port| match port {
        ShortOrLong::Short(port) => ShortOrLong::Short(port.into_long_iter()),
        ShortOrLong::Long(port) => ShortOrLong::Long(std::iter::once(port)),
    })
}

/// Long syntax for a port in a [`Service`](super::Service)'s [`Ports`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-3)
#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub struct Port {
    /// A human-readable name for the port, used to document it's usage within the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The container port.
    pub target: u16,

    /// Host port range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<Range>,

    /// Host network interface IP address to bind to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_ip: Option<IpAddr>,

    /// Port protocol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,

    /// Application protocol (TCP/IP level 4 / OSI level 7) the port is used for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_protocol: Option<String>,

    /// Port mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            name,
            target,
            published,
            host_ip,
            protocol,
            app_protocol,
            mode,
            extensions,
        } = self;

        *name == other.name
            && *target == other.target
            && *published == other.published
            && *host_ip == other.host_ip
            && *protocol == other.protocol
            && *app_protocol == other.app_protocol
            && *mode == other.mode
            && extensions.as_slice() == other.extensions.as_slice()
    }
}

impl Hash for Port {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            name,
            target,
            published,
            host_ip,
            protocol,
            app_protocol,
            mode,
            extensions,
        } = self;

        name.hash(state);
        target.hash(state);
        published.hash(state);
        host_ip.hash(state);
        protocol.hash(state);
        app_protocol.hash(state);
        mode.hash(state);
        extensions.as_slice().hash(state);
    }
}

impl Port {
    /// Create a new [`Port`], only setting the `target`.
    #[must_use]
    pub fn new(target: u16) -> Self {
        Self {
            name: None,
            target,
            published: None,
            host_ip: None,
            protocol: None,
            app_protocol: None,
            mode: None,
            extensions: Extensions::default(),
        }
    }

    /// Convert into the [`ShortPort`] syntax if possible.
    ///
    /// # Errors
    ///
    /// Returns ownership if this long syntax cannot be represented as the short syntax.
    pub fn into_short(self) -> Result<ShortPort, Self> {
        if self.name.is_none()
            && self.app_protocol.is_none()
            && self.mode.is_none()
            && self.extensions.is_empty()
            && self.published.map_or(true, |range| range.end.is_none())
        {
            Ok(ShortPort {
                host_ip: self.host_ip,
                ranges: ShortRanges {
                    host: self.published,
                    container: self.target.into(),
                },
                protocol: self.protocol,
            })
        } else {
            Err(self)
        }
    }
}

impl From<u16> for Port {
    fn from(target: u16) -> Self {
        Self::new(target)
    }
}

/// [`Port`] mode.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-3)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Publish to the host port on each node.
    Host,

    /// Load balance the port.
    #[default]
    Ingress,
}

impl Mode {
    /// Port mode as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Ingress => "ingress",
        }
    }
}

impl AsRef<str> for Mode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Short syntax for a port in a [`Service`](super::Service)'s [`Ports`].
///
/// (De)serializes from/to an integer or string in the format
/// `[[{host_ip}:][{host}]:]{container}[/{protocol}]`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#short-syntax-3)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortPort {
    /// Host network interface IP address to bind to.
    pub host_ip: Option<IpAddr>,
    /// Host and container port ranges.
    pub ranges: ShortRanges,
    /// Port protocol.
    pub protocol: Option<Protocol>,
}

impl ShortPort {
    /// Create a new [`ShortPort`].
    #[must_use]
    pub fn new(ranges: ShortRanges) -> Self {
        Self {
            host_ip: None,
            ranges,
            protocol: None,
        }
    }

    /// Convert short port syntax into an [`Iterator`] of long [`Port`] syntax.
    ///
    /// One [`ShortPort`] may represent multiple [`Port`]s as a [`Port`] may only have one target
    /// and a [`ShortPort`] may have a range of container ports.
    pub fn into_long_iter(self) -> impl Iterator<Item = Port> {
        let Self {
            host_ip,
            ranges,
            protocol,
        } = self;

        ranges.into_iter().map(move |(host, container)| Port {
            published: host.map(Into::into),
            host_ip,
            protocol: protocol.clone(),
            ..container.into()
        })
    }
}

impl From<ShortRanges> for ShortPort {
    fn from(ranges: ShortRanges) -> Self {
        Self::new(ranges)
    }
}

impl From<Range> for ShortPort {
    fn from(container: Range) -> Self {
        ShortRanges::from(container).into()
    }
}

impl From<u16> for ShortPort {
    fn from(container: u16) -> Self {
        Range::from(container).into()
    }
}

impl FromStr for ShortPort {
    type Err = ParseShortPortError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "[[{host_ip}:][{host}]:]{container}[/{protocol}]"

        let (mut s, protocol) = s
            .split_once('/')
            .map_or((s, None), |(s, protocol)| (s, Some(protocol.into())));

        let mut colon_seen = false;
        let host_ip = s
            .rsplit_once(|char| {
                // Split at the second to last ':'
                if char == ':' {
                    if colon_seen {
                        return true;
                    }
                    colon_seen = true;
                }
                false
            })
            .map(|(host_ip, rest)| {
                s = rest;
                host_ip
                    .parse()
                    .map_err(|source| ParseShortPortError::IpAddr {
                        source,
                        value: host_ip.to_owned(),
                    })
            })
            .transpose()?;

        Ok(Self {
            host_ip,
            ranges: s.parse()?,
            protocol,
        })
    }
}

impl TryFrom<&str> for ShortPort {
    type Error = ParseShortPortError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing [`ShortPort`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseShortPortError {
    /// Error parsing [`IpAddr`] for `host_ip`.
    #[error("error parsing host ip address")]
    IpAddr {
        /// Source of the error.
        source: AddrParseError,
        /// Value attempted to parse.
        value: String,
    },

    /// Error parsing [`ShortRanges`] for `ranges`.
    #[error("error parsing port ranges")]
    ShortRanges(#[from] ParseShortRangesError),
}

impl Display for ShortPort {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            host_ip,
            ranges,
            protocol,
        } = self;

        // Format is "[[{host_ip}:][{host}]:]{container}[/{protocol}]"

        if let Some(host_ip) = host_ip {
            write!(f, "{host_ip}:")?;
            if ranges.host.is_none() {
                // `ShortRanges` doesn't write a ':' colon if `host` is `None`, but it is required
                // if `host_ip` is `Some`.
                f.write_char(':')?;
            }
        }

        Display::fmt(ranges, f)?;

        if let Some(protocol) = protocol {
            write!(f, "/{protocol}")?;
        }

        Ok(())
    }
}

impl Serialize for ShortPort {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.host_ip.is_some() || self.protocol.is_some() {
            serializer.collect_str(self)
        } else {
            self.ranges.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ShortPort {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromStrOrU16Visitor::new(
            "an integer or string in the format \
                \"[[{host_ip}:][{host}]:]{container}[/{protocol}]\"",
        )
        .deserialize(deserializer)
    }
}

/// Host and container port ranges of [`ShortPort`].
///
/// (De)serializes from/to an integer or string in the format `[[{host}]:]{container}`.
#[derive(AsShort, FromShort, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShortRanges {
    /// Host port range.
    host: Option<Range>,

    /// Container port range.
    #[as_short(short)]
    container: Range,
}

impl ShortRanges {
    /// Create a new [`ShortRanges`].
    ///
    /// # Errors
    ///
    /// Returns an error if the host port range size is not equal to the container port range size.
    pub fn new(host: Option<Range>, container: Range) -> Result<Self, ShortRangesError> {
        range_size_eq(host, container)?;

        Ok(Self { host, container })
    }

    /// Host port range.
    #[must_use]
    pub fn host(&self) -> Option<Range> {
        self.host
    }

    /// Replaces the host port range, returning the old value if present.
    ///
    /// # Errors
    ///
    /// Returns an error if the new host port range size is not equal to the current container port
    /// range size.
    pub fn replace_host(&mut self, host: Range) -> Result<Option<Range>, ShortRangesError> {
        range_size_eq(Some(host), self.container)?;
        Ok(self.host.replace(host))
    }

    /// Removes the host port range and returns it.
    pub fn take_host(&mut self) -> Option<Range> {
        self.host.take()
    }

    /// Container port range.
    #[must_use]
    pub fn container(&self) -> Range {
        self.container
    }

    /// Replace the container port range.
    ///
    /// # Errors
    ///
    /// Returns an error if the new container port range size is not equal to the current host port
    /// range size if set.
    pub fn replace_container(&mut self, container: Range) -> Result<Range, ShortRangesError> {
        range_size_eq(self.host, container)?;
        Ok(std::mem::replace(&mut self.container, container))
    }
}

/// Ensure that the `host` range size is equal to the `container` range size.
///
/// # Errors
///
/// Returns an error if the range sizes are not equal.
fn range_size_eq(host: Option<Range>, container: Range) -> Result<(), ShortRangesError> {
    if let Some(host) = host {
        let host_size = host.size();
        let container_size = container.size();
        if host_size != container_size {
            return Err(ShortRangesError {
                host_size,
                container_size,
            });
        }
    }

    Ok(())
}

/// Error returned when creating a new [`ShortRanges`].
///
/// Occurs when the host port range size is not equal to the container port range size.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error(
    "host port range size `{host_size}` must be equal to \
        container port range size `{container_size}`"
)]
pub struct ShortRangesError {
    host_size: u16,
    container_size: u16,
}

impl From<u16> for ShortRanges {
    fn from(container: u16) -> Self {
        Range::from(container).into()
    }
}

impl FromStr for ShortRanges {
    type Err = ParseShortRangesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "[[{host}]:]{container}"

        if let Some((host, container)) = s.split_once(':') {
            let host = if host.is_empty() {
                None
            } else {
                Some(host.parse()?)
            };

            Ok(Self::new(host, container.parse()?)?)
        } else {
            Ok(Range::from_str(s)?.into())
        }
    }
}

impl TryFrom<&str> for ShortRanges {
    type Error = ParseShortRangesError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for ShortRanges {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { host, container } = self;

        // Format is "[{host}:]{container}"

        if let Some(host) = host {
            write!(f, "{host}:")?;
        }

        Display::fmt(container, f)
    }
}

impl Serialize for ShortRanges {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.host.is_some() {
            serializer.collect_str(self)
        } else {
            self.container.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ShortRanges {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromStrOrU16Visitor::new("an integer or string in the format \"[[{host}]:]{container}\"")
            .deserialize(deserializer)
    }
}

/// Error returned when parsing [`ShortRanges`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseShortRangesError {
    /// Error creating [`ShortRanges`]
    #[error("error creating the port ranges")]
    ShortRanges(#[from] ShortRangesError),

    /// Error parsing [`Range`].
    #[error("error parsing port range")]
    Range(#[from] ParseRangeError),
}

impl IntoIterator for ShortRanges {
    type Item = (Option<u16>, u16);

    type IntoIter = ShortRangesIter;

    fn into_iter(self) -> Self::IntoIter {
        let Self { host, container } = self;

        ShortRangesIter {
            host: host.map(Range::into_iter),
            container: container.into_iter(),
        }
    }
}

/// An [`Iterator`] which yields host-container port pairs from [`ShortRanges`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortRangesIter {
    /// Host port iterator.
    host: Option<RangeInclusive<u16>>,
    /// Container port iterator.
    container: RangeInclusive<u16>,
}

impl Iterator for ShortRangesIter {
    type Item = (Option<u16>, u16);

    fn next(&mut self) -> Option<Self::Item> {
        self.container
            .next()
            .map(|container| (self.host.as_mut().and_then(Iterator::next), container))
    }
}

/// A single or range of network ports.
///
/// The start of the range must be less than or equal to the end.
///
/// (De)serializes from/to an integer or string in the format `{start}[-{end}]` where `start` and
/// `end` are integers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    /// Start of the port range. If `end` is [`None`], then this is the only port.
    start: u16,

    /// End of the port range.
    end: Option<u16>,
}

impl Range {
    /// Create a [`Range`].
    ///
    /// # Errors
    ///
    /// Returns an error if `start` is greater than `end`.
    pub fn new(start: u16, end: Option<u16>) -> Result<Self, RangeError> {
        if let Some(end) = end {
            match start.cmp(&end) {
                Ordering::Less => Ok(Self {
                    start,
                    end: Some(end),
                }),
                Ordering::Equal => Ok(Self { start, end: None }),
                Ordering::Greater => Err(RangeError { start, end }),
            }
        } else {
            Ok(Self { start, end: None })
        }
    }

    /// Start of the port range.
    #[must_use]
    pub fn start(&self) -> u16 {
        self.start
    }

    /// End of the port range.
    ///
    /// Returns [`None`] if [`start()`](Self::start()) is the only port in the range.
    #[must_use]
    pub fn end(&self) -> Option<u16> {
        self.end
    }

    /// Size of the port range.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), compose_spec::service::ports::RangeError> {
    /// use compose_spec::service::ports::Range;
    ///
    /// let range = Range::from(8000);
    /// assert_eq!(range.size(), 1);
    ///
    /// let range = Range::new(8000, Some(8010))?;
    /// assert_eq!(range.size(), 11);
    /// assert_eq!(range.into_iter().count(), range.size().into());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn size(&self) -> u16 {
        self.end.map_or(1, |end| end - self.start + 1)
    }
}

/// Error returned when creating a [`Range`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("the start `{start}` of the port range must be less than or equal to the end `{end}`")]
pub struct RangeError {
    start: u16,
    end: u16,
}

impl PartialEq<u16> for Range {
    fn eq(&self, other: &u16) -> bool {
        self.end.is_none() && self.start == *other
    }
}

impl PartialEq<RangeInclusive<u16>> for Range {
    fn eq(&self, other: &RangeInclusive<u16>) -> bool {
        self.start == *other.start() && self.end.unwrap_or(self.start) == *other.end()
    }
}

impl From<u16> for Range {
    fn from(start: u16) -> Self {
        Self { start, end: None }
    }
}

impl TryFrom<(u16, Option<u16>)> for Range {
    type Error = RangeError;

    fn try_from((start, end): (u16, Option<u16>)) -> Result<Self, Self::Error> {
        Self::new(start, end)
    }
}

impl TryFrom<(u16, u16)> for Range {
    type Error = RangeError;

    fn try_from((start, end): (u16, u16)) -> Result<Self, Self::Error> {
        Self::new(start, Some(end))
    }
}

impl TryFrom<RangeInclusive<u16>> for Range {
    type Error = RangeError;

    fn try_from(value: RangeInclusive<u16>) -> Result<Self, Self::Error> {
        value.into_inner().try_into()
    }
}

impl Add<u16> for Range {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        let Self { start, end } = self;

        Self {
            start: start + rhs,
            end: end.map(|end| end + rhs),
        }
    }
}

impl AddAssign<u16> for Range {
    fn add_assign(&mut self, rhs: u16) {
        *self = *self + rhs;
    }
}

impl Sub<u16> for Range {
    type Output = Self;

    fn sub(self, rhs: u16) -> Self::Output {
        let Self { start, end } = self;

        Self {
            start: start - rhs,
            end: end.map(|end| end - rhs),
        }
    }
}

impl SubAssign<u16> for Range {
    fn sub_assign(&mut self, rhs: u16) {
        *self = *self - rhs;
    }
}

impl FromStr for Range {
    type Err = ParseRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s
            .split_once('-')
            .map_or((s, None), |(start, end)| (start, Some(end)));

        Ok(Self {
            start: parse_range_int(start)?,
            end: end.map(parse_range_int).transpose()?,
        })
    }
}

/// Parse a [`Range`] value into a [`u16`].
fn parse_range_int(value: &str) -> Result<u16, ParseRangeError> {
    value.parse().map_err(|source| ParseRangeError::Int {
        source,
        value: value.to_owned(),
    })
}

impl TryFrom<&str> for Range {
    type Error = ParseRangeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`Range`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseRangeError {
    /// Error creating [`Range`].
    #[error("error creating the port range")]
    Range(#[from] RangeError),

    /// Error parsing an integer.
    #[error("error parsing `{value}` as an integer")]
    Int {
        /// Source of the error.
        source: ParseIntError,
        /// Value attempted to parse.
        value: String,
    },
}

impl Display for Range {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { start, end } = self;

        Display::fmt(start, f)?;

        if let Some(end) = end {
            write!(f, "-{end}")?;
        }

        Ok(())
    }
}

impl Serialize for Range {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.end.is_some() {
            serializer.collect_str(self)
        } else {
            self.start.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Range {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromStrOrU16Visitor::new(
            "an integer or string in the format \"{start}[-{end}]\" \
                where start and end are integers",
        )
        .deserialize(deserializer)
    }
}

impl IntoIterator for Range {
    type Item = u16;

    type IntoIter = RangeInclusive<u16>;

    fn into_iter(self) -> Self::IntoIter {
        let Self { start, end } = self;

        end.map_or(start..=start, |end| start..=end)
    }
}

/// Port protocol.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ports)
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Default, Clone, PartialEq, Eq, Hash,
)]
pub enum Protocol {
    /// Transmission Control Protocol (TCP)
    #[default]
    Tcp,

    /// User Datagram Protocol (UDP)
    Udp,

    /// Some other protocol.
    Other(String),
}

impl Protocol {
    /// [`Self::Tcp`] string value.
    const TCP: &'static str = "tcp";

    /// [`Self::Udp`] string value.
    const UDP: &'static str = "udp";

    /// Parse [`Protocol`] from a string.
    pub fn parse<T>(protocol: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match protocol.as_ref() {
            Self::TCP => Self::Tcp,
            Self::UDP => Self::Udp,
            _ => Self::Other(protocol.into()),
        }
    }

    /// Returns `true` if the protocol is [`Tcp`].
    ///
    /// [`Tcp`]: Protocol::Tcp
    #[must_use]
    pub const fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp)
    }

    /// Returns `true` if the protocol is [`Udp`].
    ///
    /// [`Udp`]: Protocol::Udp
    #[must_use]
    pub const fn is_udp(&self) -> bool {
        matches!(self, Self::Udp)
    }

    /// Protocol as a string slice.
    ///
    /// Convenience method for `as_ref()` to a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tcp => Self::TCP,
            Self::Udp => Self::UDP,
            Self::Other(other) => other,
        }
    }
}

impl_from_str!(Protocol);

impl AsRef<str> for Protocol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Protocol> for String {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Tcp | Protocol::Udp => value.as_str().to_owned(),
            Protocol::Other(other) => other,
        }
    }
}

impl From<Protocol> for Cow<'static, str> {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Tcp => Self::Borrowed(Protocol::TCP),
            Protocol::Udp => Self::Borrowed(Protocol::UDP),
            Protocol::Other(other) => Self::Owned(other),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use proptest::{
        arbitrary::any,
        option, prop_assert_eq, prop_compose, prop_oneof, proptest,
        strategy::{Just, Strategy},
    };

    use super::*;

    mod short_port {
        use super::*;

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<ShortPort>();
            }

            #[test]
            fn round_trip(port in short_port()) {
                prop_assert_eq!(&port, &port.to_string().parse()?);
            }
        }
    }

    mod range {
        use super::*;

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<Range>();
            }

            #[test]
            fn round_trip(range in range()) {
                prop_assert_eq!(range, range.to_string().parse::<Range>()?);
            }
        }
    }

    proptest! {
        #[test]
        fn short_ranges_iter(ranges in short_ranges()) {
            let iter: Vec<_> = if let Some(host) = ranges.host {
                host.into_iter().map(Some).zip(ranges.container).collect()
            } else {
                std::iter::repeat(None).zip(ranges.container).collect()
            };

            let ranges: Vec<_> = ranges.into_iter().collect();
            prop_assert_eq!(ranges, iter);
        }
    }

    prop_compose! {
        fn short_port()(
            host_ip: Option<IpAddr>,
            ranges in short_ranges(),
            protocol in option::of(protocol())
        ) -> ShortPort {
            ShortPort {
                host_ip,
                ranges,
                protocol
            }
        }
    }

    prop_compose! {
        fn short_ranges()(range in range())(
            range in Just(range),
            offset in ..u16::MAX - range.end.unwrap_or(range.start)
        ) -> ShortRanges {
            ShortRanges {
                host: (offset != 0).then(|| range + offset),
                container: range,
            }
        }
    }

    pub(in super::super) fn range() -> impl Strategy<Value = Range> {
        any::<u16>()
            .prop_flat_map(|start| (Just(start), option::of(start..)))
            .prop_map(|(start, end)| Range {
                start,
                end: end.filter(|end| *end != start),
            })
    }

    pub(in super::super) fn protocol() -> impl Strategy<Value = Protocol> {
        prop_oneof![
            Just(Protocol::Tcp),
            Just(Protocol::Udp),
            any::<String>().prop_map_into(),
        ]
    }
}
