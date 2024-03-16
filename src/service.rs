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
mod memswap_limit;
pub mod network_config;
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
    common::key_impls,
    impl_from_str,
    serde::{
        default_true, display_from_str_option, duration_option, duration_us_option, skip_true,
        ItemOrListVisitor,
    },
    AsShortIter, Extensions, Identifier, InvalidIdentifierError, ItemOrList, ListOrMap, Map,
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
    memswap_limit::MemswapLimit,
    network_config::{MacAddress, NetworkConfig},
    platform::Platform,
    ports::Ports,
    ulimit::{InvalidResourceError, Resource, Ulimit, Ulimits},
    user_or_group::UserOrGroup,
};

/// A service is an abstract definition of a computing resource within an application which can be
/// scaled or replaced independently from other components.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md)
#[allow(clippy::struct_excessive_bools)]
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

    /// Network configuration of the service container.
    ///
    /// Represents either the
    /// [`network_mode`](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#network_mode)
    /// or [`networks`](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#networks)
    /// field of the compose service spec.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten, with = "network_config::option")]
    pub network_config: Option<NetworkConfig>,

    /// MAC address for the service container.
    ///
    /// Note: Container runtimes might reject this value. In that case you should use the
    /// `mac_address` field of [`Network`](network_config::Network) instead.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#mac_address-1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<MacAddress>,

    /// Percentage of anonymous pages the host kernel is allowed to swap.
    ///
    /// The default is platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#mem_swappiness)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_swappiness: Option<Percent>,

    /// The amount of memory the container is allowed to swap to disk.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#memswap_limit)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memswap_limit: Option<MemswapLimit>,

    /// Whether to disable the OOM killer for the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#oom_kill_disable)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub oom_kill_disable: bool,

    /// Preference for the container to be killed by the platform in the case of memory starvation.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#oom_score_adj)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oom_score_adj: Option<OomScoreAdj>,

    /// PID mode for the container.
    ///
    /// Supported values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#pid)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<String>,

    /// Target platform for the container to run on.
    ///
    /// Used to determine which version of the container image is pulled and/or which platform the
    /// image is built for.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#platform)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,

    /// Container ports to publish to the host.
    ///
    /// Note: Port mapping cannot be used with [`NetworkMode::Host`](network_config::NetworkMode::Host).
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ports)
    #[serde(default, skip_serializing_if = "Ports::is_empty")]
    pub ports: Ports,

    /// Whether to to run the container with elevated privileges.
    ///
    /// Support and actual impacts are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#privileged)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub privileged: bool,

    /// List of named profiles for the service to be enabled under.
    ///
    /// If empty, the service is always enabled.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#profiles)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub profiles: IndexSet<Profile>,

    /// When the platform should pull the service's image.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#pull_policy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_policy: Option<PullPolicy>,

    /// Whether the service container should be created with a read-only filesystem.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#privileged)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub read_only: bool,

    /// Restart policy that the platform applies on container termination.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#restart)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<Restart>,

    /// Runtime to use for the container.
    ///
    /// Available values are implementation specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#runtime)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,

    /// Default number of containers to deploy for this service.
    ///
    /// Must be consistent with the `replicas` field in [`Deploy`] if both are set.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#scale)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<u64>,

    /// Grant access to sensitive data defined by secrets.
    ///
    /// Services can only access secrets when explicitly granted by the `secrets` field.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#secrets)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<ShortOrLong<Identifier, ConfigOrSecret>>,

    /// Container security options.
    ///
    /// Available values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#security_opt)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub security_opt: IndexSet<String>,

    /// Size of the shared memory (`/dev/shm` on Linux) allowed for the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#shm_size)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shm_size: Option<ByteValue>,

    /// Whether to run the container with an allocated stdin.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#stdin_open)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub stdin_open: bool,

    /// How long to wait when attempting to stop a container before sending `SIGKILL`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#stop_grace_period)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub stop_grace_period: Option<Duration>,

    /// Signal to use to stop the container.
    ///
    /// Default is `SIGTERM`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#stop_signal)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_signal: Option<String>,

    /// Storage driver options.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#storage_opt)
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub storage_opt: Map,

    /// Kernel parameters to set in the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#sysctls)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub sysctls: ListOrMap,

    /// Mount temporary file systems inside the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#tmpfs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tmpfs: Option<ItemOrList<PathBuf>>,

    /// Whether to run the container with a TTY.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#tty)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub tty: bool,

    /// Override the default ulimits for the container.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#ulimits)
    #[serde(default, skip_serializing_if = "Ulimits::is_empty")]
    pub ulimits: Ulimits,

    /// Override the user used to run the container process.
    ///
    /// The default is set by the image or is `root`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#user)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "display_from_str_option::serialize"
    )]
    pub user: Option<UserOrGroup>,

    /// User namespace mode for the container.
    ///
    /// Supported values are platform specific.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#userns_mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userns_mode: Option<String>,

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
    pub fn new(percent: u8) -> Result<Self, RangeError> {
        match percent {
            0..=100 => Ok(Self(percent)),
            value => Err(RangeError {
                value: value.into(),
                start: 0,
                end: 100,
            }),
        }
    }

    /// Return the inner value.
    #[must_use]
    pub fn into_inner(self) -> u8 {
        self.0
    }
}

/// Error returned when trying to convert an integer into a type with a limited range.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("value `{value}` is not between {start} and {end}")]
pub struct RangeError {
    /// Value attempted to convert from.
    value: i64,
    /// Start of the valid range.
    start: i64,
    /// End of the valid range.
    end: i64,
}

impl TryFrom<u8> for Percent {
    type Error = RangeError;

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

impl Condition {
    /// Dependency condition as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ServiceStarted => "service_started",
            Self::ServiceHealthy => "service_healthy",
            Self::ServiceCompletedSuccessfully => "service_completed_successfully",
        }
    }
}

impl AsRef<str> for Condition {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Condition {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
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

impl Uts {
    /// UTS namespace mode as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "host"
    }
}

impl AsRef<str> for Uts {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Uts {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub options: Map,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Preference for a [`Service`] container to be killed by the platform in the case of memory
/// starvation.
///
/// Must be between -1000 and 1000, inclusive.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#oom_score_adj)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(into = "i16", try_from = "i16")]
pub struct OomScoreAdj(i16);

impl OomScoreAdj {
    /// Create a new [`OomScoreAdj`].
    ///
    /// # Errors
    ///
    /// Returns an error if the value is not between -1000 and 1000, inclusive.
    pub fn new(oom_score_adj: i16) -> Result<Self, RangeError> {
        match oom_score_adj {
            -1000..=1000 => Ok(Self(oom_score_adj)),
            value => Err(RangeError {
                value: value.into(),
                start: -1000,
                end: 1000,
            }),
        }
    }
}

impl TryFrom<i16> for OomScoreAdj {
    type Error = RangeError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OomScoreAdj> for i16 {
    fn from(value: OomScoreAdj) -> Self {
        value.0
    }
}

impl PartialEq<i16> for OomScoreAdj {
    fn eq(&self, other: &i16) -> bool {
        self.0.eq(other)
    }
}

/// Profile for [`Service`] to be enabled under.
///
/// Profiles must be a valid [`Identifier`] and cannot start with an underscore (_), dot (.), or
/// dash (-).
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Profile(Identifier);

impl Profile {
    /// Create a new [`Profile`] from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string starts with an underscore (_), dot (.), or dash (-), or is
    /// not a valid [`Identifier`].
    pub fn new<T>(profile: T) -> Result<Self, InvalidProfileError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        // Regex pattern from compose-spec: [a-zA-Z0-9][a-zA-Z0-9_.-]+
        if profile.as_ref().starts_with(['_', '.', '-']) {
            Err(InvalidProfileError::Start)
        } else {
            Ok(Self(Identifier::new(profile)?))
        }
    }
}

/// Error returned when creating a new [`Profile`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InvalidProfileError {
    /// Profile started with an underscore (_), dot (.), or dash (-).
    #[error("profile cannot start with an underscore (_), dot (.), or dash (-)")]
    Start,

    /// Profile was not a valid [`Identifier`].
    #[error("profile not a valid identifier")]
    Identifier(#[from] InvalidIdentifierError),
}

key_impls!(Profile => InvalidProfileError);

impl From<Profile> for Identifier {
    fn from(value: Profile) -> Self {
        value.0
    }
}

/// When the platform should pull a [`Service`]'s [`Image`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#pull_policy)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PullPolicy {
    /// Always pull the image from the registry.
    Always,

    /// Never pull the image from the registry and rely on the platform cached image.
    ///
    /// If there is no cached image, a failure is reported.
    Never,

    /// Pull the image only if it's not available in the platform cache.
    #[serde(alias = "if_not_present")]
    #[default]
    Missing,

    /// Build the image.
    Build,
}

impl PullPolicy {
    /// Pull policy as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::Missing => "missing",
            Self::Build => "build",
        }
    }
}

impl AsRef<str> for PullPolicy {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for PullPolicy {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Restart policy that the platform applies on [`Service`] container termination.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#restart)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Restart {
    /// Do not restart the container under any circumstance.
    #[default]
    No,
    /// Always restart the container until its removal.
    Always,
    /// Restart the container if the exit code indicates an error.
    OnFailure,
    /// Restart the container irrespective of the exit code, but stops restarting when the service
    /// is stopped or removed.
    UnlessStopped,
}

impl Restart {
    /// Restart policy as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::No => "no",
            Self::Always => "always",
            Self::OnFailure => "on-failure",
            Self::UnlessStopped => "unless-stopped",
        }
    }
}

impl AsRef<str> for Restart {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Restart {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
