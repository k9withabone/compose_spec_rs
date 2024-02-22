//! Types for (de)serializing from/to the
//! [compose-spec](https://github.com/compose-spec/compose-spec). The types are validated while they
//! are deserialized when possible.
//!
//! Note that the [`Deserialize`] implementations of many types make use of
//! [`Deserializer::deserialize_any()`](::serde::de::Deserializer::deserialize_any). This means that
//! you should only attempt to deserialize them from self-describing formats like YAML or JSON.
//!
//! Lists that must contain unique values use [`IndexSet`](indexmap::IndexSet) otherwise they are
//! [`Vec`]s.

mod common;
pub mod duration;
mod include;
mod name;
mod serde;
pub mod service;

use std::path::PathBuf;

use ::serde::{Deserialize, Serialize};
use indexmap::IndexMap;

pub use self::{
    common::{
        AsShort, ExtensionKey, Extensions, Identifier, InvalidExtensionKeyError,
        InvalidIdentifierError, InvalidMapKeyError, ItemOrList, ListOrMap, MapKey, ShortOrLong,
        Value, YamlValue,
    },
    include::Include,
    name::{InvalidNameError, Name},
    service::Service,
};

/// The Compose file is a YAML file defining a multi-containers based application.
///
/// Note that the [`Deserialize`] implementations of many types within `Compose` make use of
/// [`Deserializer::deserialize_any()`](::serde::de::Deserializer::deserialize_any). This means that
/// you should only attempt to deserialize from self-describing formats like YAML or JSON.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/03-compose-file.md)
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Compose {
    /// Declared for backward compatibility, ignored.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/04-version-and-name.md#version-top-level-element)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Define the Compose project name, until user defines one explicitly.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/04-version-and-name.md#name-top-level-element)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,

    /// Compose sub-projects to be included.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<ShortOrLong<PathBuf, Include>>,

    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md)
    pub services: IndexMap<Identifier, Service>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
