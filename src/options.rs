//! [`Options`] builder for deserialization options for a [`Compose`] file.

use std::{collections::HashMap, io::Read};

use crate::{
    variable_interpolation::{yaml_walk::interpolate_value, VariableResolver},
    Compose, YamlValue,
};

/// Deserialization options builder for a [`Compose`] file.
#[allow(missing_copy_implementations)] // Will include interpolation vars as a HashMap.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// Whether to perform merging of `<<` keys.
    apply_merge: bool,
    /// the variables to perform environment variable interpolation on, if any
    interpolate_vars: Option<VariableResolver>,
}

impl Options {
    /// Set whether to merge `<<` keys into the surrounding mapping.
    ///
    /// ```
    /// use compose_spec::Compose;
    ///
    /// let yaml = "\
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
    pub const fn apply_merge(&mut self, apply_merge: bool) -> &mut Self {
        self.apply_merge = apply_merge;
        self
    }

    /// Add a map of values to interpolate variable placeholders in YAML before constructing the `Compose`
    /// instance.
    /// The given `HashMap` could be sourced from dotenv files or the processes' environment, or both.
    /// Can be called multiple times to merge different variable sources.
    /// The last mapping for any given variable name wins.
    ///
    /// ```
    /// use compose_spec::Compose;
    /// use std::collections::HashMap;
    ///
    /// let yaml = "\
    /// services:
    ///   one:
    ///     environment:
    ///       FOO: $FOO               # simple variable without default
    ///       BAR: ${BAR-Bar-Default} # braced variable with default value
    ///       BAZ: $$ESCAPED          # double-$ to write a literal $
    /// ";
    ///
    /// let vars = HashMap::from([
    ///    ("FOO".into(), "Foo-Value".into())
    /// ]);
    ///
    /// let compose = Compose::options()
    ///     .interpolate_vars(vars)
    ///     .from_yaml_str(yaml)
    ///     .unwrap();
    ///
    /// let one_env = compose.services["one"]
    ///     .environment
    ///     .clone()
    ///     .into_map()
    ///     .unwrap();
    ///
    /// assert_eq!(one_env["FOO"].as_ref().unwrap().as_string().unwrap(), "Foo-Value");
    /// assert_eq!(one_env["BAR"].as_ref().unwrap().as_string().unwrap(), "Bar-Default");
    /// assert_eq!(one_env["BAZ"].as_ref().unwrap().as_string().unwrap(), "$ESCAPED");
    /// ```
    pub fn interpolate_vars(&mut self, vars: HashMap<String, String>) -> &mut Self {
        let mut resolver = self.interpolate_vars.take().unwrap_or_default();
        resolver.add_vars(vars);
        let _ = self.interpolate_vars.replace(resolver);
        self
    }

    /// Return `true` if any options are set.
    const fn any(&self) -> bool {
        let Self {
            apply_merge,
            interpolate_vars,
        } = self;
        *apply_merge || interpolate_vars.is_some()
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

        if let Some(vars) = &self.interpolate_vars {
            interpolate_value(vars, &mut value)?;
        }

        serde_yaml::from_value(value)
    }
}
