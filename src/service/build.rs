//! Provides long [`Build`] syntax for the `build` field of [`Service`](super::Service).

mod cache;
mod context;
mod dockerfile;
mod network;
mod ssh_auth;

use std::{net::IpAddr, ops::Not};

use compose_spec_macros::{AsShort, FromShort};
use indexmap::{IndexMap, IndexSet};
use serde::{de, Deserialize, Deserializer, Serialize};

use crate::{Extensions, Identifier, ListOrMap, MapKey, ShortOrLong};

pub use self::{
    cache::{Cache, CacheOption, CacheType, InvalidCacheOptionError, ParseCacheError},
    context::Context,
    dockerfile::Dockerfile,
    network::Network,
    ssh_auth::{Id as SshAuthId, IdError as SshAuthIdError, SshAuth},
};

use super::{extra_hosts, ByteValue, ConfigOrSecret, Hostname, Image, Platform, Ulimits};

/// Long syntax build configuration for creating a container image from source.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md)
#[derive(Serialize, Deserialize, AsShort, FromShort, Default, Debug, Clone, PartialEq)]
pub struct Build {
    /// Path to a directory containing a Dockerfile/Containerfile, or a URL to a git repository.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#context)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[as_short(short)]
    pub context: Option<Context>,

    /// Set an alternate Dockerfile/Containerfile or define its content inline.
    /// A relative path is resolved from the build context.
    ///
    /// Represents either the `dockerfile` or `dockerfile_inline` fields,
    /// which conflict with each other.
    ///
    /// This is (de)serialized by flattening [`Dockerfile`]. When deserializing, if neither the
    /// `dockerfile` or `dockerfile_inline` fields are present, this is [`None`].
    /// If both fields are present, or either is repeated, then an error is returned.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#dockerfile)
    #[serde(flatten, with = "dockerfile::option")]
    pub dockerfile: Option<Dockerfile>,

    /// Build arguments, i.e. Dockerfile/Containerfile `ARG` values.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#args)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub args: ListOrMap,

    /// SSH authentications that the image builder should use during image build.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#ssh)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub ssh: IndexSet<SshAuth>,

    /// Sources the image builder should use for cache resolution.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_from)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_from: Vec<Cache>,

    /// Export locations to be used to share build cache with future builds.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#cache_to)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_to: Vec<Cache>,

    /// Named contexts the image builder should use during image build.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#additional_contexts)
    #[serde(
        default,
        skip_serializing_if = "IndexMap::is_empty",
        deserialize_with = "additional_contexts"
    )]
    pub additional_contexts: IndexMap<MapKey, Context>,

    /// Add hostname mappings at build-time.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#extra_hosts)
    #[serde(
        default,
        skip_serializing_if = "IndexMap::is_empty",
        deserialize_with = "extra_hosts"
    )]
    pub extra_hosts: IndexMap<Hostname, IpAddr>,

    /// Specifies a build’s container isolation technology.
    ///
    /// Like [`isolation`](super::Service#structfield.isolation),
    /// supported values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#isolation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,

    /// Configure the service image to build with elevated privileges.
    ///
    /// Support and actual impacts are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#privileged)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub privileged: bool,

    /// Add metadata to the resulting image.
    ///
    /// It's recommended that you use reverse-DNS notation to prevent your labels from conflicting
    /// with other software.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#args)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Disable image builder cache and enforce a full rebuild from source for all image layers.
    ///
    /// Only applies to layers declared in the Dockerfile/Containerfile, referenced images could be
    /// retrieved from a local image store whenever the tag has been updated on registry.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#no_cache)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub no_cache: bool,

    /// Require the image builder to pull referenced images, even those already available in the
    /// local image store.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#pull)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub pull: bool,

    /// Set the network containers connect to during build for `RUN` instructions.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#network)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,

    /// Set the size of the shared memory allocated for building container images.
    ///
    /// Corresponds to the `/dev/shm` partition on Linux.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#shm_size)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shm_size: Option<ByteValue>,

    /// Set the stage to build as defined inside a multi-stage Dockerfile/Containerfile.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#target)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Grant access to sensitive data defined by [`secrets`](crate::Compose#structfield.secrets) on
    /// a per-service build basis.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#secrets)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<ShortOrLong<Identifier, ConfigOrSecret>>,

    /// List of tag mappings that must be associated to the build image.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#tags)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Image>,

    /// Override the default ulimits for a container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#ulimits)
    #[serde(default, skip_serializing_if = "Ulimits::is_empty")]
    pub ulimits: Ulimits,

    /// List of target platforms.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#platforms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<Platform>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Deserialize `additional_contexts` field of [`Build`].
///
/// Converts from [`ListOrMap`].
fn additional_contexts<'de, D>(deserializer: D) -> Result<IndexMap<MapKey, Context>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(ListOrMap::deserialize(deserializer)?
        .into_map()
        .map_err(de::Error::custom)?
        .into_iter()
        .map(|(key, value)| {
            let value = value
                .map(String::from)
                .map_or_else(Context::default, Context::from);
            (key, value)
        })
        .collect())
}
