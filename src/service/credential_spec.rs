//! Provides [`CredentialSpec`] for the `credential_spec` field of [`Service`](super::Service).

use std::{
    fmt::{self, Formatter},
    path::PathBuf,
};

use serde::{
    de::{self, MapAccess},
    Deserialize, Deserializer, Serialize,
};

use crate::{ExtensionKey, Extensions, Identifier};

/// Credential spec for a managed service account.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#credential_spec)
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CredentialSpec {
    /// One of [`config`](Kind::Config), [`file`](Kind::File), or [`registry`](Kind::Registry).
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub kind: Kind,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl<'de> Deserialize<'de> for CredentialSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(Visitor)
    }
}

/// Kind of [`CredentialSpec`].
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// [`CredentialSpec`] set in top-level `configs` field of [`Compose`](crate::Compose).
    Config(Identifier),

    /// Read from file.
    File(PathBuf),

    /// Read from the Windows registry on the daemon's host.
    Registry(String),
}

/// [`de::Visitor`] for deserializing [`CredentialSpec`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = CredentialSpec;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("struct CredentialSpec")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut kind = None;
        let mut extensions = Extensions::new();

        while let Some(key) = map.next_key()? {
            match key {
                Field::Config => set_kind(&mut kind, || map.next_value().map(Kind::Config))?,
                Field::File => set_kind(&mut kind, || map.next_value().map(Kind::File))?,
                Field::Registry => set_kind(&mut kind, || map.next_value().map(Kind::Registry))?,
                Field::Extension(key) => {
                    if extensions.insert(key, map.next_value()?).is_some() {
                        return Err(de::Error::custom("duplicate extension key"));
                    }
                }
            }
        }

        let kind = kind.ok_or_else(|| de::Error::missing_field("config, file, or registry"))?;
        Ok(CredentialSpec { kind, extensions })
    }
}

/// Set the `kind` field with the result of `f` if it is [`None`], otherwise error.
fn set_kind<E, F>(kind: &mut Option<Kind>, f: F) -> Result<(), E>
where
    E: de::Error,
    F: FnOnce() -> Result<Kind, E>,
{
    if kind.is_none() {
        *kind = Some(f()?);
        Ok(())
    } else {
        Err(E::custom(
            "only one of `config`, `file`, or `registry` may be set",
        ))
    }
}

/// Fields of [`CredentialSpec`].
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    /// [`config`](Kind::Config)
    Config,

    /// [`file`](Kind::File)
    File,

    /// [`registry`](Kind::Registry)
    Registry,

    /// Extension key.
    Extension(ExtensionKey),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let test = CredentialSpec {
            kind: Kind::File("test.json".into()),
            extensions: Extensions::from([(ExtensionKey::new("x-test").unwrap(), "test".into())]),
        };

        let string = serde_yaml::to_string(&test).unwrap();
        assert_eq!(string, "file: test.json\nx-test: test\n");

        let test2 = serde_yaml::from_str(&string).unwrap();
        assert_eq!(test, test2);
    }
}
