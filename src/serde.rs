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
    de::{self, value::SeqAccessDeserializer, IntoDeserializer, SeqAccess, Visitor},
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
pub(crate) struct ValueEnumVisitor<U = (), I = (), F = (), B = (), S = ()> {
    expecting: &'static str,
    visit_u64: U,
    visit_i64: I,
    visit_f64: F,
    visit_bool: B,
    visit_string: S,
}

impl ValueEnumVisitor {
    pub(crate) const fn new(expecting: &'static str) -> Self {
        Self {
            expecting,
            visit_u64: (),
            visit_i64: (),
            visit_f64: (),
            visit_bool: (),
            visit_string: (),
        }
    }
}

impl<I, F, B, S> ValueEnumVisitor<(), I, F, B, S> {
    pub(crate) fn u64<U, V>(self, visit_u64: U) -> ValueEnumVisitor<U, I, F, B, S>
    where
        U: FnOnce(u64) -> V,
    {
        let Self {
            expecting,
            visit_u64: (),
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        } = self;

        ValueEnumVisitor {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        }
    }
}

impl<U, F, B, S> ValueEnumVisitor<U, (), F, B, S> {
    pub(crate) fn i64<I, V>(self, visit_i64: I) -> ValueEnumVisitor<U, I, F, B, S>
    where
        I: FnOnce(i64) -> V,
    {
        let Self {
            expecting,
            visit_u64,
            visit_i64: (),
            visit_f64,
            visit_bool,
            visit_string,
        } = self;

        ValueEnumVisitor {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        }
    }
}

impl<U, I, B, S> ValueEnumVisitor<U, I, (), B, S> {
    pub(crate) fn f64<F, V>(self, visit_f64: F) -> ValueEnumVisitor<U, I, F, B, S>
    where
        F: FnOnce(f64) -> V,
    {
        let Self {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64: (),
            visit_bool,
            visit_string,
        } = self;

        ValueEnumVisitor {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        }
    }
}

impl<U, I, F, S> ValueEnumVisitor<U, I, F, (), S> {
    pub(crate) fn bool<B, V>(self, visit_bool: B) -> ValueEnumVisitor<U, I, F, B, S>
    where
        B: FnOnce(bool) -> V,
    {
        let Self {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool: (),
            visit_string,
        } = self;

        ValueEnumVisitor {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        }
    }
}

impl<U, I, F, B> ValueEnumVisitor<U, I, F, B, ()> {
    pub(crate) fn string<S, V>(self, visit_string: S) -> ValueEnumVisitor<U, I, F, B, S>
    where
        S: FnOnce(String) -> V,
    {
        let Self {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string: (),
        } = self;

        ValueEnumVisitor {
            expecting,
            visit_u64,
            visit_i64,
            visit_f64,
            visit_bool,
            visit_string,
        }
    }
}

impl<U, I, F, B, S> ValueEnumVisitor<U, I, F, B, S> {
    pub(crate) fn deserialize<'de, V, D>(self, deserializer: D) -> Result<V, D::Error>
    where
        D: Deserializer<'de>,
        Self: Visitor<'de, Value = V>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de, U, I, F, B, S, V> Visitor<'de> for ValueEnumVisitor<U, I, F, B, S>
where
    U: FnOnce(u64) -> V,
    I: FnOnce(i64) -> V,
    F: FnOnce(f64) -> V,
    B: FnOnce(bool) -> V,
    S: FnOnce(String) -> V,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        let Self { visit_bool, .. } = self;
        Ok(visit_bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        let Self { visit_i64, .. } = self;
        Ok(visit_i64(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let Self { visit_u64, .. } = self;
        Ok(visit_u64(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let Self { visit_f64, .. } = self;
        Ok(visit_f64(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        let Self { visit_string, .. } = self;
        Ok(visit_string(v))
    }
}

impl<'de, U, I, F, V> Visitor<'de> for ValueEnumVisitor<U, I, F>
where
    U: FnOnce(u64) -> V,
    I: FnOnce(i64) -> V,
    F: FnOnce(f64) -> V,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        let Self { visit_i64, .. } = self;
        Ok(visit_i64(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let Self { visit_u64, .. } = self;
        Ok(visit_u64(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let Self { visit_f64, .. } = self;
        Ok(visit_f64(v))
    }
}

impl<'de, U, I, F, S, V> Visitor<'de> for ValueEnumVisitor<U, I, F, (), S>
where
    U: FnOnce(u64) -> V,
    I: FnOnce(i64) -> V,
    F: FnOnce(f64) -> V,
    S: FnOnce(String) -> V,
{
    type Value = V;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        let Self { visit_i64, .. } = self;
        Ok(visit_i64(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let Self { visit_u64, .. } = self;
        Ok(visit_u64(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let Self { visit_f64, .. } = self;
        Ok(visit_f64(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        let Self { visit_string, .. } = self;
        Ok(visit_string(v))
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
    pub(crate) const fn new(expecting: &'static str) -> Self {
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
    pub(crate) fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
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
    pub(crate) const fn new(expecting: &'static str) -> Self {
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
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<V, D::Error> {
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
    pub(crate) const fn new(expecting: &'static str) -> Self {
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
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<V, D::Error> {
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
    pub(crate) const fn new(expecting: &'static str) -> Self {
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
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<V, D::Error> {
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
