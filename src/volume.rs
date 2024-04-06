//! Provides [`Volume`] for the top-level `volumes` field of a [`Compose`](super::Compose) file.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Extensions, ListOrMap, MapKey, Resource, StringOrNumber};

impl Resource<Volume> {
    /// Custom volume name, if set.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md#name)
    #[must_use]
    pub const fn name(&self) -> Option<&String> {
        match self {
            Self::External { name } => name.as_ref(),
            Self::Compose(volume) => volume.name.as_ref(),
        }
    }
}

impl From<Volume> for Resource<Volume> {
    fn from(value: Volume) -> Self {
        Self::Compose(value)
    }
}

/// A named volume which can be reused across multiple [`Service`](super::Service)s.
///
/// Volumes are persistent data stores implemented by the container engine.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Volume {
    /// Which driver to use for this volume.
    ///
    /// Default and available values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md#driver)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// Driver-dependent options.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md#driver_opts)
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub driver_opts: IndexMap<MapKey, StringOrNumber>,

    /// Add metadata to the volume.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md#labels)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Custom name for the volume.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/07-volumes.md#name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Volume {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            driver,
            driver_opts,
            labels,
            name,
            extensions,
        } = self;

        driver.is_none()
            && driver_opts.is_empty()
            && labels.is_empty()
            && name.is_none()
            && extensions.is_empty()
    }
}
