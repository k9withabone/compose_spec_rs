//! Common types used in various parts of a [`Compose`](super::Compose) file.

mod keys;
mod short_or_long;

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    hash::Hash,
    num::{ParseFloatError, ParseIntError, TryFromIntError},
};

use indexmap::{indexset, IndexMap, IndexSet};
use serde::{de::IntoDeserializer, Deserialize, Deserializer, Serialize};
use serde_untagged::UntaggedEnumVisitor;
pub use serde_yaml::Value as YamlValue;
use thiserror::Error;

use crate::serde::ValueEnumVisitor;

pub(crate) use self::keys::key_impls;
pub use self::{
    keys::{
        ExtensionKey, Identifier, InvalidExtensionKeyError, InvalidIdentifierError,
        InvalidMapKeyError, MapKey,
    },
    short_or_long::{AsShort, ShortOrLong},
};

/// Extensions can be used to enable experimental features or make a [`Compose`](super::Compose)
/// file easier to maintain via
/// [anchors and aliases](https://github.com/compose-spec/compose-spec/blob/master/10-fragments.md).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
pub type Extensions = IndexMap<ExtensionKey, YamlValue>;

/// A single item or a list of unique items.
///
/// `T` must be able to deserialize from a string.
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ItemOrList<T> {
    /// A single item.
    Item(T),

    /// A list of unique items.
    #[serde(bound = "T: Eq + Hash")]
    List(IndexSet<T>),
}

impl<T> PartialEq for ItemOrList<T>
where
    T: Eq + Hash,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Item(item), Self::Item(other)) => item.eq(other),
            (Self::List(list), Self::List(other)) => list.eq(other),
            _ => false,
        }
    }
}

impl<T: Eq + Hash> Eq for ItemOrList<T> {}

impl<T> ItemOrList<T> {
    /// Returns [`Some`] if a single item or the list contains exactly one element.
    #[must_use]
    pub fn as_item(&self) -> Option<&T> {
        match self {
            Self::Item(v) => Some(v),
            Self::List(list) if list.len() == 1 => list.first(),
            Self::List(_) => None,
        }
    }

    /// Returns [`Some`] if a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&IndexSet<T>> {
        if let Self::List(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<T> ItemOrList<T>
where
    T: Eq + Hash,
{
    /// Convert into a list.
    ///
    /// A new [`IndexSet`] is created if a single item.
    #[must_use]
    pub fn into_list(self) -> IndexSet<T> {
        match self {
            Self::Item(item) => indexset![item],
            Self::List(list) => list,
        }
    }
}

impl<'de, T> Deserialize<'de> for ItemOrList<T>
where
    T: Deserialize<'de> + Eq + Hash,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        UntaggedEnumVisitor::new()
            .string(|string| T::deserialize(string.into_deserializer()).map(Self::Item))
            .seq(|seq| seq.deserialize().map(Self::List))
            .deserialize(deserializer)
    }
}

impl<T> From<T> for ItemOrList<T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<T> From<IndexSet<T>> for ItemOrList<T> {
    fn from(value: IndexSet<T>) -> Self {
        Self::List(value)
    }
}

impl<T> From<ItemOrList<T>> for IndexSet<T>
where
    T: Eq + Hash,
{
    fn from(value: ItemOrList<T>) -> Self {
        value.into_list()
    }
}

/// A list of unique strings or a map with optional single values.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ListOrMap {
    /// List of unique strings.
    List(IndexSet<String>),

    /// Map with optional single values.
    Map(IndexMap<MapKey, Option<Value>>),
}

impl ListOrMap {
    /// Returns `true` if the list or map contain no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::List(list) => list.is_empty(),
            Self::Map(map) => map.is_empty(),
        }
    }

    /// Return [`Some`] if a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&IndexSet<String>> {
        if let Self::List(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Return [`Some`] if a map.
    #[must_use]
    pub fn as_map(&self) -> Option<&IndexMap<MapKey, Option<Value>>> {
        if let Self::Map(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Convert into a list.
    ///
    /// If a map, a new [`IndexSet`] is created by joining keys and values with an `=` character,
    /// like so `{key}={value}`. If the value is [`None`], the key is put into the list as is.
    ///
    /// All places [`ListOrMap`] is used within [`Compose`](super::Compose) support the use of the
    // `{key}={value}` syntax.
    #[must_use]
    pub fn into_list(self) -> IndexSet<String> {
        match self {
            Self::List(list) => list,
            Self::Map(map) => map
                .into_iter()
                .map(|(key, value)| {
                    if let Some(value) = value {
                        format!("{key}={value}")
                    } else {
                        key.into()
                    }
                })
                .collect(),
        }
    }

    /// Attempt to convert into a map.
    ///
    /// Split list items into keys and values on the '=' character, i.e. `{key}={value}`. If an item
    /// does not contain '=', then the whole item is used as the key and the value is [`None`].
    /// If the value is an empty string, then it will also be converted to [`None`].
    ///
    /// Alias for [`self.into_map_split_on(&['='])`](Self::into_map_split_on()).
    ///
    /// # Errors
    ///
    /// Returns an error if a key is not a valid [`MapKey`].
    pub fn into_map(self) -> Result<IndexMap<MapKey, Option<Value>>, InvalidMapKeyError> {
        self.into_map_split_on(&['='])
    }

    /// Attempt to convert into a map.
    ///
    /// Split list items into keys and values with the `delimiters`. If an item does not contain a
    /// delimiter, then the whole item is used as the key and the value is [`None`].
    /// If the value is an empty string, then it will also be converted to [`None`].
    ///
    /// # Errors
    ///
    /// Returns an error if a key is not a valid [`MapKey`].
    pub fn into_map_split_on(
        self,
        delimiters: &[char],
    ) -> Result<IndexMap<MapKey, Option<Value>>, InvalidMapKeyError> {
        match self {
            ListOrMap::List(list) => list
                .into_iter()
                .map(|item| {
                    let (key, value) = item
                        .split_once(delimiters)
                        .map_or((item.as_str(), None), |(key, value)| (key, Some(value)));
                    Ok((
                        key.parse()?,
                        value.filter(|value| !value.is_empty()).map(Value::parse),
                    ))
                })
                .collect(),
            ListOrMap::Map(map) => Ok(map),
        }
    }
}

impl<'de> Deserialize<'de> for ListOrMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        UntaggedEnumVisitor::new()
            .seq(|seq| seq.deserialize().map(Self::List))
            .map(|map| map.deserialize().map(Self::Map))
            .deserialize(deserializer)
    }
}

impl Default for ListOrMap {
    fn default() -> Self {
        Self::List(IndexSet::default())
    }
}

/// A single string, integer, float, or boolean value.
///
/// The maximum range of integer values deserialized is `i64::MIN..=u64::MAX`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value {
    /// A [`String`] value.
    String(String),

    /// A [`u64`] (unsigned integer) value.
    ///
    /// Positive integers will parse/deserialize into this.
    UnsignedInt(u64),

    /// A [`i64`] (signed integer) value.
    ///
    /// Negative integers will parse/deserialize into this.
    SignedInt(i64),

    /// A [`f64`] (floating point) value.
    Float(f64),

    /// A [`bool`]ean value.
    Bool(bool),
}

impl Value {
    /// Parse the given string into a [`Value`].
    ///
    /// First, it is attempted to parse the string into one of [`Value`]s non-string types.
    /// If each of those attempts fails, a [`Value::String`] is returned.
    ///
    /// In comparison, the `From<&str>` and `From<String>` implementations will always return
    /// [`Value::String`].
    pub fn parse<T>(string: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        let s = string.as_ref();
        s.parse()
            .map(Self::Bool)
            .or_else(|_| s.parse().map(Self::UnsignedInt))
            .or_else(|_| s.parse().map(Self::SignedInt))
            .or_else(|_| s.parse().map(Self::Float))
            .unwrap_or_else(|_| Self::String(string.into()))
    }

    /// Returns [`Some`] if a [`Value::String`].
    #[must_use]
    pub fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns [`Some`] if a [`Value::UnsignedInt`].
    ///
    /// Use `u64::try_from()` if conversion from other value types is wanted.
    #[must_use]
    pub fn as_unsigned_int(&self) -> Option<u64> {
        if let Self::UnsignedInt(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns [`Some`] if a [`Value::SignedInt`].
    ///
    /// Use `i64::try_from()` if conversion from other value types is wanted.
    #[must_use]
    pub fn as_signed_int(&self) -> Option<i64> {
        if let Self::SignedInt(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns [`Some`] if a [`Value::Float`].
    ///
    /// Use `f64::try_from()` if conversion from other value types is wanted.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns [`Some`] if a [`Value::Bool`].
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ValueEnumVisitor::new("a string, integer, float, or boolean")
            .string(Self::String)
            .u64(Self::UnsignedInt)
            .i64(Self::SignedInt)
            .f64(Self::Float)
            .bool(Self::Bool)
            .deserialize(deserializer)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => Display::fmt(value, f),
            Self::UnsignedInt(value) => Display::fmt(value, f),
            Self::SignedInt(value) => Display::fmt(value, f),
            Self::Float(value) => Display::fmt(value, f),
            Self::Bool(value) => Display::fmt(value, f),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Box<str>> for Value {
    fn from(value: Box<str>) -> Self {
        Self::String(value.into_string())
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(value: Cow<'a, str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::UnsignedInt(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::SignedInt(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Value> for String {
    fn from(value: Value) -> Self {
        if let Value::String(value) = value {
            value
        } else {
            value.to_string()
        }
    }
}

impl TryFrom<Value> for u64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(value) => Ok(value.parse()?),
            Value::UnsignedInt(value) => Ok(value),
            Value::SignedInt(value) => Ok(value.try_into()?),
            Value::Bool(value) => Ok(value.into()),
            Value::Float(_) => Err(TryFromValueError::WrongType),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(value) => Ok(value.parse()?),
            Value::UnsignedInt(value) => Ok(value.try_into()?),
            Value::SignedInt(value) => Ok(value),
            Value::Bool(value) => Ok(value.into()),
            Value::Float(_) => Err(TryFromValueError::WrongType),
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(value) => Ok(value.parse()?),
            Value::Float(value) => Ok(value),
            Value::Bool(value) => Ok(value.into()),
            Value::UnsignedInt(_) | Value::SignedInt(_) => Err(TryFromValueError::WrongType),
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(value) => Ok(value),
            _ => Err(TryFromValueError::WrongType),
        }
    }
}

/// Error returned when failing to convert [`Value`] into another type.
#[derive(Error, Debug)]
pub enum TryFromValueError {
    /// [`Value`] is not the correct type for conversion.
    #[error("value is not the correct type for conversion")]
    WrongType,

    /// Error converting integer type.
    ///
    /// For example, converting from [`Value::SignedInt`] to [`u64`] can fail.
    #[error("error converting integer type")]
    InvalidInt(#[from] TryFromIntError),

    /// Error parsing [`Value::String`] into an integer.
    #[error("error parsing string value into an integer")]
    ParseInt(#[from] ParseIntError),

    /// Error parsing [`Value::String`] into a float.
    #[error("error parsing string value into a float")]
    ParseFloat(#[from] ParseFloatError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_parse() {
        assert_eq!(Value::parse("true"), Value::Bool(true));
        assert_eq!(Value::parse("1"), Value::UnsignedInt(1));
        assert_eq!(Value::parse("-1"), Value::SignedInt(-1));
        assert_eq!(Value::parse("1.23"), Value::Float(1.23));
        assert_eq!(
            Value::parse("string"),
            Value::String(String::from("string")),
        );
    }
}
