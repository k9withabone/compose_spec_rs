//! Provides [`MemswapLimit`] for the `memswap_limit` field of [`Service`](super::Service).

use std::fmt::{self, Formatter};

use serde::{
    de::{self, IntoDeserializer, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::serde::forward_visitor;

use super::ByteValue;

/// The amount of memory a [`Service`](super::Service) container is allowed to swap to disk.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#memswap_limit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemswapLimit {
    /// Amount of swap memory a container may use in bytes.
    Bytes(ByteValue),

    /// Allow the container to use an unlimited amount of swap memory.
    ///
    /// (De)serializes from/to `-1`.
    Unlimited,
}

impl Default for MemswapLimit {
    fn default() -> Self {
        Self::Bytes(ByteValue::default())
    }
}

impl Serialize for MemswapLimit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bytes(bytes) => bytes.serialize(serializer),
            Self::Unlimited => serializer.serialize_i8(-1),
        }
    }
}

impl<'de> Deserialize<'de> for MemswapLimit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

/// [`de::Visitor`] for deserializing [`MemswapLimit`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = MemswapLimit;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a byte value (string or integer) or -1")
    }

    forward_visitor! {
        visit_i8,
        visit_i16: i16,
        visit_i32: i32,
        visit_i64: i64,
        visit_i128: i128,
    }

    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        match v {
            ..=-2 => Err(E::invalid_value(
                Unexpected::Signed(v.into()),
                &"-1 or positive integer",
            )),
            -1 => Ok(MemswapLimit::Unlimited),
            0.. => Ok(MemswapLimit::Bytes(ByteValue::Bytes(
                v.unsigned_abs().into(),
            ))),
        }
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        ByteValue::deserialize(v.into_deserializer()).map(MemswapLimit::Bytes)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        ByteValue::deserialize(v.into_deserializer()).map(MemswapLimit::Bytes)
    }
}
