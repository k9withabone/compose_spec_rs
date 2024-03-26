//! Provides [`Network`] for the `network` field of the long [`Build`](super::Build) syntax.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};

use crate::{impl_from_str, Identifier, InvalidIdentifierError};

/// Network containers connect to during [`Build`](super::Build) for `RUN` instructions.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#network)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// Network to connect to during build.
    Identifier(Identifier),

    /// Disable networking during build.
    None,
}

impl Network {
    /// [`Self::None`] string value.
    const NONE: &'static str = "none";

    /// Parse [`Network`] from a string.
    ///
    /// "none" converts to [`Network::None`].
    ///
    /// # Errors
    ///
    /// Returns an error if the network is not a valid [`Identifier`]
    pub fn parse<T>(network: T) -> Result<Self, T::Error>
    where
        T: AsRef<str> + TryInto<Identifier>,
    {
        if network.as_ref() == Self::NONE {
            Ok(Self::None)
        } else {
            network.try_into().map(Self::Identifier)
        }
    }

    /// String slice of the [`Network`].
    ///
    /// If [`None`](Network::None), "none" is returned.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Identifier(network) => network.as_str(),
            Self::None => Self::NONE,
        }
    }

    /// Returns `true` if the network is [`None`].
    ///
    /// [`None`]: Network::None
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Convert into [`Option<String>`].
    ///
    /// [`Network::Identifier`] converts into [`Option::Some`] and [`Network::None`] into
    /// [`Option::None`].
    #[must_use]
    pub fn into_option(self) -> Option<Identifier> {
        match self {
            Self::Identifier(network) => Some(network),
            Self::None => None,
        }
    }
}

impl_from_str!(Network => InvalidIdentifierError);

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
            Network::Identifier(network) => Self::Owned(network.into()),
            Network::None => Self::Borrowed(Network::NONE),
        }
    }
}

impl From<Network> for String {
    fn from(value: Network) -> Self {
        Cow::from(value).into_owned()
    }
}
