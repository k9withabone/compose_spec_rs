//! Provides [`Secret`] for the top-level `secrets` field of a [`Compose`](super::Compose) file.

use serde::{Deserialize, Serialize};

use crate::{Extensions, Resource};

impl From<Secret> for Resource<Secret> {
    fn from(value: Secret) -> Self {
        Self::Compose(value)
    }
}

/// Sensitive data that a [`Service`](super::Service) may be granted access to.
///
/// A secret is similar to a [`Config`](super::Config), but for sensitive data.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/09-secrets.md)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Secret {
    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
