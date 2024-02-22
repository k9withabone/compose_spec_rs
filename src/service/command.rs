use std::fmt::{self, Formatter};

use serde::{
    de::{self, value::SeqAccessDeserializer, SeqAccess},
    Deserialize, Deserializer, Serialize,
};

/// Override the default command declared by the container image.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#command)
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Command {
    /// Command run with `/bin/sh -c`.
    String(String),

    /// Arguments to the entrypoint.
    List(Vec<String>),
}

impl From<String> for Command {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<Vec<String>> for Command {
    fn from(value: Vec<String>) -> Self {
        Self::List(value)
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

/// [`de::Visitor`] for deserializing [`Command`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Command;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a string or list of strings")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Command::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Command::String(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Vec::deserialize(SeqAccessDeserializer::new(seq)).map(Command::List)
    }
}
