use std::{
    cmp::Ordering,
    convert::Infallible,
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    ops::RangeInclusive,
    str::FromStr,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

/// A single or range of network ports.
///
/// The start of the range must be less than or equal to the end.
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
#[derive(Error, Debug, Clone, PartialEq, Eq)]
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
        source: ParseIntError,
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

impl From<&str> for Protocol {
    fn from(value: &str) -> Self {
        match value {
            Self::TCP => Self::Tcp,
            Self::UDP => Self::Udp,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl FromStr for Protocol {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<String> for Protocol {
    fn from(value: String) -> Self {
        match value.as_str() {
            Self::TCP => Self::Tcp,
            Self::UDP => Self::Udp,
            _ => Self::Other(value),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use proptest::{
        arbitrary::any,
        option, prop_oneof, proptest,
        strategy::{Just, Strategy},
    };

    use super::*;

    mod range {
        use proptest::prop_assert_eq;

        use super::*;

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<Range>();
            }

            #[test]
            fn to_string_no_panic(range in range()) {
                range.to_string();
            }

            #[test]
            fn round_trip(range in range()) {
                prop_assert_eq!(range, range.to_string().parse::<Range>()?);
            }
        }
    }

    pub fn range() -> impl Strategy<Value = Range> {
        any::<u16>()
            .prop_flat_map(|start| (Just(start), option::of(start..)))
            .prop_map(|(start, end)| Range { start, end })
    }

    pub fn protocol() -> impl Strategy<Value = Protocol> {
        prop_oneof![
            Just(Protocol::Tcp),
            Just(Protocol::Udp),
            any::<String>().prop_map_into(),
        ]
    }
}
