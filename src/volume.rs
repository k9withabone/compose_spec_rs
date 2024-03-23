use serde::{Deserialize, Serialize};

use crate::{Extensions, Resource};

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
    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
