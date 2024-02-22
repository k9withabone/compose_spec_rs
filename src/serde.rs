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
    de::{self, Expected, Unexpected, Visitor},
    Deserializer,
};

pub(crate) const fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) const fn skip_true(bool: &bool) -> bool {
    *bool
}

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

/// Map a type implementing [`Error`] to one implementing [`de::Error`] by using
/// [`de::Error::custom()`] with a string of all of the error's sources.
fn error_chain<T, E>(error: T) -> E
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
