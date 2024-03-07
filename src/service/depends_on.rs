//! Provides [`DependsOn`] for the `depends_on` field of [`Service`](super::Service).

use std::{iter, ops::Not};

use indexmap::{
    map::{Iter, IterMut, Keys},
    IndexMap, IndexSet,
};
use serde::{Deserialize, Serialize};

use crate::{
    serde::{default_true, skip_true},
    Identifier,
};

/// Long [`depends_on`](super::Service#structfield.depends_on) syntax which expresses startup and
/// shutdown dependencies between services.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-1)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
pub struct DependsOn(pub IndexMap<Identifier, Config>);

impl DependsOn {
    /// Returns an iterator of [`Identifier`]s if the long syntax
    /// ([`IndexMap<Identifier, Options>`]) can be represented as the short syntax
    /// ([`IndexSet<Identifier>`]).
    #[must_use]
    pub fn as_short_iter(&self) -> Option<Keys<Identifier, Config>> {
        let default_options = Config::default();
        if self.0.values().all(|options| *options == default_options) {
            Some(self.0.keys())
        } else {
            None
        }
    }

    /// Return an iterator of [`Identifier`] and [`Config`] pairs.
    #[must_use]
    pub fn iter(&self) -> Iter<Identifier, Config> {
        self.0.iter()
    }

    /// Return an iterator of [`Identifier`] and [`Config`] pairs.
    #[must_use]
    pub fn iter_mut(&mut self) -> IterMut<Identifier, Config> {
        self.0.iter_mut()
    }
}

impl From<IndexSet<Identifier>> for DependsOn {
    fn from(value: IndexSet<Identifier>) -> Self {
        value.into_iter().collect()
    }
}

impl FromIterator<Identifier> for DependsOn {
    fn from_iter<T: IntoIterator<Item = Identifier>>(iter: T) -> Self {
        iter.into_iter()
            .zip(iter::repeat(Config::default()))
            .collect()
    }
}

impl FromIterator<(Identifier, Config)> for DependsOn {
    fn from_iter<T: IntoIterator<Item = (Identifier, Config)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for DependsOn {
    type Item = (Identifier, Config);

    type IntoIter = <IndexMap<Identifier, Config> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a DependsOn {
    type Item = (&'a Identifier, &'a Config);

    type IntoIter = Iter<'a, Identifier, Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut DependsOn {
    type Item = (&'a Identifier, &'a mut Config);

    type IntoIter = IterMut<'a, Identifier, Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Configuration of [`Service`](super::Service) dependencies.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-1)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Condition under which the dependency is considered satisfied.
    pub condition: Condition,

    /// When `true`, Compose restarts this service after it updates the dependency service.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub restart: bool,

    /// When `false`, Compose only warns you when the dependency service isn't started or available.
    ///
    /// Default is `true`.
    #[serde(default = "default_true", skip_serializing_if = "skip_true")]
    pub required: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            condition: Condition::default(),
            restart: false,
            required: true,
        }
    }
}

/// Condition under which the [dependency](DependsOn) is considered satisfied.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-1)
#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// Dependency has started.
    #[default]
    ServiceStarted,

    /// Dependency is "healthy", as defined by its [`Healthcheck`](super::Healthcheck).
    ServiceHealthy,

    /// Dependency ran to completion and exited successfully.
    ServiceCompletedSuccessfully,
}
