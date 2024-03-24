//! Provides [`Config`] for the top-level `configs` field of a [`Compose`](super::Compose) file.

use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

use crate::{Extensions, ListOrMap, Resource};

impl From<Config> for Resource<Config> {
    fn from(value: Config) -> Self {
        Self::Compose(value)
    }
}

/// Configuration which allow a [`Service`] to adapt its behavior without needing to rebuild the
/// container image.
///
/// Like [`Volume`]s, configs are mounted as files into the [`Service`]'s container's file system.
/// The location of the mount point within the container defaults to `/<config-name>` in Linux
/// containers, and `C:\<config-name>` in Windows containers.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
///
/// [`Service`]: super::Service
/// [`Volume`]: super::Volume
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    /// Source of the config's contents.
    ///
    /// Represents the `file`, `environment`, or `content` fields of the compose config spec.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub source: Source,

    /// Add metadata to the config.
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl From<Source> for Config {
    fn from(source: Source) -> Self {
        Self {
            source,
            labels: ListOrMap::default(),
            extensions: Extensions::default(),
        }
    }
}

/// Source of a [`Config`].
///
/// (De)serializes from/to a struct with a `file`, `environment`, or `content` field.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Create the config with the contents of the file at the specified path.
    File(PathBuf),
    /// Create the config with the value of an environment variable.
    Environment(String),
    /// Create the config with the given contents.
    Content(String),
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
    /// [`Source::Content`] / `content`
    Content,
}

impl SourceField {
    const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Environment => "environment",
            Self::Content => "content",
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
        format_args!(
            $args,
            SourceField::File,
            SourceField::Environment,
            SourceField::Content,
        )
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
            Self::Content(source) => {
                state.serialize_field(SourceField::Content.as_str(), source)?;
            }
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let SourceFlat {
            file,
            environment,
            content,
        } = SourceFlat::deserialize(deserializer)?;

        match (file, environment, content) {
            (Some(file), None, None) => Ok(file.into()),
            (None, Some(environment), None) => Ok(Self::Environment(environment)),
            (None, None, Some(content)) => Ok(Self::Content(content)),
            (None, None, None) => Err(de::Error::custom(format_fields!(
                "missing required field `{}`, `{}`, or `{}`"
            ))),
            _ => Err(de::Error::custom(format_fields!(
                "can only set one of `{}`, `{}`, or `{}`"
            ))),
        }
    }
}

/// Flattened version of [`Source`].
#[derive(Deserialize)]
#[serde(
    rename = "Source",
    expecting = "a struct with a `file`, `environment`, or `content` field"
)]
struct SourceFlat {
    /// [`Source::File`]
    #[serde(default)]
    file: Option<PathBuf>,

    /// [`Source::Environment`]
    #[serde(default)]
    environment: Option<String>,

    /// [`Source::Content`]
    #[serde(default)]
    content: Option<String>,
}
