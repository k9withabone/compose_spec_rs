//! (De)serialize an [`Option<Duration>`] from/to duration strings.
//!
//! See the [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md#specifying-durations)
//! for the format.

use std::{
    fmt::{self, Formatter},
    time::Duration,
};

use serde::{
    de::{self},
    Deserializer, Serialize, Serializer,
};

use super::forward_visitor;

/// Serialize an [`Option<Duration>`] as a duration string.
pub(crate) fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration
        .map(crate::duration::to_string)
        .serialize(serializer)
}

/// Deserialize an [`Option<Duration>`] from microseconds or a duration string.
pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(Visitor)
}

/// [`de::Visitor`] for deserializing an [`Option<Duration>`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Option<Duration>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("integer representing microseconds or a duration string")
    }

    forward_visitor! {
        visit_u64,
        visit_i64: i64,
        visit_i128: i128,
        visit_u128: u128,
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(Duration::from_micros(v)))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        crate::duration::parse(v).map(Some).map_err(E::custom)
    }
}
