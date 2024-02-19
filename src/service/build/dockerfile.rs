//! Provides [`Dockerfile`] for (de)serializing from/to the
//! [`dockerfile`](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile) and
//! [`dockerfile_inline`](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile_inline)
//! fields of the long [`Build`](super::Build) syntax.

use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use serde::{
    de::{self, MapAccess},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

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
    /// Struct name for (de)serializing
    const NAME: &'static str = "Dockerfile";

    /// Possible fields
    const FIELDS: [&'static str; 2] =
        [Field::Dockerfile.as_str(), Field::DockerfileInline.as_str()];
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
        deserializer.deserialize_struct(Self::NAME, &Self::FIELDS, Visitor)
    }
}

/// (De)serialize `Option<Dockerfile>`, for use in `#[serde(with = "option")]`.
///
/// For deserialization, the following is returned:
/// - `Ok(Dockerfile::File(_))`, if given struct/map with `dockerfile` field.
/// - `Ok(Dockerfile::Inline(_))`, if given struct/map with `dockerfile_inline` field.
/// - `Ok(None)`, if neither the `dockerfile` or `dockerfile_inline` fields are present.
/// - `Err(_)`, if both fields are present, or a field is repeated multiple times, with custom error message.
/// - `Err(_)`, if there is an error deserializing either field value, or the `deserializer` returns an error.
pub(super) mod option {
    use serde::{Deserializer, Serialize, Serializer};

    use super::{Dockerfile, OptionVisitor};

    /// Serialize `Option<Dockerfile>`
    ///
    /// # Errors
    ///
    /// Returns an error if the `serializer` does while serializing.
    pub fn serialize<S>(value: &Option<Dockerfile>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.serialize(serializer)
    }

    /// Deserialize `Option<Dockerfile>`
    ///
    /// # Errors
    ///
    /// Returns an error if the `deserializer` does, if there is an error deserializing either
    /// [`Dockerfile`] variant, if both fields are present, or if either field is repeated.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Dockerfile>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct(Dockerfile::NAME, &Dockerfile::FIELDS, OptionVisitor)
    }
}

/// Possible [`Dockerfile`] fields.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Dockerfile,
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

/// [`de::Visitor`] for deserializing [`Dockerfile`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Dockerfile;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        OptionVisitor.expecting(formatter)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        OptionVisitor
            .visit_map(map)?
            .ok_or_else(|| de::Error::custom(format_fields!("missing field `{}` or `{}`")))
    }
}

/// [`de::Visitor`] for deserializing [`Option<Dockerfile>`].
struct OptionVisitor;

impl<'de> de::Visitor<'de> for OptionVisitor {
    type Value = Option<Dockerfile>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_fmt(format_fields!("`{}` or `{}`"))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut field = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::Dockerfile => {
                    check_multiple(&field)?;
                    field = Some(Dockerfile::File(map.next_value()?));
                }
                Field::DockerfileInline => {
                    check_multiple(&field)?;
                    field = Some(Dockerfile::Inline(map.next_value()?));
                }
            }
        }

        Ok(field)
    }
}

/// Check if `field` is occupied and return [`Err`] if so.
fn check_multiple<T, E: de::Error>(field: &Option<T>) -> Result<(), E> {
    if field.is_some() {
        Err(E::custom(format_fields!(
            "only one of `{}` or `{}` can be specified"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize(source: &str) -> serde_yaml::Result<Dockerfile> {
        serde_yaml::from_str(source)
    }

    #[test]
    fn file() {
        assert_eq!(
            deserialize(r#"{"dockerfile": "file"}"#).unwrap(),
            Dockerfile::File("file".into()),
        );
    }

    #[test]
    fn inline() {
        assert_eq!(
            deserialize(r#"{"dockerfile_inline": "inline"}"#).unwrap(),
            Dockerfile::Inline("inline".into()),
        );
    }

    #[test]
    fn empty_err() {
        assert!(deserialize("{}").is_err());
    }

    #[test]
    fn multiple_err() {
        assert!(deserialize(r#"{"dockerfile": "file", "dockerfile_inline": "inline"}"#).is_err());
    }

    #[derive(Deserialize)]
    struct Test {
        #[serde(flatten, with = "option")]
        test: Option<Dockerfile>,
    }

    #[test]
    fn flatten_option_empty() {
        assert!(serde_yaml::from_str::<Test>("{}").unwrap().test.is_none());
    }

    #[test]
    fn flatten_option_multiple_err() {
        assert!(serde_yaml::from_str::<Test>(
            r#"{"dockerfile": "file", "dockerfile_inline": "inline"}"#
        )
        .is_err());
    }
}
