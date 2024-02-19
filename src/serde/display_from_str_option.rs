//! (De)serialize an [`Option`]al value with its [`Display`] and [`FromStr`] implementations.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    str::FromStr,
};

use serde::{de, Deserialize, Deserializer, Serializer};

use super::FromStrVisitor;

/// Serialize an [`Option`]al value using its [`Display`] implementation.
pub(crate) fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    if let Some(value) = value {
        serializer.serialize_some(&format_args!("{value}"))
    } else {
        serializer.serialize_none()
    }
}

/// Deserialize an [`Option`]al value using its [`FromStr`] implementation.
pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr + Deserialize<'de>,
    T::Err: Error,
    D: Deserializer<'de>,
{
    deserializer.deserialize_option(Visitor::new())
}

struct Visitor<T> {
    value: PhantomData<T>,
}

impl<T> Visitor<T> {
    fn new() -> Self {
        Self { value: PhantomData }
    }
}

impl<'de, T> de::Visitor<'de> for Visitor<T>
where
    T: FromStr,
    T::Err: Error,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a string or none")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map(Some).map_err(de::Error::custom)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        FromStrVisitor::default()
            .deserialize(deserializer)
            .map(Some)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}
