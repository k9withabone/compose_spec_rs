//! Provides [`Service`] for the [`Compose`](super::Compose) top-level `services` field.

pub mod blkio_config;
pub mod build;
mod byte_value;
mod cgroup;
mod config_or_secret;
mod container_name;
mod cpuset;
mod credential_spec;
pub mod deploy;
pub mod develop;
pub mod device;
pub mod env_file;
mod expose;
pub mod healthcheck;
mod hostname;
pub mod image;
pub mod platform;
pub mod ports;
mod ulimit;
pub mod user_or_group;

use std::{
    fmt::{self, Display, Formatter},
    net::IpAddr,
    ops::Not,
    path::PathBuf,
    time::Duration,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::{map::Keys, IndexMap, IndexSet};
use serde::{de, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{
    impl_from_str,
    serde::{default_true, duration_option, duration_us_option, skip_true, ItemOrListVisitor},
    AsShortIter, Extensions, Identifier, InvalidIdentifierError, ItemOrList, ListOrMap, MapKey,
    ShortOrLong, Value,
};

use self::build::Context;
pub use self::{
    blkio_config::BlkioConfig,
    build::Build,
    byte_value::{ByteValue, ParseByteValueError},
    cgroup::{Cgroup, ParseCgroupError},
    config_or_secret::ConfigOrSecret,
    container_name::{ContainerName, InvalidContainerNameError},
    cpuset::{CpuSet, ParseCpuSetError},
    credential_spec::{CredentialSpec, Kind as CredentialSpecKind},
    deploy::Deploy,
    develop::Develop,
    device::Device,
    env_file::EnvFile,
    expose::Expose,
    healthcheck::Healthcheck,
    hostname::{Hostname, InvalidHostnameError},
    image::Image,
    platform::Platform,
    ulimit::{InvalidResourceError, Resource, Ulimit, Ulimits},
    user_or_group::UserOrGroup,
};

/// A service is an abstract definition of a computing resource within an application which can be
/// scaled or replaced independently from other components.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md)
#[derive(Serialize, Deserialize, Debug, compose_spec_macros::Default, Clone, PartialEq)]
pub struct Service {
    /// When defined and set to `false` Compose does not collect service logs, until you explicitly
    /// request it to.
    ///
    /// The default is `true`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#attach)
    #[serde(default = "default_true", skip_serializing_if = "skip_true")]
    #[default = true]
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
    pub depends_on: DependsOn,

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

    /// Custom DNS servers to set on the container network interface configuration.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#dns)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns: Option<ItemOrList<IpAddr>>,

    /// List of custom DNS options to be passed to the container's DNS resolver (`/etc/resolv.conf`
    /// file on Linux).
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#dns_opt)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub dns_opt: IndexSet<String>,

    /// Custom DNS search domains to set on the container network interface configuration.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#dns_search)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns_search: Option<ItemOrList<Hostname>>,

    /// Custom domain name to use for the service container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#domainname)
    #[serde(
        rename = "domainname",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub domain_name: Option<Hostname>,

    /// Overrides the default entrypoint declared by the container image.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#entrypoint)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<Command>,

    /// Add environment variables to the container from one or more files.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#env_file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_file: Option<EnvFile>,

    /// Define environment variables set in the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#environment)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub environment: ListOrMap,

    /// Incoming port or range of ports which are exposed from the service container to the host.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#expose)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub expose: IndexSet<Expose>,

    /// Share common configurations among different services or [`Compose`](super::Compose) files.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#extends)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<Extends>,

    /// Annotations for the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#annotations)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub annotations: ListOrMap,

    /// Link service containers to services managed externally.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#external_links)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub external_links: IndexSet<Link>,

    /// Add hostname mappings to the container network interface configuration
    /// (`/etc/hosts` for Linux).
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#extra_hosts)
    #[serde(
        default,
        skip_serializing_if = "IndexMap::is_empty",
        deserialize_with = "extra_hosts"
    )]
    pub extra_hosts: IndexMap<Hostname, IpAddr>,

    /// Additional groups which the user inside the container must be a member of.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#group_add)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub group_add: IndexSet<UserOrGroup>,

    /// A check that is run to determine whether the service container is "healthy".
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#healthcheck)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<Healthcheck>,

    /// A custom hostname to use for the service container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#hostname)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<Hostname>,

    /// Image to start the container from.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#image)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,

    /// Run an init process (PID 1) inside the container that forwards signals and reaps processes.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#init)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub init: bool,

    /// UTS namespace mode for the service container.
    ///
    /// The default is the decision of the container runtime, if supported.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#uts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uts: Option<Uts>,

    /// Specifies a build's container isolation technology.
    ///
    /// Supported values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#isolation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,

    /// Add metadata to containers.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#labels)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// Network links to containers in another service.
    ///
    /// Note: Availability of the `links` field is implementation specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#links)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub links: IndexSet<Link>,

    /// Logging configuration for the service.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#logging)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,

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

/// Override the default command or entrypoint declared by the container image.
///
/// [command compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#command)
///
/// [entrypoint compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#entrypoint)
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Command {
    /// Command run with `/bin/sh -c`.
    String(String),

    /// The command and its arguments.
    List(Vec<String>),
}

impl From<String> for Command {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<Vec<String>> for Command {
    fn from(value: Vec<String>) -> Self {
        Self::List(value)
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ItemOrListVisitor::<_, String>::new("a string or list of strings").deserialize(deserializer)
    }
}

/// Short or long [`depends_on`](Service#structfield.depends_on) syntax which expresses startup and
/// shutdown dependencies between services.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#depends_on)
pub type DependsOn = ShortOrLong<IndexSet<Identifier>, IndexMap<Identifier, Dependency>>;

/// Configuration of a [`Service`] dependency.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-1)
#[derive(Serialize, Deserialize, Debug, compose_spec_macros::Default, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Condition under which the dependency is considered satisfied.
    pub condition: Condition,

    /// When `true`, Compose restarts this service after it updates the dependency service.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub restart: bool,

    /// When `false`, Compose only warns you when the dependency service isn't started or available.
    ///
    /// Default is `true`.
    #[serde(default = "default_true", skip_serializing_if = "skip_true")]
    #[default = true]
    pub required: bool,
}

/// Condition under which a [`Service`] [`Dependency`] is considered satisfied.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-1)
#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// Dependency has started.
    #[default]
    ServiceStarted,

    /// Dependency is "healthy", as defined by its [`Healthcheck`].
    ServiceHealthy,

    /// Dependency ran to completion and exited successfully.
    ServiceCompletedSuccessfully,
}

impl<'a> AsShortIter<'a> for IndexMap<Identifier, Dependency> {
    type Iter = Keys<'a, Identifier, Dependency>;

    fn as_short_iter(&'a self) -> Option<Self::Iter> {
        let default_options = Dependency::default();
        self.values()
            .all(|options| *options == default_options)
            .then(|| self.keys())
    }
}

/// Returns `true` if `depends_on` is empty.
fn depends_on_is_empty(depends_on: &DependsOn) -> bool {
    match depends_on {
        ShortOrLong::Short(short) => short.is_empty(),
        ShortOrLong::Long(long) => long.is_empty(),
    }
}

/// Share common configurations among different [`Service`]s or [`Compose`](super::Compose) files.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#extends)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Extends {
    /// Name of the [`Service`] referenced as a base.
    pub service: Identifier,

    /// Location of a [`Compose`](super::Compose) configuration file defining the `service`.
    ///
    /// If [`None`], that indicates `service` refers to another service within this Compose file.
    /// May be an absolute path or a path relative to the directory of this Compose file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
}

/// Network link from a [`Service`] container to a container in another service in this
/// [`Compose`](crate::Compose) file (for `links`), or an externally managed container (for
/// `external_links`).
///
/// (De)serializes from/to a string in the format `{service}[:{alias}]`.
///
/// [`external_links` compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#external_links)
///
/// [`links` compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#links)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link {
    /// Externally managed container.
    pub service: Identifier,

    /// Optional alias.
    pub alias: Option<String>,
}

impl Link {
    /// Parse a [`Link`] from string in the format `{service}[:{alias}]`.
    ///
    /// # Errors
    ///
    /// Returns an error if the service is not a valid [`Identifier`].
    pub fn parse<T>(link: T) -> Result<Self, InvalidIdentifierError>
    where
        T: AsRef<str> + TryInto<Identifier>,
        T::Error: Into<InvalidIdentifierError>,
    {
        // Format is "{service}[:{alias}]".
        if let Some((service, alias)) = link.as_ref().split_once(':') {
            Ok(Self {
                service: service.parse()?,
                alias: Some(alias.to_owned()),
            })
        } else {
            // Reuse potential string allocation.
            link.try_into().map(Into::into).map_err(Into::into)
        }
    }
}

impl From<Identifier> for Link {
    fn from(service: Identifier) -> Self {
        Self {
            service,
            alias: None,
        }
    }
}

impl_from_str!(Link => InvalidIdentifierError);

impl Display for Link {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self { service, alias } = self;

        // Format is "{service}[:{alias}]".

        Display::fmt(service, f)?;

        if let Some(alias) = alias {
            write!(f, ":{alias}")?;
        }

        Ok(())
    }
}

impl From<Link> for String {
    fn from(value: Link) -> Self {
        if value.alias.is_none() {
            // Reuse `service`'s string allocation if there is no `alias`.
            value.service.into()
        } else {
            value.to_string()
        }
    }
}

/// Deserialize `extra_hosts` field of [`Service`] and long [`Build`] syntax.
///
/// Converts from [`ListOrMap`].
fn extra_hosts<'de, D>(deserializer: D) -> Result<IndexMap<Hostname, IpAddr>, D::Error>
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

            Ok((
                Hostname::new(key).map_err(de::Error::custom)?,
                value.parse().map_err(de::Error::custom)?,
            ))
        })
        .collect()
}

/// UTS namespace modes for [`Service`] containers.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#uts)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Uts {
    /// Use the same UTS namespace as the host.
    #[default]
    Host,
}

/// Logging configuration for a [`Service`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#logging)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Logging {
    /// Logging driver for the [`Service`] container.
    ///
    /// The default and available values are platform specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// Driver specific options.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub options: IndexMap<MapKey, Option<Value>>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
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
