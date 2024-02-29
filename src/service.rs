//! Provides [`Service`] for the [`Compose`](super::Compose) top-level `services` field.

pub mod blkio_config;
pub mod build;
mod byte_value;
mod cgroup;
mod command;
mod config_or_secret;
mod container_name;
mod cpuset;
mod credential_spec;
pub mod depends_on;
pub mod deploy;
pub mod develop;
pub mod device;
pub mod image;
pub mod platform;
mod ulimit;

use std::{net::IpAddr, time::Duration};

use indexmap::{IndexMap, IndexSet};
use serde::{de, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{
    serde::{default_true, duration_option, duration_us_option, skip_true},
    Extensions, Identifier, ListOrMap, MapKey, ShortOrLong, Value,
};

use self::build::Context;
pub use self::{
    blkio_config::BlkioConfig,
    build::Build,
    byte_value::{ByteValue, ParseByteValueError},
    cgroup::{Cgroup, ParseCgroupError},
    command::Command,
    config_or_secret::ConfigOrSecret,
    container_name::{ContainerName, InvalidContainerNameError},
    cpuset::{CpuSet, ParseCpuSetError},
    credential_spec::{CredentialSpec, Kind as CredentialSpecKind},
    depends_on::DependsOn,
    deploy::Deploy,
    develop::Develop,
    device::Device,
    image::Image,
    platform::Platform,
    ulimit::{InvalidResourceError, Resource, Ulimit, Ulimits},
};

/// A service is an abstract definition of a computing resource within an application which can be
/// scaled or replaced independently from other components.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Service {
    /// When defined and set to `false` Compose does not collect service logs, until you explicitly
    /// request it to.
    ///
    /// The default service configuration is `attach: true`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#attach)
    #[serde(default = "default_true", skip_serializing_if = "skip_true")]
    pub attach: bool,

    /// Build configuration for creating a container image from source.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#build)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<ShortOrLong<Context, Build>>,

    /// Configuration options to set block IO limits for a service.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#blkio_config)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blkio_config: Option<BlkioConfig>,

    /// Number of usable CPUs for the service container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_count)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<u64>,

    /// Usable percentage of the available CPUs.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_percent)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<Percent>,

    /// Service container's relative CPU weight versus other containers.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_shares)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_shares: Option<u64>,

    /// Configure CPU CFS (Completely Fair Scheduler) period when a platform is based on Linux kernel.
    ///
    /// (De)serialized from/to microseconds.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_period)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_us_option"
    )]
    pub cpu_period: Option<Duration>,

    /// Configure CPU CFS (Completely Fair Scheduler) quota when a platform is based on Linux kernel.
    ///
    /// (De)serialized from/to microseconds.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_quota)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_us_option"
    )]
    pub cpu_quota: Option<Duration>,

    /// Configure CPU allocation parameters for platforms with support for realtime scheduler.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_rt_runtime)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub cpu_rt_runtime: Option<Duration>,

    /// Configure CPU allocation parameters for platforms with support for realtime scheduler.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpu_rt_period)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub cpu_rt_period: Option<Duration>,

    /// CPUs in which to allow execution.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpuset)
    #[serde(default, skip_serializing_if = "CpuSet::is_empty")]
    pub cpuset: CpuSet,

    /// Add additional container [**capabilities**(7)](https://man7.org/linux/man-pages/man7/capabilities.7.html).
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cap_add)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub cap_add: IndexSet<String>,

    /// Drop container [**capabilities**(7)](https://man7.org/linux/man-pages/man7/capabilities.7.html).
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cap_drop)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub cap_drop: IndexSet<String>,

    /// [Cgroup](https://man7.org/linux/man-pages/man7/cgroups.7.html) namespace to join.
    ///
    /// When unset, it is the container runtime's decision to select which cgroup namespace to use,
    /// if supported.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cgroup)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cgroup: Option<Cgroup>,

    /// Optional parent [cgroup](https://man7.org/linux/man-pages/man7/cgroups.7.html) for the
    /// container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cgroup_parent)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cgroup_parent: Option<String>,

    /// Overrides the default command declared by the container image.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#command)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,

    /// Configs allow services to adapt their behavior without the need to rebuild a container image.
    ///
    /// Services can only access configs when explicitly granted by the `configs` field.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#configs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<ShortOrLong<Identifier, ConfigOrSecret>>,

    /// Custom container name.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#container_name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_name: Option<ContainerName>,

    /// Credential spec for a managed service account.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#credential_spec)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_spec: Option<CredentialSpec>,

    /// Startup and shutdown dependencies between services.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#depends_on)
    #[serde(default, skip_serializing_if = "depends_on_is_empty")]
    pub depends_on: ShortOrLong<IndexSet<Identifier>, DependsOn>,

    /// Configuration for the deployment and lifecycle of services.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#deploy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<Deploy>,

    /// Development configuration for maintaining a container in sync with source.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#develop)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub develop: Option<Develop>,

    /// List of device cgroup rules for this container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_cgroup_rules)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub device_cgroup_rules: IndexSet<device::CgroupRule>,

    /// List of device mappings for the created container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub devices: IndexSet<Device>,

    /// Specifies a build's container isolation technology.
    ///
    /// Supported values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#isolation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// A percentage, must be between 0 and 100, inclusive.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(into = "u8", try_from = "u8")]
pub struct Percent(u8);

impl Percent {
    /// Create a new [`Percent`].
    ///
    /// # Errors
    ///
    /// Returns an error if the percent is not between 0 and 100, inclusive.
    pub fn new(percent: u8) -> Result<Self, PercentRangeError> {
        match percent {
            0..=100 => Ok(Self(percent)),
            percent => Err(PercentRangeError(percent)),
        }
    }

    /// Return the inner value.
    #[must_use]
    pub fn into_inner(self) -> u8 {
        self.0
    }
}

/// Error returned when attempting to create a [`Percent`] and the value is not between 0 and 100,
/// inclusive.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("percent `{0}` is not between 0 and 100")]
pub struct PercentRangeError(u8);

impl TryFrom<u8> for Percent {
    type Error = PercentRangeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Percent> for u8 {
    fn from(value: Percent) -> Self {
        value.into_inner()
    }
}

impl PartialEq<u8> for Percent {
    fn eq(&self, other: &u8) -> bool {
        self.0.eq(other)
    }
}

/// Returns `true` if `depends_on` is in short syntax form and the [`IndexSet`] is empty.
fn depends_on_is_empty(depends_on: &ShortOrLong<IndexSet<Identifier>, DependsOn>) -> bool {
    if let ShortOrLong::Short(set) = depends_on {
        set.is_empty()
    } else {
        false
    }
}

/// Deserialize `extra_hosts` field of [`Service`] and long [`Build`] syntax.
///
/// Converts from [`ListOrMap`].
fn extra_hosts<'de, D>(deserializer: D) -> Result<IndexMap<MapKey, IpAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    // `extra_hosts` can be a list of strings with the format "{host}={ip}" or "{host}:{ip}"
    // or a map of strings. Additionally, IPv6 addresses may be enclosed in square brackets.
    ListOrMap::deserialize(deserializer)?
        .into_map_split_on(&['=', ':'])
        .map_err(de::Error::custom)?
        .into_iter()
        .map(|(key, value)| {
            let value = value.as_ref().and_then(Value::as_string).ok_or_else(|| {
                de::Error::custom("extra host value must be a string representing an IP address")
            })?;

            // Remove brackets possibly surrounding IP address, e.g. `[::1]`
            let value = value.strip_prefix('[').unwrap_or(value);
            let value = value.strip_suffix(']').unwrap_or(value);

            Ok((key, value.parse().map_err(de::Error::custom)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::{arbitrary::Arbitrary, path::PathParams, strategy::Strategy};

    /// [`Strategy`] for generating [`PathBuf`]s that do not contain colons.
    pub(super) fn path_no_colon() -> impl Strategy<Value = PathBuf> {
        PathBuf::arbitrary_with(PathParams::default().with_component_regex("[^:]*"))
    }
}
