//! Provides [`ByteValue`].

use std::{
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

use compose_spec_macros::SerializeDisplay;
use serde::{de, Deserialize, Deserializer};
use thiserror::Error;

use crate::serde::error_chain;

/// A value representing a number of bytes.
///
/// [`Serialize`]s to the string "{value}{unit}", where unit is "b", "kb", "mb", or "gb".
///
/// [`Deserialize`]s from an unsigned integer (representing bytes) or a string in form
/// "{value}{unit}", where unit is one of the above values, "k", "m", or "g".
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md#specifying-byte-values)
///
/// [`Serialize`]: ::serde::Serialize
#[derive(SerializeDisplay, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteValue {
    /// Bytes (b)
    Bytes(u64),
    /// Kilobytes (kb): 1kb = 1,000 [`Bytes`](Self::Bytes).
    Kilobytes(u64),
    /// Megabytes (mb): 1mb = 1,000,000 [`Bytes`](Self::Bytes).
    Megabytes(u64),
    /// Gigabytes (gb): 1gb = 1,000,000,000 [`Bytes`](Self::Bytes).
    Gigabytes(u64),
}

impl ByteValue {
    /// Convert to bytes by multiplying by the correct value.
    ///
    /// Returns [`None`] if an overflow occurred.
    #[must_use]
    pub const fn into_bytes(self) -> Option<u64> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Kilobytes(kilobytes) => kilobytes.checked_mul(1_000),
            Self::Megabytes(megabytes) => megabytes.checked_mul(1_000_000),
            Self::Gigabytes(gigabytes) => gigabytes.checked_mul(1_000_000_000),
        }
    }

    /// Unit ("b", "kb", "mb", or "gb") as a static string slice.
    #[must_use]
    pub const fn unit(&self) -> &'static str {
        match self {
            Self::Bytes(_) => "b",
            Self::Kilobytes(_) => "kb",
            Self::Megabytes(_) => "mb",
            Self::Gigabytes(_) => "gb",
        }
    }
}

impl Default for ByteValue {
    fn default() -> Self {
        Self::Bytes(0)
    }
}

impl FromStr for ByteValue {
    type Err = ParseByteValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(ParseByteValueError::Empty)
        } else if let Ok(bytes) = s.parse() {
            Ok(Self::Bytes(bytes))
        } else if let Some(gigabytes) = s.strip_suffix("gb").or_else(|| s.strip_suffix(['g', 'G']))
        {
            parse_u64(gigabytes, Self::Gigabytes)
        } else if let Some(megabytes) = s.strip_suffix("mb").or_else(|| s.strip_suffix(['m', 'M']))
        {
            parse_u64(megabytes, Self::Megabytes)
        } else if let Some(kilobytes) = s.strip_suffix("kb").or_else(|| s.strip_suffix(['k', 'K']))
        {
            parse_u64(kilobytes, Self::Kilobytes)
        } else if let Some(bytes) = s.strip_suffix(['b', 'B']) {
            parse_u64(bytes, Self::Bytes)
        } else {
            Err(ParseByteValueError::UnknownUnit(s.to_owned()))
        }
    }
}

/// Parse `value` as a [`u64`] and map it to the correct type.
///
/// # Errors
///
/// Returns a [`ParseByteValueError::ParseInt`] error if `value` is not an unsigned integer.
fn parse_u64<T>(value: &str, f: impl FnOnce(u64) -> T) -> Result<T, ParseByteValueError> {
    value
        .parse()
        .map(f)
        .map_err(|source| ParseByteValueError::ParseInt {
            source,
            value: value.to_owned(),
        })
}

/// Error returned when parsing a [`ByteValue`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseByteValueError {
    /// Empty string value.
    #[error("value was an empty string")]
    Empty,

    /// Unknown byte unit contained in value.
    #[error(
        "value `{0}` contains an unknown unit, \
        unit must be \"b\", \"k\", \"kb\", \"m\", \"mb\", \"g\", or \"gb\""
    )]
    UnknownUnit(String),

    /// Error parsing value as an unsigned integer.
    #[error("value `{value}` could not be parsed as an unsigned integer")]
    ParseInt {
        /// Source of the error.
        source: ParseIntError,
        /// Value that was attempted to parse.
        value: String,
    },
}

impl Display for ByteValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (Self::Bytes(bytes)
        | Self::Kilobytes(bytes)
        | Self::Megabytes(bytes)
        | Self::Gigabytes(bytes)) = self;

        Display::fmt(bytes, f)?;
        f.write_str(self.unit())
    }
}

impl<'de> Deserialize<'de> for ByteValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = ByteValue;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .write_str("an integer (representing bytes) or a string in the from \"{value}{unit}\"")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(ByteValue::Bytes(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.parse().map_err(error_chain)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::U64Deserializer;

    use super::*;

    #[test]
    fn bytes() {
        assert_eq!(
            ByteValue::deserialize(U64Deserializer::<de::value::Error>::new(1000)).unwrap(),
            ByteValue::Bytes(1000),
        );
        assert_eq!(ByteValue::Bytes(1000), "1000".parse().unwrap());

        let string = "1000b";
        let value: ByteValue = string.parse().unwrap();
        assert_eq!(value, ByteValue::Bytes(1000));
        assert_eq!(value.to_string(), string);
    }

    #[test]
    fn kilobytes() {
        let string = "256kb";
        let value: ByteValue = string.parse().unwrap();
        assert_eq!(value, ByteValue::Kilobytes(256));
        assert_eq!(value.to_string(), string);
        assert_eq!(value, "256k".parse().unwrap());
    }

    #[test]
    fn megabytes() {
        let string = "256mb";
        let value: ByteValue = string.parse().unwrap();
        assert_eq!(value, ByteValue::Megabytes(256));
        assert_eq!(value.to_string(), string);
        assert_eq!(value, "256m".parse().unwrap());
    }

    #[test]
    fn gigabytes() {
        let string = "2gb";
        let value: ByteValue = string.parse().unwrap();
        assert_eq!(value, ByteValue::Gigabytes(2));
        assert_eq!(value.to_string(), string);
        assert_eq!(value, "2g".parse().unwrap());
    }
}
