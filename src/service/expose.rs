//! Provides [`Expose`] for the `expose` field of [`Service`](super::Service).

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use serde::{
    de::{self, IntoDeserializer},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::serde::FromStrVisitor;

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
        deserializer.deserialize_any(Visitor)
    }
}

/// [`de::Visitor`] for deserializing [`Expose`].
struct Visitor;

/// Implement [`de::Visitor`] functions by forwarding to [`de::Visitor::visit_u16()`].
macro_rules! forward_to_u16 {
    ($($f:ident: $ty:ty,)*) => {
        $(
            fn $f<E: de::Error>(self, v: $ty) -> Result<Self::Value, E> {
                self.visit_u16(v.try_into().map_err(E::custom)?)
            }
        )*
    };
}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Expose;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an integer or string representing a port or port range")
    }

    forward_to_u16! {
        visit_i8: i8,
        visit_i16: i16,
        visit_i32: i32,
        visit_i64: i64,
        visit_i128: i128,
        visit_u32: u32,
        visit_u64: u64,
        visit_u128: u128,
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        self.visit_u16(v.into())
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(v.into())
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        FromStrVisitor::new("a string representing a port or port range")
            .deserialize(v.into_deserializer())
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
