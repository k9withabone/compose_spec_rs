//! [`Options`] builder for deserialization options for a [`Compose`] file.

use std::io::Read;

use crate::{Compose, YamlValue};

/// Deserialization options builder for a [`Compose`] file.
#[allow(missing_copy_implementations)] // Will include interpolation vars as a HashMap.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// Whether to perform merging of `<<` keys.
    apply_merge: bool,
}

impl Options {
    /// Set whether to merge `<<` keys into the surrounding mapping.
    ///
    /// ```
    /// use compose_spec::Compose;
    ///
    /// let yaml = "
    /// services:
    ///   one:
    ///     environment: &env
    ///       FOO: foo
    ///       BAR: bar
    ///   two:
    ///     environment:
    ///       <<: *env
    ///       BAR: baz
    /// ";
    ///
    /// let compose = Compose::options()
    ///     .apply_merge(true)
    ///     .from_yaml_str(yaml)
    ///     .unwrap();
    ///
    /// let two_env = compose.services["two"]
    ///     .environment
    ///     .clone()
    ///     .into_map()
    ///     .unwrap();
    ///
    /// assert_eq!(two_env["FOO"].as_ref().unwrap().as_string().unwrap(), "foo");
    /// assert_eq!(two_env["BAR"].as_ref().unwrap().as_string().unwrap(), "baz");
    /// ```
    pub fn apply_merge(&mut self, apply_merge: bool) -> &mut Self {
        self.apply_merge = apply_merge;
        self
    }

    /// Return `true` if any options are set.
    const fn any(&self) -> bool {
        let Self { apply_merge } = *self;
        apply_merge
    }

    /// Use the set options to deserialize a [`Compose`] file from a string slice of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_str(&self, yaml: &str) -> serde_yaml::Result<Compose> {
        if self.any() {
            self.from_yaml_value(serde_yaml::from_str(yaml)?)
        } else {
            serde_yaml::from_str(yaml)
        }
    }

    /// Use the set options to deserialize a [`Compose`] file from an IO stream of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_reader<R: Read>(&self, reader: R) -> serde_yaml::Result<Compose> {
        if self.any() {
            self.from_yaml_value(serde_yaml::from_reader(reader)?)
        } else {
            serde_yaml::from_reader(reader)
        }
    }

    /// Use the set options to deserialize a [`Compose`] file from bytes of YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_slice(&self, slice: &[u8]) -> serde_yaml::Result<Compose> {
        if self.any() {
            self.from_yaml_value(serde_yaml::from_slice(slice)?)
        } else {
            serde_yaml::from_slice(slice)
        }
    }

    /// Use the set options to deserialize a [`Compose`] file from a YAML [`Value`](YamlValue).
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml_value(&self, mut value: YamlValue) -> serde_yaml::Result<Compose> {
        if self.apply_merge {
            value.apply_merge()?;
        }
        serde_yaml::from_value(value)
    }
}
