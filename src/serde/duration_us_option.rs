//! (De)serialize an [`Option<Duration>`] from/to microseconds.

use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize an [`Option<Duration>`] as microseconds.
pub(crate) fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration
        .as_ref()
        .map(Duration::as_micros)
        .serialize(serializer)
}

/// Deserialize an [`Option<Duration>`] from microseconds.
pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<u64>::deserialize(deserializer).map(|micros| micros.map(Duration::from_micros))
}
