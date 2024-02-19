//! Provides [`Include`], for the long syntax of [`Compose`](super::Compose) files' top-level
//! `include` field.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{common::ItemOrList, AsShort};

/// A Compose sub-project to include.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Include {
    /// The location of the Compose file(s) to be parsed and included into the local Compose model.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md#path)
    pub path: ItemOrList<PathBuf>,

    /// Base path to resolve relative paths set in the Compose file.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md#project_directory)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_directory: Option<String>,

    /// Environment file(s) to use to define default values when interpolating variables in the
    /// Compose file being parsed.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/14-include.md#env_file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_file: Option<ItemOrList<PathBuf>>,
}

impl AsShort for Include {
    type Short = PathBuf;

    fn as_short(&self) -> Option<&Self::Short> {
        let Self {
            path,
            project_directory,
            env_file,
        } = self;

        if project_directory.is_none() && env_file.is_none() {
            path.as_item()
        } else {
            None
        }
    }
}

impl From<PathBuf> for Include {
    fn from(value: PathBuf) -> Self {
        Self {
            path: ItemOrList::Item(value),
            project_directory: None,
            env_file: None,
        }
    }
}
