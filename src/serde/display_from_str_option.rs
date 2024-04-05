//! (De)serialize an [`Option`]al value with its [`Display`] and [`FromStr`] implementations.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    str::FromStr,
};

use serde::{de, Deserialize, Deserializer, Serializer};

use super::{error_chain, FromStrVisitor};

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

/// [`de::Visitor`] for deserializing [`Option<T>`] using `T`'s [`FromStr`] implementation.
struct Visitor<T> {
    /// The optional value type to deserialize.
    value: PhantomData<T>,
}

impl<T> Visitor<T> {
    /// Create a new [`Visitor`].
    const fn new() -> Self {
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

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.parse().map(Some).map_err(error_chain)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        FromStrVisitor::default()
            .deserialize(deserializer)
            .map(Some)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
}
