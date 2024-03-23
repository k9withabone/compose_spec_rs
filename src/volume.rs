use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Extensions, MapKey, Resource, StringOrNumber};

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

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
