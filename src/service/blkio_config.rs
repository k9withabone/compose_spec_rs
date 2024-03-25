//! Provides [`BlkioConfig`] for the `blkio_config` field of [`Service`](super::Service).

use std::{
    convert::Infallible,
    num::{NonZeroU16, TryFromIntError},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ByteValue;

/// Configuration options to set block IO limits for a [`Service`](super::Service).
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#blkio_config)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct BlkioConfig {
    /// Limit in bytes per second for read operations on a given device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_bps-device_write_bps)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_read_bps: Vec<BpsLimit>,

    /// Limit in operations per second for read operations on a given device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_iops-device_write_iops)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_read_iops: Vec<IopsLimit>,

    /// Limit in bytes per second for write operations on a given device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_bps-device_write_bps)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_write_bps: Vec<BpsLimit>,

    /// Limit in operations per second for write operations on a given device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_iops-device_write_iops)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_write_iops: Vec<IopsLimit>,

    /// Proportion of bandwidth allocated to a [`Service`](super::Service) relative to other services.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#weight)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<Weight>,

    /// Fine-tune bandwidth allocation by device.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#weight_device)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weight_device: Vec<WeightDevice>,
}

/// Limit in bytes per second for read/write operations on a given device.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_bps-device_write_bps)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BpsLimit {
    /// Symbolic path to the affected device.
    pub path: PathBuf,

    /// Bytes per second rate limit.
    pub rate: ByteValue,
}

/// Limit in operations per second for read/write operations on a given device.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_read_iops-device_write_iops)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IopsLimit {
    /// Symbolic path to the affected device.
    pub path: PathBuf,

    /// Operations per second rate limit.
    pub rate: u64,
}

/// Fine-tune bandwidth allocation by device.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#weight_device)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WeightDevice {
    /// Symbolic path to the affected device.
    pub path: PathBuf,

    /// Proportion of bandwidth allocated to the device.
    pub weight: Weight,
}

/// Proportion of bandwidth allocated to a [`Service`](super::Service) relative to other services.
///
/// Must be between 10 and 1000, inclusive. 500 is the default.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(into = "u16", try_from = "u16")]
pub struct Weight(NonZeroU16);

impl Weight {
    /// The default value, 500.
    // TODO: Remove unsafe once `Option::expect()` in const is
    //       [stable](https://github.com/rust-lang/rust/issues/67441). Then, replace it with
    //       `NonZeroU16::new(500).expect("500 is not zero")` and remove clippy allow above.
    // SAFETY: 500 is not zero.
    pub const DEFAULT: Self = Self(unsafe { NonZeroU16::new_unchecked(500) });

    /// Create a new [`Weight`].
    ///
    /// # Errors
    ///
    /// Returns an error if the given weight is not between 10 and 1000, inclusive.
    pub fn new<T>(weight: T) -> Result<Self, WeightRangeError>
    where
        T: TryInto<NonZeroU16>,
        WeightRangeError: From<T::Error>,
    {
        let weight = weight.try_into()?;
        match weight.get() {
            10..=1000 => Ok(Self(weight)),
            _ => Err(WeightRangeError { source: None }),
        }
    }

    /// Return the inner value.
    #[must_use]
    pub fn into_inner(self) -> NonZeroU16 {
        self.0
    }
}

/// Error returned when attempting to create a [`Weight`] and the number is not between 10 and 1000,
/// inclusive.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("weights must be between 10 and 1000")]
pub struct WeightRangeError {
    /// Source of the error when converting into a [`NonZeroU16`] fails.
    #[from]
    source: Option<TryFromIntError>,
}

impl From<Infallible> for WeightRangeError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl Default for Weight {
    /// Default [`Weight`] value, 500.
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<NonZeroU16> for Weight {
    type Error = WeightRangeError;

    fn try_from(value: NonZeroU16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<u16> for Weight {
    type Error = WeightRangeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Weight> for NonZeroU16 {
    fn from(value: Weight) -> Self {
        value.into_inner()
    }
}

impl From<Weight> for u16 {
    fn from(value: Weight) -> Self {
        value.into_inner().into()
    }
}

impl PartialEq<NonZeroU16> for Weight {
    fn eq(&self, other: &NonZeroU16) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<u16> for Weight {
    fn eq(&self, other: &u16) -> bool {
        self.0.get().eq(other)
    }
}
