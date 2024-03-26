//! Provides [`Cache`] for the `cache_from` and `cache_to` fields of the long
//! [`Build`](super::Build) syntax.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use indexmap::{indexmap, IndexMap};
use thiserror::Error;

use crate::{impl_from_str, InvalidMapKeyError, MapKey};

/// Cache options for the `cache_from` and `cache_to` fields of the long [`Build`](super::Build)
/// syntax.
///
/// (De)serializes from/to "type=TYPE[,KEY=VALUE[,...]]", or deserializes/parses from an image name,
/// see [`Cache::from_image()`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_from)
#[derive(SerializeDisplay, DeserializeFromStr, Default, Clone, Debug, PartialEq, Eq)]
#[serde(expecting = "an image name or a string of the format \"type=TYPE[,KEY=VALUE,...]\"")]
pub struct Cache {
    kind: Kind,
    options: IndexMap<MapKey, Box<str>>,
}

impl Cache {
    /// Create a new [`Cache`].
    ///
    /// # Errors
    ///
    /// Returns an error if a key in `options` fails to convert into a [`MapKey`],
    /// or if `cache_type` is [`Registry`](Kind::Registry) and `options` is missing a "ref" option.
    pub fn new<O, K, V>(cache_type: Kind, options: O) -> Result<Self, Error>
    where
        O: IntoIterator<Item = (K, V)>,
        K: TryInto<MapKey>,
        Error: From<K::Error>,
        V: Into<Box<str>>,
    {
        let options: IndexMap<_, _> = options
            .into_iter()
            .map(|(key, value)| key.try_into().map(|key| (key, value.into())))
            .collect::<Result<_, _>>()?;

        if cache_type.is_registry() {
            let ref_option = options.get("ref");
            if ref_option.is_none() || ref_option.is_some_and(|option| option.is_empty()) {
                return Err(Error::RegistryMissingRef);
            }
        }

        Ok(Self {
            kind: cache_type,
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
    /// use compose_spec::service::build::{Cache, CacheType};
    ///
    /// assert_eq!(
    ///     Cache::from_image("image"),
    ///     "type=registry,ref=image".parse().unwrap(),
    /// );
    ///
    /// assert_eq!(
    ///     Cache::from_image("image"),
    ///     Cache::new(CacheType::Registry, [("ref", "image")]).unwrap(),
    /// );
    /// ```
    #[must_use]
    pub fn from_image(image: &str) -> Self {
        let key = MapKey::new_unchecked("ref");
        Self {
            kind: Kind::Registry,
            options: indexmap! {
                key => image.into(),
            },
        }
    }

    /// The value of the cache "type" field.
    #[doc(alias = "kind", alias = "type")]
    #[must_use]
    pub fn cache_type(&self) -> &Kind {
        &self.kind
    }

    /// Cache options.
    #[must_use]
    pub fn options(&self) -> &IndexMap<MapKey, Box<str>> {
        &self.options
    }
}

/// Error returned when creating a [`Cache`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// [`Registry`](Kind::Registry) cache type given without a corresponding "ref" option.
    #[error("caches with type \"registry\" must have a \"ref\" option")]
    RegistryMissingRef,

    /// Option keys must be valid [`MapKey`]s.
    #[error("invalid option key")]
    OptionKey(#[from] InvalidMapKeyError),
}

impl FromStr for Cache {
    type Err = ParseCacheError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "NAME | type=TYPE[,KEY=VALUE[,...]]", where NAME is an image name.

        let mut options = s.split(',');
        let kind = options.next().expect("Split has at least one element");

        if let Some(kind) = kind.strip_prefix("type=") {
            let options: Vec<_> = options
                .map(|option| {
                    option
                        .split_once('=')
                        .filter(|(_, value)| !value.is_empty())
                        .ok_or(ParseCacheError::OptionValueMissing)
                })
                .collect::<Result<_, _>>()?;

            Self::new(kind.into(), options).map_err(Into::into)
        } else {
            Ok(Self::from_image(s))
        }
    }
}

impl Display for Cache {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self { kind, options } = self;

        write!(f, "type={kind}")?;

        for (key, value) in options {
            write!(f, ",{key}={value}")?;
        }

        Ok(())
    }
}

/// Error returned when parsing a [`Cache`] from a string.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseCacheError {
    /// Error while creating [`Cache`].
    #[error("invalid cache options")]
    Cache(#[from] Error),

    /// An option was missing a value.
    #[error("cache options must have a value")]
    OptionValueMissing,
}

/// Cache type, all compose implementations must support the [`Registry`](Kind::Registry) type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_from)
#[doc(alias = "CacheKind")]
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Retrieve build cache from an OCI image set by option "ref".
    #[default]
    Registry,

    /// Some other cache type.
    Other(String),
}

impl Kind {
    /// [`Self::Registry`] string value.
    const REGISTRY: &'static str = "registry";

    /// Parse [`CacheType`](Self) from a string.
    pub fn parse<T>(cache_kind: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match cache_kind.as_ref() {
            Self::REGISTRY => Self::Registry,
            _ => Self::Other(cache_kind.into()),
        }
    }

    /// Returns `true` if the cache type is [`Registry`].
    ///
    /// [`Registry`]: Kind::Registry
    #[must_use]
    pub fn is_registry(&self) -> bool {
        matches!(self, Self::Registry)
    }

    /// Cache type as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Registry => Self::REGISTRY,
            Self::Other(kind) => kind,
        }
    }
}

impl_from_str!(Kind);

impl AsRef<str> for Kind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<Kind> for String {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Registry => value.as_str().to_owned(),
            Kind::Other(value) => value,
        }
    }
}

impl From<Kind> for Cow<'static, str> {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Registry => Self::Borrowed(Kind::REGISTRY),
            Kind::Other(other) => Self::Owned(other),
        }
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let cache = Cache::from_image("test");
        let string = cache.to_string();
        assert_eq!(string, "type=registry,ref=test");
        assert_eq!(cache, string.parse().unwrap());
    }
}
