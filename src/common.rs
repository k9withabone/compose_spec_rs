//! Common types used in various parts of a [`Compose`](super::Compose) file.

mod keys;
mod short_or_long;

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display, Formatter, LowerExp, UpperExp},
    hash::Hash,
    num::{ParseFloatError, ParseIntError, TryFromIntError},
    str::FromStr,
};

use indexmap::{indexset, IndexMap, IndexSet};
use serde::{
    de::{
        self,
        value::{MapAccessDeserializer, SeqAccessDeserializer},
        IntoDeserializer, MapAccess, SeqAccess, Visitor,
    },
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
pub use serde_yaml::Value as YamlValue;
use thiserror::Error;

use crate::serde::{ItemOrListVisitor, ValueEnumVisitor};

pub(crate) use self::keys::key_impls;
pub use self::{
    keys::{
        ExtensionKey, Identifier, InvalidExtensionKeyError, InvalidIdentifierError,
        InvalidMapKeyError, MapKey,
    },
    short_or_long::{AsShort, AsShortIter, ShortOrLong},
};

/// Extensions can be used to enable experimental features or make a [`Compose`](super::Compose)
/// file easier to maintain via
/// [anchors and aliases](https://github.com/compose-spec/compose-spec/blob/master/10-fragments.md).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
pub type Extensions = IndexMap<ExtensionKey, YamlValue>;

/// A single item or a list of unique items.
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
    pub const fn as_list(&self) -> Option<&IndexSet<T>> {
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
        ItemOrListVisitor::<_, T, IndexSet<T>>::default().deserialize(deserializer)
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
    Map(Map),
}

/// Map with optional single values.
pub type Map = IndexMap<MapKey, Option<Value>>;

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
    pub const fn as_list(&self) -> Option<&IndexSet<String>> {
        if let Self::List(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Return [`Some`] if a map.
    #[must_use]
    pub const fn as_map(&self) -> Option<&Map> {
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
    pub fn into_map(self) -> Result<Map, InvalidMapKeyError> {
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
    pub fn into_map_split_on(self, delimiters: &[char]) -> Result<Map, InvalidMapKeyError> {
        match self {
            Self::List(list) => list
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
            Self::Map(map) => Ok(map),
        }
    }
}

impl<'de> Deserialize<'de> for ListOrMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ListOrMapVisitor)
    }
}

/// [`Visitor`] for deserializing [`ListOrMap`].
struct ListOrMapVisitor;

impl<'de> Visitor<'de> for ListOrMapVisitor {
    type Value = ListOrMap;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a sequence of strings or map of strings to optional values")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        IndexSet::deserialize(SeqAccessDeserializer::new(seq)).map(ListOrMap::List)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        Map::deserialize(MapAccessDeserializer::new(map)).map(ListOrMap::Map)
    }
}

impl Default for ListOrMap {
    fn default() -> Self {
        Self::List(IndexSet::default())
    }
}

/// A single string, number, or boolean value.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value {
    /// A [`String`] value.
    String(String),

    /// A [`Number`] value.
    Number(Number),

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
            .or_else(|_| s.parse().map(Self::Number))
            .unwrap_or_else(|_| Self::String(string.into()))
    }

    /// Returns `true` if the value is a [`String`].
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    /// Returns [`Some`] if the value is a [`String`].
    #[must_use]
    pub const fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is a [`Number`].
    #[must_use]
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    /// Returns [`Some`] if the value is a [`Number`].
    #[must_use]
    pub const fn as_number(&self) -> Option<&Number> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is a [`bool`].
    #[must_use]
    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(..))
    }

    /// Returns [`Some`] if the value is a [`bool`].
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueEnumVisitor::new("a string, integer, float, or boolean")
            .string(Self::String)
            .u64(Into::into)
            .i64(Into::into)
            .f64(Into::into)
            .bool(Self::Bool)
            .deserialize(deserializer)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => Display::fmt(value, f),
            Self::Number(value) => Display::fmt(value, f),
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

impl From<Cow<'_, str>> for Value {
    fn from(value: Cow<str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Number::from(value).into()
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Number::from(value).into()
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Number::from(value).into()
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

impl TryFrom<Value> for Number {
    type Error = ParseNumberError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(value) => value.parse(),
            Value::Number(value) => Ok(value),
            Value::Bool(value) => Ok(u64::from(value).into()),
        }
    }
}

impl TryFrom<Value> for u64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Number::try_from(value)?.try_into().map_err(Into::into)
    }
}

impl TryFrom<Value> for i64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Number::try_from(value)?.try_into().map_err(Into::into)
    }
}

impl TryFrom<Value> for f64 {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Number::try_from(value)?.try_into().map_err(Into::into)
    }
}

impl TryFrom<Value> for bool {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(value) => Ok(value),
            _ => Err(TryFromValueError::IntoBool),
        }
    }
}

/// Error returned when attempting to convert [`Value`] into another type.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TryFromValueError {
    /// Error parsing [`Value::String`] into a [`Number`].
    #[error("error parsing string value into a number")]
    ParseNumber(#[from] ParseNumberError),

    /// Error converting [`Number`] into the type.
    #[error("error converting number")]
    TryFromNumber(#[from] TryFromNumberError),

    /// Error converting integer type.
    #[error("error converting integer type")]
    TryFromInt(#[from] TryFromIntError),

    /// Can only convert [`Value::Bool`] into a [`bool`].
    #[error("cannot convert a non-bool value into a bool")]
    IntoBool,
}

/// A numerical [`Value`].
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(untagged)]
pub enum Number {
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
}

impl Number {
    /// Returns `true` if the number is an [`UnsignedInt`].
    ///
    /// [`UnsignedInt`]: Number::UnsignedInt
    #[must_use]
    pub const fn is_unsigned_int(&self) -> bool {
        matches!(self, Self::UnsignedInt(..))
    }

    /// Returns [`Some`] if the number is an [`UnsignedInt`].
    ///
    /// [`UnsignedInt`]: Number::UnsignedInt
    #[must_use]
    pub const fn as_unsigned_int(&self) -> Option<u64> {
        if let Self::UnsignedInt(v) = *self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the number is a [`SignedInt`].
    ///
    /// [`SignedInt`]: Number::SignedInt
    #[must_use]
    pub const fn is_signed_int(&self) -> bool {
        matches!(self, Self::SignedInt(..))
    }

    /// Returns [`Some`] if the number is a [`SignedInt`].
    ///
    /// [`SignedInt`]: Number::SignedInt
    #[must_use]
    pub const fn as_signed_int(&self) -> Option<i64> {
        if let Self::SignedInt(v) = *self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the number is a [`Float`].
    ///
    /// [`Float`]: Number::Float
    #[must_use]
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float(..))
    }

    /// Returns [`Some`] if the number is a [`Float`].
    ///
    /// [`Float`]: Number::Float
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        if let Self::Float(v) = *self {
            Some(v)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueEnumVisitor::new("an integer or float")
            .u64(Self::UnsignedInt)
            .i64(Self::SignedInt)
            .f64(Self::Float)
            .deserialize(deserializer)
    }
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        Self::UnsignedInt(value)
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::SignedInt(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.strip_prefix(['+', '-'])
            .unwrap_or(s)
            .contains(|char: char| !char.is_ascii_digit())
        {
            // Parse as float if `s` contains non-digits, e.g. "5." or "inf".
            Ok(Self::Float(s.parse()?))
        } else if s.starts_with('-') {
            Ok(Self::SignedInt(s.parse()?))
        } else {
            Ok(Self::UnsignedInt(s.parse()?))
        }
    }
}

impl TryFrom<&str> for Number {
    type Error = ParseNumberError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`Number`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseNumberError {
    /// Error parsing [`u64`] or [`i64`].
    #[error("error parsing number as an integer")]
    Int(#[from] ParseIntError),

    /// Error parsing [`f64`].
    #[error("error parsing number as a float")]
    Float(#[from] ParseFloatError),
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsignedInt(number) => Display::fmt(number, f),
            Self::SignedInt(number) => Display::fmt(number, f),
            Self::Float(number) => Display::fmt(number, f),
        }
    }
}

impl LowerExp for Number {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsignedInt(number) => LowerExp::fmt(number, f),
            Self::SignedInt(number) => LowerExp::fmt(number, f),
            Self::Float(number) => LowerExp::fmt(number, f),
        }
    }
}

impl UpperExp for Number {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsignedInt(number) => UpperExp::fmt(number, f),
            Self::SignedInt(number) => UpperExp::fmt(number, f),
            Self::Float(number) => UpperExp::fmt(number, f),
        }
    }
}

impl TryFrom<Number> for u64 {
    type Error = TryFromNumberError;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        match value {
            Number::UnsignedInt(value) => Ok(value),
            Number::SignedInt(value) => value.try_into().map_err(Into::into),
            Number::Float(_) => Err(TryFromNumberError::FloatToInt),
        }
    }
}

impl TryFrom<Number> for i64 {
    type Error = TryFromNumberError;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        match value {
            Number::UnsignedInt(value) => value.try_into().map_err(Into::into),
            Number::SignedInt(value) => Ok(value),
            Number::Float(_) => Err(TryFromNumberError::FloatToInt),
        }
    }
}

impl TryFrom<Number> for f64 {
    type Error = TryFromIntError;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        match value {
            Number::UnsignedInt(value) => u32::try_from(value).map(Into::into),
            Number::SignedInt(value) => i32::try_from(value).map(Into::into),
            Number::Float(value) => Ok(value),
        }
    }
}

/// Error returned when failing to convert a [`Number`] into another type.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryFromNumberError {
    /// Cannot convert from a [`Float`](Number::Float) to an integer.
    #[error("cannot convert a float to an integer")]
    FloatToInt,

    /// Error converting integer type.
    ///
    /// For example, converting from [`Number::SignedInt`] to [`u64`] can fail.
    #[error("error converting integer type")]
    TryFromInt(#[from] TryFromIntError),
}

/// A string or number value.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum StringOrNumber {
    /// A [`String`] value.
    String(String),

    /// A [`Number`] value.
    Number(Number),
}

impl StringOrNumber {
    /// Parse a string into a [`StringOrNumber`].
    ///
    /// It is first attempted to parse the string into a [`Number`].
    /// If that fails a [`StringOrNumber::String`] is returned.
    pub fn parse<T>(value: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        value
            .as_ref()
            .parse()
            .map_or_else(|_| Self::String(value.into()), Self::Number)
    }

    /// Returns `true` if the value is a [`String`].
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    /// Returns [`Some`] if the value is a [`String`].
    #[must_use]
    pub const fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is a [`Number`].
    #[must_use]
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    /// Returns [`Some`] if the value is a [`Number`].
    #[must_use]
    pub const fn as_number(&self) -> Option<Number> {
        if let Self::Number(v) = *self {
            Some(v)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for StringOrNumber {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueEnumVisitor::new("a string, integer, float, or boolean")
            .string(Self::String)
            .u64(Into::into)
            .i64(Into::into)
            .f64(Into::into)
            .deserialize(deserializer)
    }
}

impl From<String> for StringOrNumber {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<Box<str>> for StringOrNumber {
    fn from(value: Box<str>) -> Self {
        value.into_string().into()
    }
}

impl From<&str> for StringOrNumber {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}

impl From<Cow<'_, str>> for StringOrNumber {
    fn from(value: Cow<'_, str>) -> Self {
        value.into_owned().into()
    }
}

impl From<Number> for StringOrNumber {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<u64> for StringOrNumber {
    fn from(value: u64) -> Self {
        Number::from(value).into()
    }
}

impl From<i64> for StringOrNumber {
    fn from(value: i64) -> Self {
        Number::from(value).into()
    }
}

impl From<f64> for StringOrNumber {
    fn from(value: f64) -> Self {
        Number::from(value).into()
    }
}

impl Display for StringOrNumber {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::String(value) => f.write_str(value),
            Self::Number(value) => Display::fmt(value, f),
        }
    }
}

impl From<StringOrNumber> for String {
    fn from(value: StringOrNumber) -> Self {
        match value {
            StringOrNumber::String(value) => value,
            StringOrNumber::Number(value) => value.to_string(),
        }
    }
}

impl TryFrom<StringOrNumber> for Number {
    type Error = ParseNumberError;

    fn try_from(value: StringOrNumber) -> Result<Self, Self::Error> {
        match value {
            StringOrNumber::String(value) => value.parse(),
            StringOrNumber::Number(value) => Ok(value),
        }
    }
}

/// A resource managed either externally or by the compose implementation, e.g.
/// a [`Network`](super::Network) or [`Volume`](super::Volume).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource<T> {
    /// Externally managed resource.
    ///
    /// (De)serializes from/to the mapping `external: true` with an optional `name` entry.
    External {
        /// A custom name for the resource.
        // #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },

    /// Resource manged by the compose implementation.
    Compose(T),
}

impl<T> Resource<T> {
    /// [`Self::External`] field name.
    const EXTERNAL: &'static str = "external";

    /// `Self::External.name` field.
    const NAME: &'static str = "name";

    /// Create a [`Resource::External`] with an optional `name`.
    #[must_use]
    pub const fn external(name: Option<String>) -> Self {
        Self::External { name }
    }

    /// Returns `true` if the resource is [`External`].
    ///
    /// [`External`]: Resource::External
    #[must_use]
    pub const fn is_external(&self) -> bool {
        matches!(self, Self::External { .. })
    }

    /// Returns `true` if the resource is managed by the [`Compose`] implementation.
    ///
    /// [`Compose`]: Resource::Compose
    #[must_use]
    pub const fn is_compose(&self) -> bool {
        matches!(self, Self::Compose(..))
    }

    /// Returns [`Some`] if the resource is managed by the [`Compose`] implementation.
    ///
    /// [`Compose`]: Resource::Compose
    pub const fn as_compose(&self) -> Option<&T> {
        if let Self::Compose(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<T: Serialize> Serialize for Resource<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::External { name } => {
                let mut map = serializer.serialize_map(Some(1 + usize::from(name.is_some())))?;
                map.serialize_entry(Self::EXTERNAL, &true)?;
                if let Some(name) = name {
                    map.serialize_entry(Self::NAME, name)?;
                }
                map.end()
            }
            Self::Compose(resource) => resource.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Resource<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = HashMap::<String, YamlValue>::deserialize(deserializer)?;

        let external = map
            .remove(Self::EXTERNAL)
            .map(bool::deserialize)
            .transpose()
            .map_err(de::Error::custom)?
            .unwrap_or_default();

        if external {
            let name = map
                .remove(Self::NAME)
                .map(String::deserialize)
                .transpose()
                .map_err(de::Error::custom)?;

            if map.is_empty() {
                Ok(Self::External { name })
            } else {
                Err(de::Error::custom(
                    "cannot set `external` and fields other than `name`",
                ))
            }
        } else {
            T::deserialize(map.into_deserializer())
                .map(Self::Compose)
                .map_err(de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_parse() {
        assert_eq!(Value::parse("true"), Value::Bool(true));
        assert_eq!(Value::parse("1"), Value::Number(1_u64.into()));
        assert_eq!(Value::parse("-1"), Value::Number((-1_i64).into()));
        assert_eq!(Value::parse("1.23"), Value::Number(1.23_f64.into()));
        assert_eq!(
            Value::parse("string"),
            Value::String(String::from("string")),
        );
    }
}
