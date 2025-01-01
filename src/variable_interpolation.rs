//! Implementation of the environment variable interpolation as defined in the
//! [compose spec](https://github.com/compose-spec/compose-spec/blob/main/12-interpolation.md)
//!
//! `YamlValues` are interpolated in-place.

use std::collections::HashMap;

mod error;
mod parser;
pub(crate) mod yaml_walk;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// Resolves values for a given key from the relevant sources (.env files, process environment, ...)
pub(crate) struct VariableResolver {
    /// map of all resolvable variable names
    vars: HashMap<String, String>,
}

impl VariableResolver {
    /// add some name <-> value mappings to the interpolation. last definition of a name wins.
    pub(crate) fn add_vars(&mut self, new_vars: HashMap<String, String>) -> &mut Self {
        for (key, value) in new_vars {
            let _ = self.vars.insert(key, value);
        }
        self
    }

    /// get the value associated with the given key.
    pub(crate) fn get(&self, key: impl AsRef<str>) -> Option<&str> {
        self.vars.get(key.as_ref()).map(String::as_str)
    }
}
