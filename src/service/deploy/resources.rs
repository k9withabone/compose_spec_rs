//! Provides [`Resources`] for the `resources` field of [`Deploy`](super::Deploy).

use std::{
    convert::Infallible,
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

use crate::{service::ByteValue, Extensions, ListOrMap};

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

/// Limits on [`Resources`] a container may allocate.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Limits {
    /// How much of the available CPU resources, as number of cores, a container can use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#cpus)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpus: Option<f64>,

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

/// [`Resources`] the platform must guarantee the container can allocate.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#resources)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Reservations {
    /// How much of the available CPU resources, as number of cores, a container reserves for use.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#cpus)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpus: Option<f64>,

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

/// A device a container may [reserve](Reservations).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/deploy.md#devices)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

    /// Returns `true` if the capability is [`Gpu`].
    ///
    /// [`Gpu`]: Capability::Gpu
    #[must_use]
    pub fn is_gpu(&self) -> bool {
        matches!(self, Self::Gpu)
    }

    /// Returns `true` if the capability is [`Tpu`].
    ///
    /// [`Tpu`]: Capability::Tpu
    #[must_use]
    pub fn is_tpu(&self) -> bool {
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

impl From<&str> for Capability {
    fn from(value: &str) -> Self {
        match value {
            Self::GPU => Self::Gpu,
            Self::TPU => Self::Tpu,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl FromStr for Capability {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<String> for Capability {
    fn from(value: String) -> Self {
        match value.as_str() {
            Self::GPU => Self::Gpu,
            Self::TPU => Self::Tpu,
            _ => Self::Other(value),
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
        if s == "all" {
            Ok(Self::All)
        } else {
            s.parse().map(Self::Integer)
        }
    }
}

impl Serialize for Count {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => serializer.serialize_str("all"),
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
