//! Provides [`Secret`] for the top-level `secrets` field of a [`Compose`](super::Compose) file.

use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use indexmap::IndexMap;
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

use crate::{Extensions, ListOrMap, MapKey, Resource, StringOrNumber};

impl From<Secret> for Resource<Secret> {
    fn from(value: Secret) -> Self {
        Self::Compose(value)
    }
}

/// Sensitive data that a [`Service`](super::Service) may be granted access to.
///
/// A secret is similar to a [`Config`](super::Config), but for sensitive data.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/09-secrets.md)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Secret {
    /// Source of the secret's contents.
    ///
    /// Represents the `file` or `environment` fields of the compose secret spec.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub source: Source,

    /// Add metadata to the secret.
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Which driver to use for this secret.
    ///
    /// Default and available values are platform specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// Driver-dependent options.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub driver_opts: IndexMap<MapKey, StringOrNumber>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl From<Source> for Secret {
    fn from(source: Source) -> Self {
        Self {
            source,
            labels: ListOrMap::default(),
            driver: None,
            driver_opts: IndexMap::default(),
            extensions: Extensions::default(),
        }
    }
}

/// Source of a [`Secret`].
///
/// (De)serializes from/to a struct with a `file`, `environment`, or `content` field.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Create the secret with the contents of the file at the specified path.
    File(PathBuf),
    /// Create the secret with the value of an environment variable.
    Environment(String),
}

impl Source {
    /// Struct name for serializing.
    const NAME: &'static str = "Source";
}

impl From<PathBuf> for Source {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

/// Possible [`Source`] fields.
#[derive(Debug, Clone, Copy)]
enum SourceField {
    /// [`Source::File`] / `file`
    File,
    /// [`Source::Environment`] / `environment`
    Environment,
}

impl SourceField {
    /// [`Source`] field name as a static string slice.
    const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Environment => "environment",
        }
    }
}

impl Display for SourceField {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// [`format_args`] with all [`SourceField`]s.
macro_rules! format_fields {
    ($args:literal) => {
        format_args!($args, SourceField::File, SourceField::Environment,)
    };
}

impl Serialize for Source {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(Self::NAME, 1)?;

        match self {
            Self::File(source) => state.serialize_field(SourceField::File.as_str(), source)?,
            Self::Environment(source) => {
                state.serialize_field(SourceField::Environment.as_str(), source)?;
            }
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let SourceFlat { file, environment } = SourceFlat::deserialize(deserializer)?;

        match (file, environment) {
            (Some(file), None) => Ok(file.into()),
            (None, Some(environment)) => Ok(Self::Environment(environment)),
            (None, None) => Err(de::Error::custom(format_fields!(
                "missing required field `{}` or `{}`"
            ))),
            (Some(_), Some(_)) => Err(de::Error::custom(format_fields!(
                "can only set one of `{}` or `{}`"
            ))),
        }
    }
}

/// Flattened version of [`Source`].
#[derive(Deserialize)]
#[serde(
    rename = "Source",
    expecting = "a struct with a `file` or `environment` field"
)]
struct SourceFlat {
    /// [`Source::File`]
    #[serde(default)]
    file: Option<PathBuf>,

    /// [`Source::Environment`]
    #[serde(default)]
    environment: Option<String>,
}
