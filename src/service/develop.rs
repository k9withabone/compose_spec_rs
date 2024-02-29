//! Provides [`Develop`] for the `develop` field of [`Service`](super::Service).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Extensions;

/// Development constraints and workflows for maintaining a container in sync with source.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Develop {
    /// List of rules that control automatic service updates based on local file changes.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#watch)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch: Vec<WatchRule>,
}

/// Rule which controls automatic service updates based on local file changes.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#watch)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WatchRule {
    /// Action to take when changes are detected.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#action)
    pub action: Action,

    /// Patterns for paths to be ignored. Any updated file that matches a pattern, or belongs to a
    /// folder that matches a pattern, won't trigger services to be re-created.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#ignore)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<PathBuf>,

    /// Path to source code (relative to the project directory) to monitor for changes.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#path)
    pub path: PathBuf,

    /// Files within `path` with changes are synchronized to the container filesystem at this
    /// location, so that the latter is always running with up-to-date content.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#target)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PathBuf>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Action to take when changes are detected.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/develop.md#action)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// Rebuild the [`Service`](super::Service) image based on the `build` section and recreate the
    /// service with the updated image.
    Rebuild,

    /// Keep the existing service container(s) running, but synchronize source files with container
    /// content according to the `target` field.
    Sync,

    /// Synchronize source files with container content according to the `target` field, and then
    /// restart the container.
    #[serde(rename = "sync+restart")]
    SyncAndRestart,
}
