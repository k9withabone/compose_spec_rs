use std::path::PathBuf;

use compose_spec_macros::{AsShort, FromShort};
use serde::{Deserialize, Serialize};

use crate::{serde::display_from_str_option, Extensions, Identifier};

/// Long syntax config or secret configuration.
///
/// # Config
///
/// Configs allow services to adapt their behavior without the need to rebuild a container image.
/// Services can only access configs when explicitly granted by the
/// [`configs`](super::Service#structfield.configs) attribute.
///
/// [service config compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#configs)
///
/// # Secret
///
/// Secrets grant access to sensitive data defined by
/// [`secrets`](crate::Compose#structfield.secrets) on a per-service basis.
///
/// [service secrets compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#configs)
///
/// [build secrets compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#secrets)
#[derive(Serialize, Deserialize, AsShort, FromShort, Debug, Clone, PartialEq, Eq)]
pub struct ConfigOrSecret {
    /// The name of the config/secret as it exists on the platform.
    #[as_short(short)]
    pub source: Identifier,

    /// Configs: The path and name of the file to be mounted in the service's task containers.
    /// Defaults to `/<source>` if not specified.
    ///
    /// Secrets: The name of the file to be mounted in `/run/secrets/` in the service's task
    /// containers. Defaults to `source` if not specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PathBuf>,

    /// The numeric UID that owns the mounted config/secret file within the service's task
    /// containers.
    ///
    /// Default value when not specified is the UID from the container image's USER.
    ///
    /// (De)serialized from/to a string.
    #[serde(
        default,
        with = "display_from_str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub uid: Option<u32>,

    /// The numeric GID that owns the mounted config/secret file within the service's task
    /// containers.
    ///
    /// Default value when not specified is the GID from the container image's USER.
    ///
    /// (De)serialized from/to a string.
    #[serde(
        default,
        with = "display_from_str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub gid: Option<u32>,

    /// The permissions for the file that is mounted within the service's task containers.
    ///
    /// Default value is world-readable permissions (mode `0o444`). The writable bit must be ignored
    /// if set. The executable bit may be set.
    ///
    /// Note that, when deserializing with [`serde_yaml`], octal numbers must start with `0o`, e.g.
    /// `0o555`, otherwise, they are interpreted as decimal numbers. Unfortunately, for
    /// serialization, there is no way to specify that a number should be serialized in octal form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}
