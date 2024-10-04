//! Provides [`Limit`] for the `memswap_limit` and `pids_limit` fields of
//! [`Service`](super::Service).

use std::{
    fmt::{self, Formatter},
    marker::PhantomData,
};

use serde::{
    de::{self, IntoDeserializer, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::ByteValue;

/// A limit on a [`Service`](super::Service) container resource.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Limit<T> {
    /// Amount of the resource the container may use.
    Value(T),

    /// Allow the container to use an unlimited amount of the resource.
    ///
    /// (De)serializes from/to `-1`.
    #[default]
    Unlimited,
}

impl From<u32> for Limit<u32> {
    fn from(value: u32) -> Self {
        Self::Value(value)
    }
}

impl From<u64> for Limit<u64> {
    fn from(value: u64) -> Self {
        Self::Value(value)
    }
}

impl From<ByteValue> for Limit<ByteValue> {
    fn from(value: ByteValue) -> Self {
        Self::Value(value)
    }
}

impl<T: Serialize> Serialize for Limit<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => value.serialize(serializer),
            Self::Unlimited => serializer.serialize_i8(-1),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Limit<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor { value: PhantomData })
    }
}

/// [`de::Visitor`] for deserializing [`Limit<T>`].
struct Visitor<T> {
    /// The type the [`Limit`] contains.
    value: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
    type Value = Limit<T>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a value or -1")
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        match v {
            ..=-2 => Err(E::invalid_value(
                Unexpected::Signed(v),
                &"-1 or positive integer",
            )),
            -1 => Ok(Limit::Unlimited),
            0.. => self.visit_u64(v.unsigned_abs()),
        }
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(Limit::Value)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(Limit::Value)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(Limit::Value)
    }
}
