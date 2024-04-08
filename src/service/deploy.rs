//! Provides [`Deploy`] for the `deploy` field of [`Service`](super::Service).

mod endpoint_mode;
pub mod resources;

use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{serde::duration_option, Extensions, ListOrMap};

pub use self::{endpoint_mode::EndpointMode, resources::Resources};

/// Declare additional metadata on [`Service`]s for allocating and configuring platform resources.
///
/// Note: Deploy is an optional part of the Compose specification.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md)
///
/// [`Service`]: super::Service
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Deploy {
    /// Service discovery method for external clients connecting to a service.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#endpoint_mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_mode: Option<EndpointMode>,

    /// Specify metadata for the service.
    ///
    /// These labels are only set on the service and not on any containers for the service. This
    /// assumes the platform has some native concept of a "service" that matches the Compose
    /// application model.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#labels)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub labels: ListOrMap,

    /// The replication model used to run the service on the platform.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,

    /// Constraints and preferences for the platform to select a physical node to run service
    /// containers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,

    /// If the service is [`Replicated`](Mode::Replicated) (which is the default), the number of
    /// containers that should be running at any given time.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#replicas)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u64>,

    /// Physical resource constraints for the service container to run on the platform.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,

    /// If and how to restart containers when they exit.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#restart_policy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<RestartPolicy>,

    /// How the service should be rolled back in case of a failing update.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#rollback_config)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_config: Option<UpdateOrRollbackConfig>,

    /// How the service should be updated. Useful for configuring rolling updates.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#update_config)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_config: Option<UpdateOrRollbackConfig>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Deploy {
    /// Returns `true` if all fields are [`None`] or empty.
    ///
    /// The `placement`, `resources`, `restart_policy`, `rollback_config`, and `update_config`
    /// fields count as empty if they are [`None`] or contain an empty value.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::{Deploy, deploy::Placement};
    ///
    /// let mut deploy = Deploy::default();
    /// assert!(deploy.is_empty());
    ///
    /// deploy.placement = Some(Placement::default());
    /// assert!(deploy.is_empty());
    ///
    /// deploy.placement = Some(Placement {
    ///     constraints: vec!["constraint".to_owned()],
    ///     ..Placement::default()
    /// });
    /// assert!(!deploy.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            endpoint_mode,
            labels,
            mode,
            placement,
            replicas,
            resources,
            restart_policy,
            rollback_config,
            update_config,
            extensions,
        } = self;

        endpoint_mode.is_none()
            && labels.is_empty()
            && mode.is_none()
            && !placement
                .as_ref()
                .is_some_and(|placement| !placement.is_empty())
            && replicas.is_none()
            && !resources
                .as_ref()
                .is_some_and(|resources| !resources.is_empty())
            && !restart_policy
                .as_ref()
                .is_some_and(|restart| !restart.is_empty())
            && !rollback_config
                .as_ref()
                .is_some_and(|rollback| !rollback.is_empty())
            && !update_config
                .as_ref()
                .is_some_and(|update| !update.is_empty())
            && extensions.is_empty()
    }
}

/// The replication model used to run the service on the platform.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#mode)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Exactly one container per physical node.
    Global,

    /// A specified number of containers.
    #[default]
    Replicated,
}

impl Mode {
    /// Replication mode as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Replicated => "replicated",
        }
    }
}

impl AsRef<str> for Mode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Constraints and preferences for the platform to select a physical node to run
/// [`Service`](crate::Service) containers.
// TODO: Update once [compose-spec#469](https://github.com/compose-spec/compose-spec/issues/469)
// is resolved. The specification and schema do not currently agree on the structure of this.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Placement {
    /// Required property the platform's node must fulfill to run the service container.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,

    /// Properties the platform's node should fulfill to run service container.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferences: Vec<Preference>,

    /// Maximum number of replicas of a service container that should be on a single node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replicas_per_node: Option<u64>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Placement {
    /// Returns `true` if all fields are empty or [`None`].
    ///
    /// The `preferences` field counts as empty if all [`Preference`]s are
    /// [empty](Preference::is_empty()) or if the [`Vec`] is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::deploy::{Placement, Preference};
    ///
    /// let mut placement = Placement::default();
    /// assert!(placement.is_empty());
    ///
    /// placement.preferences.push(Preference::default());
    /// assert!(placement.is_empty());
    ///
    /// placement.preferences.push(Preference {
    ///     spread: Some("spread".to_owned()),
    ///     ..Preference::default()
    /// });
    /// assert!(!placement.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            constraints,
            preferences,
            max_replicas_per_node,
            extensions,
        } = self;

        constraints.is_empty()
            && preferences.iter().all(Preference::is_empty)
            && max_replicas_per_node.is_none()
            && extensions.is_empty()
    }
}

/// A property the platform's node should fulfill to run service container.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Preference {
    /// Preferred spread of service container replicas across nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<String>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Preference {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self { spread, extensions } = self;

        spread.is_none() && extensions.is_empty()
    }
}

/// If and how to restart containers when they exit.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#restart_policy)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct RestartPolicy {
    /// When to restart containers based on their exit status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<RestartCondition>,

    /// How long to wait between restart attempts.
    ///
    /// The default is 0, meaning restart attempts can occur immediately.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub delay: Option<Duration>,

    /// How many times to attempt to restart a container before giving up.
    ///
    /// The default is to never give up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u64>,

    /// How long to wait before deciding if a restart has succeeded.
    ///
    /// The default is to decide immediately.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub window: Option<Duration>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl RestartPolicy {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            condition,
            delay,
            max_attempts,
            window,
            extensions,
        } = self;

        condition.is_none()
            && delay.is_none()
            && max_attempts.is_none()
            && window.is_none()
            && extensions.is_empty()
    }
}

/// When to restart containers based on their exit status.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#restart_policy)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RestartCondition {
    /// Containers are not automatically restarted regardless of the exit status.
    None,

    /// Containers are restarted if they exit with a non-zero exit code.
    OnFailure,

    /// Containers are restarted regardless of the exit status.
    #[default]
    Any,
}

impl RestartCondition {
    /// Restart condition as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::OnFailure => "on-failure",
            Self::Any => "any",
        }
    }
}

impl AsRef<str> for RestartCondition {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for RestartCondition {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Update or rollback configuration.
///
/// # Update Config
///
/// How the [`Service`] should be updated. Useful for configuring rolling updates.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#update_config)
///
/// # Rollback Config
///
/// How the [`Service`] should be rolled back in case of a failing update.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#rollback_config)
///
/// [`Service`]: super::Service
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct UpdateOrRollbackConfig {
    /// Number of containers to update/rollback at a time.
    ///
    /// If set to 0, all containers are updated / rolled back simultaneously.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<u64>,

    /// Time to wait between each container group's update/rollback.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub delay: Option<Duration>,

    /// What to do if an update/rollback fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_action: Option<FailureAction>,

    /// Duration after each task update/rollback to monitor for failure.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub monitor: Option<Duration>,

    /// Failure rate to tolerate during an update/rollback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_failure_ratio: Option<u64>,

    /// Order of operations during updates/rollbacks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Order>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl UpdateOrRollbackConfig {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            parallelism,
            delay,
            failure_action,
            monitor,
            max_failure_ratio,
            order,
            extensions,
        } = self;

        parallelism.is_none()
            && delay.is_none()
            && failure_action.is_none()
            && monitor.is_none()
            && max_failure_ratio.is_none()
            && order.is_none()
            && extensions.is_empty()
    }
}

/// What to do if an [update or rollback](UpdateOrRollbackConfig) fails.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#rollback_config)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FailureAction {
    /// Continue the update/rollback.
    Continue,

    /// Pause the update/rollback.
    #[default]
    Pause,
}

impl FailureAction {
    /// Failure action as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Pause => "pause",
        }
    }
}

impl AsRef<str> for FailureAction {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for FailureAction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Order of operations during [updates or rollbacks](UpdateOrRollbackConfig).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#rollback_config)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// The old task is stopped before starting the new one.
    #[default]
    StopFirst,

    /// The new task is started first, and the running tasks briefly overlap.
    StartFirst,
}

impl Order {
    /// Order as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StopFirst => "stop-first",
            Self::StartFirst => "start-first",
        }
    }
}

impl AsRef<str> for Order {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
