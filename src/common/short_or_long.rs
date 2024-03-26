use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Formatter},
    hash::Hash,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use indexmap::{IndexMap, IndexSet};
use serde::{
    de::{
        self,
        value::{
            BorrowedBytesDeserializer, BorrowedStrDeserializer, EnumAccessDeserializer,
            MapAccessDeserializer, SeqAccessDeserializer, UnitDeserializer,
        },
        EnumAccess, MapAccess, SeqAccess,
    },
    Deserialize, Deserializer, Serialize,
};

use crate::{
    service::{
        build::Context,
        env_file,
        ports::{Port, ShortPort},
        volumes::{Mount, ShortVolume},
        Build, ConfigOrSecret, Ulimit,
    },
    Identifier, Include,
};

/// Wrapper for types which may be represented as a [`Short`] or [`Long`] syntax.
///
/// The [`Serialize`] implementation forwards to the wrapped types.
///
/// Single values ([`bool`], [`u8`], [`&str`], etc.), options, bytes, unit, newtype structs, enums,
/// and sequences are [`Deserialize`]d into the [`Short`] syntax. Maps are [`Deserialize`]d into the
/// [`Long`] syntax.
///
/// [`Short`]: Self::Short
/// [`Long`]: Self::Long
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum ShortOrLong<S, L> {
    /// Short syntax, a single value.
    Short(S),

    /// Long syntax, a sequence or map.
    Long(L),
}

impl<S, L> Default for ShortOrLong<S, L>
where
    S: Default,
{
    fn default() -> Self {
        Self::Short(S::default())
    }
}

impl<S, L> ShortOrLong<S, L> {
    /// Returns `true` if [`Short`].
    ///
    /// [`Short`]: Self::Short
    #[must_use]
    pub fn is_short(&self) -> bool {
        matches!(self, Self::Short(..))
    }

    /// Returns `true` if [`Long`].
    ///
    /// [`Long`]: Self::Long
    #[must_use]
    pub fn is_long(&self) -> bool {
        matches!(self, Self::Long(..))
    }

    /// Returns [`Some`] if [`Long`].
    ///
    /// [`Long`]: Self::Long
    #[must_use]
    pub fn as_long(&self) -> Option<&L> {
        if let Self::Long(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<S, L> ShortOrLong<S, L>
where
    Self: Into<L>,
{
    /// Convert into [`Long`](Self::Long) syntax.
    #[must_use]
    pub fn into_long(self) -> L {
        self.into()
    }
}

/// Trait for types that represent a long syntax which could also be represented in a short syntax.
pub trait AsShort {
    /// The short syntax type, returned from [`as_short()`](AsShort::as_short()).
    type Short: ?Sized;

    /// Returns [`Some`] if the long syntax can be represented as the short syntax.
    #[must_use]
    fn as_short(&self) -> Option<&Self::Short>;
}

impl<T> AsShort for &T
where
    T: AsShort,
{
    type Short = T::Short;

    fn as_short(&self) -> Option<&Self::Short> {
        T::as_short(self)
    }
}

impl<L> AsShort for ShortOrLong<L::Short, L>
where
    L: AsShort,
    L::Short: Sized,
{
    type Short = L::Short;

    fn as_short(&self) -> Option<&L::Short> {
        match self {
            Self::Short(short) => Some(short),
            Self::Long(long) => long.as_short(),
        }
    }
}

/// Trait similar to [`AsShort`] except it returns an [`Iterator`] instead of a reference.
pub trait AsShortIter<'a> {
    /// [`Iterator`] returned from [`as_short_iter()`](AsShortIter::as_short_iter()).
    type Iter: Iterator;

    /// Returns an [`Iterator`] if the long syntax can be represented as the short syntax.
    #[must_use]
    fn as_short_iter(&'a self) -> Option<Self::Iter>;
}

impl<'a, T> AsShortIter<'a> for &T
where
    T: AsShortIter<'a>,
{
    type Iter = T::Iter;

    fn as_short_iter(&'a self) -> Option<Self::Iter> {
        T::as_short_iter(self)
    }
}

impl<'a, S, L> AsShortIter<'a> for ShortOrLong<S, L>
where
    S: 'a,
    &'a S: IntoIterator<Item = <L::Iter as Iterator>::Item>,
    L: AsShortIter<'a>,
{
    type Iter = ShortOrLong<<&'a S as IntoIterator>::IntoIter, L::Iter>;

    fn as_short_iter(&'a self) -> Option<Self::Iter> {
        match self {
            Self::Short(short) => Some(ShortOrLong::Short(short.into_iter())),
            Self::Long(long) => long.as_short_iter().map(ShortOrLong::Long),
        }
    }
}

impl<S, L> Iterator for ShortOrLong<S, L>
where
    S: Iterator<Item = L::Item>,
    L: Iterator,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Short(iter) => iter.next(),
            Self::Long(iter) => iter.next(),
        }
    }
}

/// `impl<L> From<Type> for ShortOrLong<Type, L>`
macro_rules! impl_from_short {
    ($($t:ty),* $(,)?) => {
        $(
            impl<L> From<$t> for ShortOrLong<$t, L> {
                fn from(value: $t) -> Self {
                    Self::Short(value)
                }
            }
        )*
    };
}

impl_from_short! {
    (),
    bool,
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    String,
    Box<str>,
    PathBuf,
    Box<Path>,
    OsString,
    Box<OsStr>,
    Identifier,
    IndexSet<Identifier>,
    Context,
    ShortPort,
    ShortVolume,
}

impl<L> From<String> for ShortOrLong<PathBuf, L> {
    fn from(value: String) -> Self {
        Self::Short(value.into())
    }
}

/// `impl<S> From<Type> for ShortOrLong<S, Type>` and `impl<S> From<ShortOrLong<S, Type>> for Type`
macro_rules! impl_long_conversion {
    ($($t:ty),* $(,)?) => {
        $(
            impl<S> From<$t> for ShortOrLong<S, $t> {
                fn from(value: $t) -> Self {
                    Self::Long(value)
                }
            }

            impl<S> From<ShortOrLong<S, Self>> for $t
            where
                S: Into<Self>,
            {
                fn from(value: ShortOrLong<S, Self>) -> Self {
                    match value {
                        ShortOrLong::Short(short) => short.into(),
                        ShortOrLong::Long(long) => long,
                    }
                }
            }
        )*
    };
}

impl_long_conversion! {
    Include,
    Build,
    ConfigOrSecret,
    Ulimit,
    env_file::Config,
    Port,
    Mount,
}

impl<S, K, V> From<IndexMap<K, V>> for ShortOrLong<S, IndexMap<K, V>> {
    fn from(value: IndexMap<K, V>) -> Self {
        Self::Long(value)
    }
}

impl<S, K, V> From<ShortOrLong<S, Self>> for IndexMap<K, V>
where
    S: IntoIterator<Item = K>,
    K: Hash + Eq,
    V: Default,
{
    fn from(value: ShortOrLong<S, Self>) -> Self {
        match value {
            ShortOrLong::Short(short) => short
                .into_iter()
                .zip(std::iter::repeat_with(V::default))
                .collect(),
            ShortOrLong::Long(long) => long,
        }
    }
}

/// Single values ([`bool`], [`u8`], [`&str`], etc.), options, bytes, unit, newtype structs, enums,
/// and sequences are deserialized into the [`Short`] syntax. Maps are deserialized into the
/// [`Long`] syntax.
///
/// [`Short`]: Self::Short
/// [`Long`]: Self::Long
impl<'de, S, L> Deserialize<'de> for ShortOrLong<S, L>
where
    S: Deserialize<'de>,
    L: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor::new())
    }
}

/// [`de::Visitor`] for deserializing [`ShortOrLong`].
struct Visitor<S, L> {
    short: PhantomData<S>,
    long: PhantomData<L>,
}

impl<S, L> Visitor<S, L> {
    /// Create a new [`Visitor`].
    fn new() -> Self {
        Self {
            short: PhantomData,
            long: PhantomData,
        }
    }
}

/// Forward the [`de::Visitor`] implementation to the inner type and wrap it in the outer type.
macro_rules! forward_visitor {
    ($outer:path, $inner:ty, $($fn:ident: $v:ty,)*) => {
        $(
            fn $fn<E>(self, v: $v) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                let value = ::serde::de::IntoDeserializer::into_deserializer(v);
                let value: $inner = ::serde::Deserialize::deserialize(value)?;
                Ok($outer(value))
            }
        )*
    };
}

impl<'de, S, L> de::Visitor<'de> for Visitor<S, L>
where
    S: Deserialize<'de>,
    L: Deserialize<'de>,
{
    type Value = ShortOrLong<S, L>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a single value, map, sequence, or enum")
    }

    forward_visitor! {
        ShortOrLong::Short, S,
        visit_bool: bool,
        visit_i8: i8,
        visit_i16: i16,
        visit_i32: i32,
        visit_i64: i64,
        visit_i128: i128,
        visit_u8: u8,
        visit_u16: u16,
        visit_u32: u32,
        visit_u64: u64,
        visit_u128: u128,
        visit_f32: f32,
        visit_f64: f64,
        visit_char: char,
        visit_str: &str,
        visit_string: String,
        visit_bytes: &[u8],
        visit_byte_buf: Vec<u8>,
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = S::deserialize(BorrowedStrDeserializer::new(v))?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = S::deserialize(BorrowedBytesDeserializer::new(v))?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = S::deserialize(UnitDeserializer::new())?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = S::deserialize(deserializer)?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = S::deserialize(UnitDeserializer::new())?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = S::deserialize(deserializer)?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let value = S::deserialize(SeqAccessDeserializer::new(seq))?;
        Ok(ShortOrLong::Short(value))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let value = L::deserialize(MapAccessDeserializer::new(map))?;
        Ok(ShortOrLong::Long(value))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let value = S::deserialize(EnumAccessDeserializer::new(data))?;
        Ok(ShortOrLong::Short(value))
    }
}
