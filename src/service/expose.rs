//! Provides [`Expose`] for the `expose` field of [`Service`](super::Service).

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::serde::FromStrOrU16Visitor;

use super::ports::{ParseRangeError, Protocol, Range};

/// Incoming port or range of ports which are exposed from the [`Service`](super::Service) container
/// to the host.
///
/// (De)serializes from/to an integer or string in the format `{start}[-{end}][/{protocol}]`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#expose)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Expose {
    /// Port or port range.
    pub range: Range,

    /// Port protocol.
    pub protocol: Option<Protocol>,
}

impl From<u16> for Expose {
    fn from(start: u16) -> Self {
        Self {
            range: start.into(),
            protocol: None,
        }
    }
}

impl From<Range> for Expose {
    fn from(range: Range) -> Self {
        Self {
            range,
            protocol: None,
        }
    }
}

impl FromStr for Expose {
    type Err = ParseRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "{range}[/{protocol}]".

        let (range, protocol) = s.split_once('/').map_or((s, None), |(range, protocol)| {
            (range, Some(protocol.into()))
        });

        Ok(Self {
            range: range.parse()?,
            protocol,
        })
    }
}

impl TryFrom<&str> for Expose {
    type Error = ParseRangeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for Expose {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self { range, protocol } = self;

        // Format is "{range}[/{protocol}]".

        Display::fmt(range, f)?;

        if let Some(protocol) = protocol {
            write!(f, "/{protocol}")?;
        }

        Ok(())
    }
}

impl Serialize for Expose {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.range.end().is_none() && self.protocol.is_none() {
            self.range.start().serialize(serializer)
        } else {
            serializer.collect_str(self)
        }
    }
}

impl<'de> Deserialize<'de> for Expose {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromStrOrU16Visitor::new("an integer or string representing a port or port range")
            .deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use proptest::{option, prop_assert_eq, prop_compose, proptest};

    use crate::service::ports::tests::{protocol, range};

    use super::*;

    proptest! {
        #[test]
        fn parse_no_panic(string: String) {
            let _ = string.parse::<Expose>();
        }

        #[test]
        fn to_string_no_panic(expose in expose()) {
            expose.to_string();
        }

        #[test]
        fn round_trip(expose in expose()) {
            prop_assert_eq!(&expose, &expose.to_string().parse()?);
        }
    }

    prop_compose! {
        fn expose()(range in range(), protocol in option::of(protocol())) -> Expose {
            Expose {
                range,
                protocol,
            }
        }
    }
}
