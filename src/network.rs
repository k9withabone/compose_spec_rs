//! Provides [`Network`] for the top-level `networks` field of a [`Compose`](super::Compose) file.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use serde::{Deserialize, Serialize};

use crate::{impl_from_str, Extensions, Map};

/// A named network which allows for [`Service`](super::Service)s to communicate with each other.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Network {
    /// Which driver to use for this network.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#driver)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<Driver>,

    /// Driver-dependent options.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/06-networks.md#driver_opts)
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub driver_opts: Map,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// [`Network`] driver.
///
/// Default and available values are platform specific.
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum Driver {
    /// Use the host's networking stack.
    Host,
    /// Turn off networking.
    None,
    /// Other network driver.
    Other(String),
}

impl Driver {
    /// [`Self::Host`] string value.
    const HOST: &'static str = "host";

    /// [`Self::None`] string value.
    const NONE: &'static str = "none";

    /// Parse a [`Driver`] from a string.
    pub fn parse<T>(driver: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        match driver.as_ref() {
            Self::HOST => Self::Host,
            Self::NONE => Self::None,
            _ => Self::Other(driver.into()),
        }
    }

    /// Network driver as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Host => Self::HOST,
            Self::None => Self::NONE,
            Self::Other(other) => other,
        }
    }
}

impl_from_str!(Driver);

impl AsRef<str> for Driver {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Driver {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Driver> for Cow<'static, str> {
    fn from(value: Driver) -> Self {
        match value {
            Driver::Host => Driver::HOST.into(),
            Driver::None => Driver::NONE.into(),
            Driver::Other(other) => other.into(),
        }
    }
}

impl From<Driver> for String {
    fn from(value: Driver) -> Self {
        Cow::from(value).into_owned()
    }
}
