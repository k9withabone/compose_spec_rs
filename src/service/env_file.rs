//! Provides [`EnvFile`] for the `env_file` field of [`Service`](super::Service).

use std::path::PathBuf;

use compose_spec_macros::{AsShort, FromShort};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    serde::{default_true, skip_true, ItemOrListVisitor},
    AsShort, ShortOrLong,
};

/// [`List`](EnvFile::List) of environment file paths.
type List = Vec<ShortOrLong<PathBuf, Config>>;

/// A single or list of paths to environment files that add to the environment variables of the
/// [`Service`](super::Service) container.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#env_file)
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum EnvFile {
    /// Path to a single environment file.
    Single(PathBuf),

    /// List of environment file paths.
    List(List),
}

impl EnvFile {
    /// Returns [`Some`] if there is only a [`Single`](Self::Single) environment file path, or only
    /// one in the [`List`](Self::List) that is required.
    pub fn as_single(&self) -> Option<&PathBuf> {
        match self {
            Self::Single(path) => Some(path),
            Self::List(list) if list.len() == 1 => list.first().and_then(ShortOrLong::as_short),
            Self::List(_) => None,
        }
    }

    /// Convert into a list of environment file paths.
    ///
    /// If [`Single`](Self::Single), a new [`Vec`] is created.
    #[must_use]
    pub fn into_list(self) -> List {
        match self {
            Self::Single(path) => vec![path.into()],
            Self::List(list) => list,
        }
    }
}

impl From<PathBuf> for EnvFile {
    fn from(value: PathBuf) -> Self {
        Self::Single(value)
    }
}

impl From<String> for EnvFile {
    fn from(value: String) -> Self {
        Self::Single(value.into())
    }
}

impl From<List> for EnvFile {
    fn from(value: List) -> Self {
        Self::List(value)
    }
}

impl<I> FromIterator<I> for EnvFile
where
    I: Into<ShortOrLong<PathBuf, Config>>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self::List(iter.into_iter().map(Into::into).collect())
    }
}

impl<'de> Deserialize<'de> for EnvFile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ItemOrListVisitor::<_, PathBuf, List>::new("a string or list of strings or maps")
            .deserialize(deserializer)
    }
}

/// [`EnvFile`] configuration, allows for specifying that it does not need to exist.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#env_file)
#[derive(Serialize, Deserialize, AsShort, FromShort, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path of the environment file on the host, relative to the [`Compose`](crate::Compose) file's
    /// directory.
    #[as_short(short)]
    pub path: PathBuf,

    /// Whether it is required for the environment file to exist.
    ///
    /// Default is `true`.
    #[serde(default = "default_true", skip_serializing_if = "skip_true")]
    #[as_short(default = default_true, if_fn = skip_true)]
    pub required: bool,
}

impl From<String> for Config {
    fn from(value: String) -> Self {
        PathBuf::from(value).into()
    }
}
