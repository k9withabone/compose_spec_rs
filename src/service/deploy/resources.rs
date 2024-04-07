//! Provides [`Resources`] for the `resources` field of [`Deploy`](super::Deploy).

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexSet;
use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;

use crate::{
    impl_from_str, impl_try_from, serde::forward_visitor, service::ByteValue, Extensions, ListOrMap,
};

/// Physical resource constraints for the service container to run on the platform.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Resources {
    /// Limits on resources a container may allocate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,

    /// Resources the platform must guarantee the container can allocate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservations: Option<Reservations>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Resources {
    /// Returns `true` if all fields are [`None`] or empty.
    ///
    /// The `limits` field counts as empty if it is [`None`] or [empty](Limits::is_empty()).
    ///
    /// The `reservations` field counts as empty if it is [`None`] or
    /// [empty](Reservations::is_empty()).
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::deploy::{Resources, resources::Limits};
    ///
    /// let mut resources = Resources::default();
    /// assert!(resources.is_empty());
    ///
    /// resources.limits = Some(Limits::default());
    /// assert!(resources.is_empty());
    ///
    /// resources.limits = Some(Limits {
    ///     pids: Some(100),
    ///     ..Limits::default()
    /// });
    /// assert!(!resources.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            limits,
            reservations,
            extensions,
        } = self;

        !limits.as_ref().is_some_and(|limits| !limits.is_empty())
            && !reservations
                .as_ref()
                .is_some_and(|reservations| !reservations.is_empty())
            && extensions.is_empty()
    }
}

/// Limits on [`Resources`] a container may allocate.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Limits {
    /// How much of the available CPU resources, as number of cores, a container can use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#cpus)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpus: Option<Cpus>,

    /// The amount of memory a container can allocate.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#memory)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<ByteValue>,

    /// Tune a container's PIDs limit.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#pids)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pids: Option<u32>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Limits {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            cpus,
            memory,
            pids,
            extensions,
        } = self;

        cpus.is_none() && memory.is_none() && pids.is_none() && extensions.is_empty()
    }
}

/// [`Resources`] the platform must guarantee the container can allocate.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Reservations {
    /// How much of the available CPU resources, as number of cores, a container reserves for use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#cpus)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpus: Option<Cpus>,

    /// The amount of memory a container reserves for use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#memory)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<ByteValue>,

    /// Devices a container can use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#devices)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Device>,

    /// Generic resources to reserve.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_resources: Vec<GenericResource>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Reservations {
    /// Returns `true` if all fields are [`None`] or empty.
    ///
    /// The `devices` field counts as empty if all [`Device`]s are [empty](Device::is_empty()) or
    /// the [`Vec`] is empty.
    ///
    /// The `generic_resources` field counts as empty if all [`GenericResource`]s are
    /// [empty](GenericResource::is_empty()) or the [`Vec`] is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::deploy::resources::{Device, Reservations};
    ///
    /// let mut reservations = Reservations::default();
    /// assert!(reservations.is_empty());
    ///
    /// reservations.devices.push(Device::default());
    /// assert!(reservations.is_empty());
    ///
    /// reservations.devices.push(Device::new(["capability"]));
    /// assert!(!reservations.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            cpus,
            memory,
            devices,
            generic_resources,
            extensions,
        } = self;

        cpus.is_none()
            && memory.is_none()
            && devices.iter().all(Device::is_empty)
            && generic_resources.iter().all(GenericResource::is_empty)
            && extensions.is_empty()
    }
}

/// How much of the available CPU resources, as number of cores, a container reserves for use.
///
/// Must be a positive and finite number.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#cpus)
#[derive(Serialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[serde(into = "f64")]
pub struct Cpus(f64);

impl Cpus {
    /// Create a new [`Cpus`].
    ///
    /// # Errors
    ///
    /// Returns an error if the value is not positive or finite.
    pub fn new<T: Into<f64>>(cpus: T) -> Result<Self, InvalidCpusError> {
        let cpus = cpus.into();
        if !cpus.is_sign_positive() {
            Err(InvalidCpusError::Negative)
        } else if !cpus.is_finite() {
            Err(InvalidCpusError::Infinite)
        } else {
            Ok(Self(cpus))
        }
    }

    /// Return the inner value.
    #[must_use]
    pub const fn into_inner(self) -> f64 {
        self.0
    }
}

/// Error returned when creating [`Cpus`] fails.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidCpusError {
    /// Number was negative.
    #[error("cpus cannot be negative")]
    Negative,

    /// Number was infinite or NaN.
    #[error("cpus must be a finite number")]
    Infinite,
}

impl_try_from!(Cpus::new -> InvalidCpusError, f32, f64, i8, i16, i32);

impl From<u32> for Cpus {
    fn from(value: u32) -> Self {
        Self(value.into())
    }
}

impl From<u16> for Cpus {
    fn from(value: u16) -> Self {
        u32::from(value).into()
    }
}

impl From<u8> for Cpus {
    fn from(value: u8) -> Self {
        u32::from(value).into()
    }
}

impl From<Cpus> for f64 {
    fn from(value: Cpus) -> Self {
        value.into_inner()
    }
}

impl PartialEq<f64> for Cpus {
    fn eq(&self, other: &f64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<f64> for Cpus {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl<'de> Deserialize<'de> for Cpus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(CpusVisitor)
    }
}

/// [`Visitor`] for deserializing [`Cpus`].
struct CpusVisitor;

impl<'de> Visitor<'de> for CpusVisitor {
    type Value = Cpus;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a number")
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        self.visit_u32(v.into())
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        self.visit_u32(v.into())
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(v.into())
    }

    forward_visitor! {
        visit_f64,
        visit_i8: i8,
        visit_i16: i16,
        visit_i32: i32,
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        v.try_into().map_err(E::custom)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_f64(v.parse().map_err(E::custom)?)
    }
}

/// A device a container may [reserve](Reservations).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#devices)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Device {
    /// Generic and driver specific device capabilities.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#capabilities)
    pub capabilities: IndexSet<Capability>,

    /// A different driver for the reserved device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#driver)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// Number of devices matching the specified capabilities to reserve.
    ///
    /// Conflicts with `device_ids`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#count)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<Count>,

    /// Reserve devices with the specified IDs provided they satisfy the requested capabilities.
    ///
    /// Conflicts with `count`.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#device_ids)
    #[serde(default, skip_serializing_if = "IndexSet::is_empty")]
    pub device_ids: IndexSet<String>,

    /// Driver specific options.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#options)
    #[serde(default, skip_serializing_if = "ListOrMap::is_empty")]
    pub options: ListOrMap,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Device {
    /// Create a new [`Device`] from an iterator of capabilities.
    pub fn new<I>(capabilities: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Capability>,
    {
        Self {
            capabilities: capabilities.into_iter().map(Into::into).collect(),
            driver: None,
            count: None,
            device_ids: IndexSet::new(),
            options: ListOrMap::default(),
            extensions: Extensions::new(),
        }
    }

    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            capabilities,
            driver,
            count,
            device_ids,
            options,
            extensions,
        } = self;

        capabilities.is_empty()
            && driver.is_none()
            && count.is_none()
            && device_ids.is_empty()
            && options.is_empty()
            && extensions.is_empty()
    }
}

/// [`Device`] capability.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#capabilities)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Graphics accelerator.
    Gpu,

    /// AI accelerator.
    Tpu,

    /// Other, driver specific, capability.
    Other(String),
}

impl Capability {
    /// [`Self::Gpu`] string value.
    const GPU: &'static str = "gpu";

    /// [`Self::Tpu`] string value.
    const TPU: &'static str = "tpu";

    /// Parse a [`Capability`] from a string.
    pub fn parse<T>(capability: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match capability.as_ref() {
            Self::GPU => Self::Gpu,
            Self::TPU => Self::Tpu,
            _ => Self::Other(capability.into()),
        }
    }

    /// Returns `true` if the capability is [`Gpu`].
    ///
    /// [`Gpu`]: Capability::Gpu
    #[must_use]
    pub const fn is_gpu(&self) -> bool {
        matches!(self, Self::Gpu)
    }

    /// Returns `true` if the capability is [`Tpu`].
    ///
    /// [`Tpu`]: Capability::Tpu
    #[must_use]
    pub const fn is_tpu(&self) -> bool {
        matches!(self, Self::Tpu)
    }

    /// Capability as a string slice.
    ///
    /// Convenience method for `as_ref()` to a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Gpu => Self::GPU,
            Self::Tpu => Self::TPU,
            Self::Other(other) => other,
        }
    }
}

impl_from_str!(Capability);

impl AsRef<str> for Capability {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Capability> for String {
    fn from(value: Capability) -> Self {
        match value {
            Capability::Gpu | Capability::Tpu => value.as_str().to_owned(),
            Capability::Other(other) => other,
        }
    }
}

impl From<Capability> for Cow<'static, str> {
    fn from(value: Capability) -> Self {
        match value {
            Capability::Gpu => Self::Borrowed(Capability::GPU),
            Capability::Tpu => Self::Borrowed(Capability::TPU),
            Capability::Other(other) => Self::Owned(other),
        }
    }
}

/// Number of [`Device`]s matching the specified capabilities to [reserve](Reservations).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#count)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    /// Reserve all devices that satisfy the requested capabilities.
    #[default]
    All,

    /// Reserve a specific number of devices.
    Integer(u64),
}

impl Count {
    /// [`Self::All`] string value.
    const ALL: &'static str = "all";
}

impl From<u64> for Count {
    fn from(value: u64) -> Self {
        Self::Integer(value)
    }
}

impl PartialEq<u64> for Count {
    fn eq(&self, other: &u64) -> bool {
        match self {
            Self::All => false,
            Self::Integer(count) => count.eq(other),
        }
    }
}

impl FromStr for Count {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::ALL {
            Ok(Self::All)
        } else {
            s.parse().map(Self::Integer)
        }
    }
}

impl Serialize for Count {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => serializer.serialize_str(Self::ALL),
            Self::Integer(count) => serializer.serialize_u64(*count),
        }
    }
}

impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(CountVisitor)
    }
}

/// [`Visitor`] for deserializing [`Count`].
struct CountVisitor;

impl<'de> Visitor<'de> for CountVisitor {
    type Value = Count;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("\"all\" or an integer")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Count::Integer(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        #[allow(clippy::map_err_ignore)]
        v.parse()
            .map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
    }
}

/// Generic [`Resources`] to [reserve](Reservations).
// TODO: Update once [compose-spec#469](https://github.com/compose-spec/compose-spec/issues/469)
// is resolved.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct GenericResource {
    /// Discrete resource spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrete_resource_spec: Option<DiscreteResourceSpec>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl GenericResource {
    /// Returns `true` if all fields are [`None`] or empty.
    ///
    /// The `discrete_resource_spec` field counts as empty if it is [`None`] or
    /// [empty](DiscreteResourceSpec::is_empty()).
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::deploy::resources::{DiscreteResourceSpec, GenericResource};
    ///
    /// let mut resource = GenericResource::default();
    /// assert!(resource.is_empty());
    ///
    /// resource.discrete_resource_spec = Some(DiscreteResourceSpec::default());
    /// assert!(resource.is_empty());
    ///
    /// resource.discrete_resource_spec = Some(DiscreteResourceSpec {
    ///     kind: Some("kind".to_owned()),
    ///     ..DiscreteResourceSpec::default()
    /// });
    /// assert!(!resource.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            discrete_resource_spec,
            extensions,
        } = self;

        !discrete_resource_spec
            .as_ref()
            .is_some_and(|discrete_resource_spec| !discrete_resource_spec.is_empty())
            && extensions.is_empty()
    }
}

/// Discrete resource spec.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct DiscreteResourceSpec {
    /// Discrete resource spec kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Discrete resource spec value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl DiscreteResourceSpec {
    /// Returns `true` if all fields are [`None`] or empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self {
            kind,
            value,
            extensions,
        } = self;

        kind.is_none() && value.is_none() && extensions.is_empty()
    }
}
