//! Provides [`Config`] for the top-level `configs` field of a [`Compose`](super::Compose) file.

use serde::{Deserialize, Serialize};

use crate::{Extensions, Resource};

impl From<Config> for Resource<Config> {
    fn from(value: Config) -> Self {
        Self::Compose(value)
    }
}

/// Configuration which allow a [`Service`] to adapt its behaviour without needing to rebuild the
/// container image.
///
/// Like [`Volume`]s, configs are mounted as files into the [`Service`]'s container's file system.
/// The location of the mount point within the container defaults to `/<config-name>` in Linux
/// containers, and `C:\<config-name>` in Windows containers.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/08-configs.md)
///
/// [`Service`]: super::Service
/// [`Volume`]: super::Volume
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
