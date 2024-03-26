//! Provides [`Dockerfile`] for (de)serializing from/to the
//! [`dockerfile`](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile) and
//! [`dockerfile_inline`](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile_inline)
//! fields of the long [`Build`](super::Build) syntax.

use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// Represents either the `dockerfile` or `dockerfile_inline` fields of the long [`Build`] syntax.
///
/// These fields conflict with each other so they are represented as an enum.
///
/// This (de)serializes from/to a struct with either the `dockerfile` or `dockerfile_inline` field,
/// which is flattened into [`Build`].
///
/// [`Build`]: super::Build
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dockerfile {
    /// Set an alternate Dockerfile/Containerfile.
    /// A relative path is resolved from the build context.
    ///
    /// Represents the `dockerfile` field.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile)
    File(PathBuf),

    /// Define the Dockerfile/Containerfile content as an inlined string.
    ///
    /// Represents the `dockerfile_inline` field.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile_inline)
    Inline(String),
}

impl Dockerfile {
    /// Struct name for (de)serializing.
    const NAME: &'static str = "Dockerfile";
}

/// Possible [`Dockerfile`] fields.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    /// [`Dockerfile::File`] / `dockerfile`
    Dockerfile,
    /// [`Dockerfile::Inline`] / `dockerfile_inline`
    DockerfileInline,
}

impl Field {
    /// Field identifier as static string slice.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Dockerfile => "dockerfile",
            Self::DockerfileInline => "dockerfile_inline",
        }
    }
}

impl From<&Dockerfile> for Field {
    fn from(value: &Dockerfile) -> Self {
        match value {
            Dockerfile::File(_) => Self::Dockerfile,
            Dockerfile::Inline(_) => Self::DockerfileInline,
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// [`format_args`] with all [`Field`]s.
macro_rules! format_fields {
    ($args:literal) => {
        format_args!($args, Field::Dockerfile, Field::DockerfileInline)
    };
}

impl Serialize for Dockerfile {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(Self::NAME, 1)?;

        let key = Field::from(self).as_str();
        match self {
            Self::File(path) => state.serialize_field(key, path)?,
            Self::Inline(string) => state.serialize_field(key, string)?,
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Dockerfile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        option::deserialize(deserializer)?
            .ok_or_else(|| de::Error::custom(format_fields!("missing required field `{}` or `{}`")))
    }
}

/// (De)serialize [`Option<Dockerfile>`], for use in `#[serde(with = "option")]`.
///
/// For deserialization, the following is returned:
///
/// - `Ok(Some(Dockerfile::File(_)))`, if given a struct/map with a `dockerfile` field.
/// - `Ok(Some(Dockerfile::Inline(_)))`, if given a struct/map with a `dockerfile_inline` field.
/// - `Ok(None)`, if neither the `dockerfile` or `dockerfile_inline` fields are present.
/// - `Err(_)`, if both fields are present.
/// - `Err(_)`, if there is an error deserializing either field value.
pub(super) mod option {
    use std::path::PathBuf;

    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    use super::{Dockerfile, Field};

    /// Serialize `Option<Dockerfile>`
    ///
    /// # Errors
    ///
    /// Returns an error if the `serializer` does while serializing.
    pub(in super::super) fn serialize<S: Serializer>(
        value: &Option<Dockerfile>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }

    /// Deserialize `Option<Dockerfile>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `deserializer` does, there is an error deserializing either
    /// [`Dockerfile`] variant, or both fields are present.
    pub(in super::super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Dockerfile>, D::Error> {
        let DockerfileFlat {
            dockerfile,
            dockerfile_inline,
        } = DockerfileFlat::deserialize(deserializer)?;

        match (dockerfile, dockerfile_inline) {
            (Some(dockerfile), None) => Ok(Some(Dockerfile::File(dockerfile))),
            (None, Some(dockerfile_inline)) => Ok(Some(Dockerfile::Inline(dockerfile_inline))),
            (None, None) => Ok(None),
            (Some(_), Some(_)) => Err(de::Error::custom(format_fields!(
                "cannot set both `{}` and `{}`"
            ))),
        }
    }

    /// Flattened version of [`Dockerfile`].
    #[derive(Deserialize)]
    #[serde(
        rename = "Dockerfile",
        expecting = "a struct with either a `dockerfile` or `dockerfile_inline` field"
    )]
    struct DockerfileFlat {
        /// [`Dockerfile::File`]
        #[serde(default)]
        dockerfile: Option<PathBuf>,

        /// [`Dockerfile::Inline`]
        #[serde(default)]
        dockerfile_inline: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file() {
        let dockerfile = Dockerfile::File("file".into());
        let string = "dockerfile: file\n";
        assert_eq!(dockerfile, serde_yaml::from_str(string).unwrap());
        assert_eq!(serde_yaml::to_string(&dockerfile).unwrap(), string);
    }

    #[test]
    fn inline() {
        let dockerfile = Dockerfile::Inline("inline".into());
        let string = "dockerfile_inline: inline\n";
        assert_eq!(dockerfile, serde_yaml::from_str(string).unwrap());
        assert_eq!(serde_yaml::to_string(&dockerfile).unwrap(), string);
    }

    #[test]
    fn missing_err() {
        assert!(serde_yaml::from_str::<Dockerfile>("{}")
            .unwrap_err()
            .to_string()
            .contains("missing"));
    }

    #[test]
    fn both_err() {
        assert!(serde_yaml::from_str::<Dockerfile>(
            "{ dockerfile: file, dockerfile_inline: inline }"
        )
        .unwrap_err()
        .to_string()
        .contains("both"));
    }

    #[derive(Deserialize, Debug)]
    struct Test {
        #[serde(flatten, with = "option")]
        dockerfile: Option<Dockerfile>,
    }

    #[test]
    fn flatten_option_none() {
        assert_eq!(serde_yaml::from_str::<Test>("{}").unwrap().dockerfile, None);
    }

    #[test]
    fn flatten_option_both_err() {
        assert!(
            serde_yaml::from_str::<Test>("{ dockerfile: file, dockerfile_inline: inline }")
                .unwrap_err()
                .to_string()
                .contains("both")
        );
    }
}
