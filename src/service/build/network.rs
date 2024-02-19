//! Provides [`Network`] for the `network` field of the long [`Build`](super::Build) syntax.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};

/// Network containers connect to during [`Build`](super::Build) for `RUN` instructions.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#network)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// Network to connect to during build.
    ///
    /// A compose implementation may have more specific network kinds such as "host".
    String(String),
    /// Disable networking during build.
    None,
}

impl Network {
    /// Parse [`Network`] from a string.
    ///
    /// "none" converts to [`Network::None`].
    pub fn parse<T>(network: T) -> Self
    where
        T: AsRef<str> + Into<String>,
    {
        if network.as_ref() == "none" {
            Self::None
        } else {
            Self::String(network.into())
        }
    }

    /// String slice of the [`Network`].
    ///
    /// If [`None`](Network::None), "none" is returned.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::String(string) => string,
            Self::None => "none",
        }
    }

    /// Returns `true` if the network is [`None`].
    ///
    /// [`None`]: Network::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Convert into [`Option<String>`].
    ///
    /// [`Network::String`] converts to [`Option::Some`] and [`Network::None`] to [`Option::None`].
    #[must_use]
    pub fn into_option(self) -> Option<String> {
        match self {
            Self::String(string) => Some(string),
            Self::None => None,
        }
    }
}

impl From<String> for Network {
    fn from(value: String) -> Self {
        Self::parse(value)
    }
}

impl From<&str> for Network {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<Box<str>> for Network {
    fn from(value: Box<str>) -> Self {
        Self::parse(value)
    }
}

impl<'a> From<Cow<'a, str>> for Network {
    fn from(value: Cow<'a, str>) -> Self {
        Self::parse(value)
    }
}

impl AsRef<str> for Network {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Network> for Cow<'static, str> {
    fn from(value: Network) -> Self {
        match value {
            Network::String(string) => string.into(),
            Network::None => Self::Borrowed("none"),
        }
    }
}

impl From<Network> for String {
    fn from(value: Network) -> Self {
        Cow::from(value).into_owned()
    }
}

impl From<Network> for Option<String> {
    fn from(value: Network) -> Self {
        value.into_option()
    }
}
