//! [`Options`] builder for deserialization options for a [`Compose`] file.

use std::io::Read;

use crate::{Compose, YamlValue};

/// Deserialization options builder for a [`Compose`] file.
#[allow(missing_copy_implementations)] // Will include interpolation vars as a HashMap.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// TODO
    merge_anchors: bool,
}

impl Options {
    /// Use the set options to deserialize a [`Compose`] file from a string slice of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_str(&self, yaml: &str) -> serde_yaml::Result<Compose> {
        serde_yaml::from_str(yaml)
    }

    /// Use the set options to deserialize a [`Compose`] file from an IO stream of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_reader<R: Read>(&self, reader: R) -> serde_yaml::Result<Compose> {
        serde_yaml::from_reader(reader)
    }

    /// Use the set options to deserialize a [`Compose`] file from bytes of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_slice(&self, slice: &[u8]) -> serde_yaml::Result<Compose> {
        serde_yaml::from_slice(slice)
    }

    /// Use the set options to deserialize a [`Compose`] file from a YAML [`Value`](YamlValue).
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_value(&self, value: YamlValue) -> serde_yaml::Result<Compose> {
        serde_yaml::from_value(value)
    }
}
