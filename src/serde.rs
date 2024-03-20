pub(crate) mod display_from_str_option;
pub(crate) mod duration_option;
pub(crate) mod duration_us_option;

use std::{
    error::Error,
    fmt::{self, Formatter, Write},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    de::{
        self, value::SeqAccessDeserializer, Expected, IntoDeserializer, SeqAccess, Unexpected,
        Visitor,
    },
    Deserialize, Deserializer,
};

pub(crate) const fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) const fn skip_true(bool: &bool) -> bool {
    *bool
}

/// Implement [`Visitor`] functions by forwarding to `visit`.
macro_rules! forward_visitor {
    ($visit:ident, $($f:ident: $ty:ty,)*) => {
        $(
            fn $f<E: ::serde::de::Error>(self, v: $ty) -> ::std::result::Result<Self::Value, E> {
                self.$visit(v.try_into().map_err(E::custom)?)
            }
        )*
    };
}

pub(crate) use forward_visitor;

#[derive(Debug)]
pub(crate) struct ValueEnumVisitor<B = (), I = (), U = (), F = (), S = ()> {
    expecting: &'static str,
    visit_bool: Option<B>,
    visit_i64: Option<I>,
    visit_u64: Option<U>,
    visit_f64: Option<F>,
    visit_string: Option<S>,
}

impl<B, I, U, F, S> ValueEnumVisitor<B, I, U, F, S> {
    pub fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            visit_bool: None,
            visit_i64: None,
            visit_u64: None,
            visit_f64: None,
            visit_string: None,
        }
    }

    pub fn bool(mut self, visit: B) -> Self {
        self.visit_bool = Some(visit);
        self
    }

    pub fn i64(mut self, visit: I) -> Self {
        self.visit_i64 = Some(visit);
        self
    }

    pub fn u64(mut self, visit: U) -> Self {
        self.visit_u64 = Some(visit);
        self
    }

    pub fn f64(mut self, visit: F) -> Self {
        self.visit_f64 = Some(visit);
        self
    }

    pub fn string(mut self, visit: S) -> Self {
        self.visit_string = Some(visit);
        self
    }

    pub fn deserialize<'de, D, V>(self, deserializer: D) -> Result<V, D::Error>
    where
        D: Deserializer<'de>,
        Self: Visitor<'de, Value = V>,
    {
        deserializer.deserialize_any(self)
    }

    fn invalid_type<E: de::Error>(&self, unexpected: Unexpected) -> E
    where
        Self: Expected,
    {
        de::Error::invalid_type(unexpected, self)
    }
}

impl<'de, B, I, U, F, S, V> Visitor<'de> for ValueEnumVisitor<B, I, U, F, S>
where
    B: FnOnce(bool) -> V,
    I: FnOnce(i64) -> V,
    U: FnOnce(u64) -> V,
    F: FnOnce(f64) -> V,
    S: FnOnce(String) -> V,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_bool) = self.visit_bool {
            Ok(visit_bool(v))
        } else {
            Err(self.invalid_type(Unexpected::Bool(v)))
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_i64) = self.visit_i64 {
            Ok(visit_i64(v))
        } else {
            Err(self.invalid_type(Unexpected::Signed(v)))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_u64) = self.visit_u64 {
            Ok(visit_u64(v))
        } else {
            Err(self.invalid_type(Unexpected::Unsigned(v)))
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_f64) = self.visit_f64 {
            Ok(visit_f64(v))
        } else {
            Err(self.invalid_type(Unexpected::Float(v)))
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_string) = self.visit_string {
            Ok(visit_string(v.to_owned()))
        } else {
            Err(self.invalid_type(Unexpected::Str(v)))
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(visit_string) = self.visit_string {
            Ok(visit_string(v))
        } else {
            Err(self.invalid_type(Unexpected::Str(&v)))
        }
    }
}

/// A [`Visitor`] for deserializing a single item or a list.
#[derive(Debug)]
pub(crate) struct ItemOrListVisitor<V, I, L = Vec<I>> {
    expecting: &'static str,
    value: PhantomData<V>,
    item: PhantomData<I>,
    list: PhantomData<L>,
}

impl<V, I, L> ItemOrListVisitor<V, I, L> {
    /// Create a new [`ItemOrListVisitor`].
    ///
    /// `expecting` should complete the sentence "This Visitor expects to receive ...",
    /// the [`Default`] implementation uses "a single value or sequence".
    pub fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            value: PhantomData,
            item: PhantomData,
            list: PhantomData,
        }
    }
}

impl<V, I, L> Default for ItemOrListVisitor<V, I, L> {
    fn default() -> Self {
        Self::new("a single value or sequence")
    }
}

impl<'de, V, I, L> ItemOrListVisitor<V, I, L>
where
    I: Into<V> + Deserialize<'de>,
    L: Into<V> + Deserialize<'de>,
{
    /// Alias for `deserializer.deserialize_any(visitor)`.
    pub fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
        deserializer.deserialize_any(self)
    }
}

/// Implement [`Visitor`] by using [`IntoDeserializer`] on the input, deserializing into `t`, and
/// then turning it [`Into`] the [`Value`](Visitor::Value).
macro_rules! visit_item {
    (item: $t:ty, $($f:ident: $ty:ty,)*) => {
        $(
            fn $f<E: de::Error>(self, v: $ty) -> Result<Self::Value, E> {
                <$t>::deserialize(v.into_deserializer()).map(Into::into)
            }
        )*
    };
}

impl<'de, V, I, L> Visitor<'de> for ItemOrListVisitor<V, I, L>
where
    I: Into<V> + Deserialize<'de>,
    L: Into<V> + Deserialize<'de>,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    visit_item! {
        item: I,
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

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        L::deserialize(SeqAccessDeserializer::new(seq)).map(Into::into)
    }
}

/// A [`Visitor`] which deserializes a type using its [`FromStr`] implementation.
#[derive(Debug)]
pub(crate) struct FromStrVisitor<V> {
    expecting: &'static str,
    value: PhantomData<V>,
}

impl<V> FromStrVisitor<V> {
    /// Create a new [`FromStrVisitor`].
    ///
    /// `expecting` should complete the sentence "This Visitor expects to receive ...",
    /// the [`Default`] implementation uses "a string".
    pub fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            value: PhantomData,
        }
    }
}

impl<V> FromStrVisitor<V>
where
    V: FromStr,
    V::Err: Error,
{
    /// Alias for `deserializer.deserialize_str(visitor)`.
    pub fn deserialize<'de, D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl<V> Default for FromStrVisitor<V> {
    fn default() -> Self {
        Self::new("a string")
    }
}

impl<'de, V> Visitor<'de> for FromStrVisitor<V>
where
    V: FromStr,
    V::Err: Error,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.parse().map_err(error_chain)
    }
}

/// A [`Visitor`] which deserializes a type using its [`TryFrom<String>`] and [`TryFrom<&str>`]
/// implementations.
#[derive(Debug)]
pub(crate) struct TryFromStringVisitor<V> {
    expecting: &'static str,
    value: PhantomData<V>,
}

impl<V> TryFromStringVisitor<V> {
    /// Create a new [`TryFromStringVisitor`].
    ///
    /// `expecting` should complete the sentence "This Visitor expects to receive ...",
    /// the [`Default`] implementation uses "a string".
    pub fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            value: PhantomData,
        }
    }
}

impl<V> TryFromStringVisitor<V>
where
    String: TryInto<V>,
    for<'a> &'a str: TryInto<V>,
    <String as TryInto<V>>::Error: Error,
    for<'a> <&'a str as TryInto<V>>::Error: Error,
{
    /// Alias for `deserializer.deserialize_string(visitor)`.
    pub fn deserialize<'de, D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
        deserializer.deserialize_string(self)
    }
}

impl<V> Default for TryFromStringVisitor<V> {
    fn default() -> Self {
        Self::new("a string")
    }
}

impl<'de, V> Visitor<'de> for TryFromStringVisitor<V>
where
    String: TryInto<V>,
    for<'a> &'a str: TryInto<V>,
    <String as TryInto<V>>::Error: Error,
    for<'a> <&'a str as TryInto<V>>::Error: Error,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.try_into().map_err(error_chain)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        v.try_into().map_err(error_chain)
    }
}

/// A [`Visitor`] for deserializing via [`FromStr`] or from a [`u16`].
pub(crate) struct FromStrOrU16Visitor<V> {
    expecting: &'static str,
    value: PhantomData<V>,
}

impl<V> FromStrOrU16Visitor<V> {
    /// Create a new [`FromStrOrU16Visitor`].
    ///
    /// `expecting` should complete the sentence "This Visitor expects to receive ...",
    /// the [`Default`] implementation uses "a string or integer".
    pub fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            value: PhantomData,
        }
    }
}

impl<V> FromStrOrU16Visitor<V>
where
    u16: Into<V>,
    V: FromStr,
    V::Err: Error,
{
    /// Alias for `deserializer.deserialize_any(visitor)`.
    pub fn deserialize<'de, D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<V> Default for FromStrOrU16Visitor<V> {
    fn default() -> Self {
        Self::new("a string or integer")
    }
}

impl<'de, V> Visitor<'de> for FromStrOrU16Visitor<V>
where
    u16: Into<V>,
    V: FromStr,
    V::Err: Error,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    forward_visitor! {
        visit_u16,
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
        v.parse().map_err(error_chain)
    }
}

/// Map a type implementing [`Error`] to one implementing [`de::Error`] by using
/// [`de::Error::custom()`] with a string of all of the error's sources.
pub(crate) fn error_chain<T, E>(error: T) -> E
where
    T: Error,
    E: de::Error,
{
    let mut output = String::new();
    write!(output, "{error}").expect("write to string never fails");

    if let Some(source) = error.source() {
        // TODO: replace with [`Error::sources()`] when stable.
        for error in ErrorSources::new(source) {
            write!(output, ": {error}").expect("write to string never fails");
        }
    }

    de::Error::custom(output)
}

#[derive(Debug, Clone)]
struct ErrorSources<'a> {
    current: Option<&'a (dyn Error + 'static)>,
}

impl<'a> ErrorSources<'a> {
    fn new(error: &'a (dyn Error + 'static)) -> Self {
        Self {
            current: Some(error),
        }
    }
}

impl<'a> Iterator for ErrorSources<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(Error::source);
        current
    }
}
