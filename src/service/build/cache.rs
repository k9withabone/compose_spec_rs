//! Provides [`Cache`] for the `cache_from` and `cache_to` fields of the long
//! [`Build`](super::Build) syntax.

use std::fmt::{self, Display, Formatter};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexMap;
use thiserror::Error;

use crate::{
    common::key_impls,
    impl_from_str,
    service::{image::InvalidImageError, Image},
};

/// Cache options for the `cache_from` and `cache_to` fields of the long [`Build`](super::Build)
/// syntax.
///
/// (De)serializes from/to an [`Image`] name or "type=TYPE[,KEY=VALUE...]".
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_from)
#[derive(SerializeDisplay, DeserializeTryFromString, Clone, Debug, PartialEq, Eq)]
#[serde(expecting = "an image name or string in the format \"type=TYPE[,KEY=VALUE,...]\"")]
pub struct Cache {
    /// The type of the cache.
    pub cache_type: CacheType,
    /// Cache options.
    pub options: IndexMap<CacheOption, CacheOption>,
}

impl Cache {
    /// Parse [`Cache`] from a string.
    ///
    /// The format is `{image}|type={cache_type}[,{key}={value}...]` where `image` is shorthand for
    /// `type=registry,ref={image}`.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is just an image name and it is not a valid [`Image`],
    /// otherwise the string must start with `type=`, its options must be valid [`CacheOption`]s,
    /// and if its [`CacheType::Registry`], it must contain a `ref` option with value being a valid
    /// [`Image`].
    pub fn parse<T>(cache: T) -> Result<Self, ParseCacheError>
    where
        T: AsRef<str> + Into<String>,
    {
        if cache.as_ref().contains(',') {
            Self::parse_str(cache.as_ref())
        } else {
            Image::parse(cache)
                .map(Self::from_image)
                .map_err(Into::into)
        }
    }

    /// Concrete implementation for [`Cache::parse()`] for string slices.
    fn parse_str(cache: &str) -> Result<Self, ParseCacheError> {
        // Format is "type=TYPE[,KEY=VALUE[,...]]"

        let mut options = cache.split(',');

        let cache_type = options
            .next()
            .expect("split has at least one element")
            .strip_prefix("type=")
            .ok_or(ParseCacheError::TypeFirst)?;

        let mut options: IndexMap<CacheOption, CacheOption> = options
            .map(|option| {
                let (key, value) = option.split_once('=').unwrap_or((option, ""));
                Ok((key.parse()?, value.parse()?))
            })
            .collect::<Result<_, ParseCacheError>>()?;

        let cache_type = match cache_type {
            "registry" => options
                .shift_remove("ref")
                .ok_or(ParseCacheError::MissingRef)?
                .0
                .try_into()
                .map(CacheType::Registry)?,
            other => other.parse().map(CacheType::Other)?,
        };

        Ok(Self {
            cache_type,
            options,
        })
    }

    /// Create a [`Cache`] from an image ref.
    ///
    /// Shorthand for "type=registry,ref={image}".
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), compose_spec::service::build::ParseCacheError> {
    /// use indexmap::IndexMap;
    /// use compose_spec::service::{build::{Cache, CacheType}, Image};
    ///
    /// let image = Image::parse("image")?;
    /// let cache = Cache::from_image(image.clone());
    ///
    /// assert_eq!(cache,"type=registry,ref=image".parse()?);
    ///
    /// assert_eq!(
    ///     cache,
    ///     Cache {
    ///         cache_type: CacheType::Registry(image),
    ///         options: IndexMap::default(),
    ///     },
    /// );
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn from_image(image: Image) -> Self {
        CacheType::from(image).into()
    }

    /// Convert cache into a map of options.
    ///
    /// Inserts the [`CacheType`] into the options at the beginning.
    #[must_use]
    pub fn into_options(self) -> IndexMap<CacheOption, CacheOption> {
        let Self {
            cache_type,
            mut options,
        } = self;

        match cache_type {
            CacheType::Registry(image) => {
                let (key, value) = CacheOption::pair_from_image(image);
                options.shift_insert(0, key, value);

                options.shift_insert(
                    0,
                    CacheOption("type".into()),
                    CacheOption("registry".into()),
                );

                options
            }
            CacheType::Other(cache_type) => {
                options.shift_insert(0, CacheOption("type".into()), cache_type);
                options
            }
        }
    }
}

impl_from_str!(Cache => ParseCacheError);

/// Error returned when parsing a [`Cache`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseCacheError {
    /// Error parsing [`Image`] for [`CacheType::Registry`].
    #[error("error parsing cache image ref")]
    Image(#[from] InvalidImageError),

    /// Cache did not start with `type=`.
    #[error("cache options must start with `type=` if not an image")]
    TypeFirst,

    /// Error parsing [`CacheOption`].
    #[error("error parsing cache option")]
    CacheOption(#[from] InvalidCacheOptionError),

    /// [`CacheType::Registry`] requires a `ref` option.
    #[error("cache type `registry` missing required `ref` option")]
    MissingRef,
}

impl From<CacheType> for Cache {
    fn from(cache_type: CacheType) -> Self {
        Self {
            cache_type,
            options: IndexMap::default(),
        }
    }
}

impl From<Image> for Cache {
    fn from(image: Image) -> Self {
        Self::from_image(image)
    }
}

impl Display for Cache {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            cache_type,
            options,
        } = self;

        if options.is_empty() {
            if let CacheType::Registry(image) = cache_type {
                return Display::fmt(image, f);
            }
        }

        Display::fmt(cache_type, f)?;

        for (key, value) in options {
            write!(f, ",{key}={value}")?;
        }

        Ok(())
    }
}

/// [`Cache`] type.
///
/// The [`Display`] format is `type=registry,ref={image}` or `type={other}`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_from)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::module_name_repetitions)]
pub enum CacheType {
    /// Retrieve build cache from an OCI image set by option "ref".
    Registry(Image),

    /// Some other cache type.
    Other(CacheOption),
}

impl CacheType {
    /// Returns `true` if the cache type is [`Registry`].
    ///
    /// [`Registry`]: CacheType::Registry
    #[must_use]
    pub const fn is_registry(&self) -> bool {
        matches!(self, Self::Registry(_))
    }

    /// Returns [`Some`] if the cache type is [`Registry`].
    ///
    /// [`Registry`]: CacheType::Registry
    #[must_use]
    pub const fn as_registry(&self) -> Option<&Image> {
        if let Self::Registry(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the cache type is [`Other`].
    ///
    /// [`Other`]: CacheType::Other
    #[must_use]
    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other(..))
    }

    /// Returns [`Some`] if the cache type is [`Other`].
    ///
    /// [`Other`]: CacheType::Other
    #[must_use]
    pub const fn as_other(&self) -> Option<&CacheOption> {
        if let Self::Other(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<Image> for CacheType {
    fn from(image: Image) -> Self {
        Self::Registry(image)
    }
}

impl Display for CacheType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Registry(image) => write!(f, "type=registry,ref={image}"),
            Self::Other(other) => write!(f, "type={other}"),
        }
    }
}

/// An option for a [`Cache`].
///
/// Cache options cannot be empty or contain whitespace.
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[allow(clippy::module_name_repetitions)]
pub struct CacheOption(Box<str>);

impl CacheOption {
    /// Create a new [`CacheOption`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `option` is empty or contains whitespace.
    pub fn new<T>(option: T) -> Result<Self, InvalidCacheOptionError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        if option.as_ref().is_empty() {
            Err(InvalidCacheOptionError::Empty)
        } else if option.as_ref().contains(char::is_whitespace) {
            Err(InvalidCacheOptionError::Whitespace)
        } else {
            Ok(Self(option.into()))
        }
    }

    /// Return a pair of cache options appropriate for inserting into a [`Cache`]'s `options` map.
    fn pair_from_image(image: Image) -> (Self, Self) {
        (Self("ref".into()), image.into())
    }
}

/// Error returned when creating a new [`CacheOption`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidCacheOptionError {
    /// Cache options cannot be empty.
    #[error("cache options cannot be empty")]
    Empty,

    /// Cache options cannot container whitespace.
    #[error("cache options cannot contain whitespace")]
    Whitespace,
}

key_impls!(CacheOption => InvalidCacheOptionError);

impl From<Image> for CacheOption {
    fn from(value: Image) -> Self {
        // Images are never empty or contain whitespace.
        Self(value.into_inner().into_boxed_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let cache = Cache::from_image("image".parse().unwrap());
        let string = cache.to_string();
        assert_eq!(string, "image");
        assert_eq!(cache, string.parse().unwrap());
    }
}
