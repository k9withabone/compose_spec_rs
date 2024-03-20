use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

/// [Cgroup](https://man7.org/linux/man-pages/man7/cgroups.7.html) namespace to join.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cgroup)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cgroup {
    /// Run the container in the Container runtime cgroup namespace.
    Host,

    /// Run the container in its own private cgroup namespace.
    Private,
}

impl Cgroup {
    /// [`Cgroup`] option as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Private => "private",
        }
    }
}

impl AsRef<str> for Cgroup {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Cgroup {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Cgroup> for &'static str {
    fn from(value: Cgroup) -> Self {
        value.as_str()
    }
}

impl FromStr for Cgroup {
    type Err = ParseCgroupError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "host" => Ok(Self::Host),
            "private" => Ok(Self::Private),
            s => Err(ParseCgroupError(s.to_owned())),
        }
    }
}

impl TryFrom<&str> for Cgroup {
    type Error = ParseCgroupError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`Cgroup`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("invalid cgroup option `{0}`, cgroup must be `host` or `private`")]
pub struct ParseCgroupError(String);
