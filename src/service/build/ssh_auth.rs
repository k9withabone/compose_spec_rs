//! Provides [`SshAuth`] for the `ssh` field of the long [`Build`](super::Build) syntax.

use std::{
    fmt::{self, Display, Formatter},
    path::Path,
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

/// SSH authentication for use by the image builder.
///
/// (De)serializes from/to "default" or "{id}={path}".
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#ssh)
#[derive(SerializeDisplay, DeserializeFromStr, Default, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(expecting = r#""default" or "{id}={path}" string"#)]
pub enum SshAuth {
    /// Let the builder connect to the ssh-agent.
    #[default]
    Default,

    /// SSH authentication ID and associated path to a PEM file or ssh-agent socket.
    Id(Id),
}

impl SshAuth {
    /// Returns [`Some`] if [`SshAuth::Id`].
    #[must_use]
    pub fn as_id(&self) -> Option<&Id> {
        if let Self::Id(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// The ID of the SSH authentication.
    ///
    /// Returns [`Some`] if [`SshAuth::Id`].
    pub fn id(&self) -> Option<&str> {
        self.as_id().map(Id::id)
    }

    /// The path of the PEM file or ssh-agent socket.
    ///
    /// Returns [`Some`] if [`SshAuth::Id`].
    pub fn path(&self) -> Option<&Path> {
        self.as_id().map(Id::path)
    }
}

impl FromStr for SshAuth {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
            s => Ok(Self::Id(s.parse()?)),
        }
    }
}

impl Display for SshAuth {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Default => f.write_str("default"),
            Self::Id(id) => Display::fmt(id, f),
        }
    }
}

/// SSH authentication ID for use by the image builder.
///
/// The [`Display`] and [`FromStr`] implementations use the format "{id}={path}".
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#ssh)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id {
    /// SSH authentication ID.
    id: Box<str>,
    /// Path to a PEM file or ssh-agent socket.
    path: Box<Path>,
}

impl Id {
    /// Create a new SSH authentication ID.
    ///
    /// `path` should be a path to a PEM file or ssh-agent socket.
    ///
    /// # Errors
    ///
    /// Returns an error is either the `id` or `path` are empty.
    pub fn new<I, P>(id: I, path: P) -> Result<Self, IdError>
    where
        I: AsRef<str>,
        P: AsRef<Path>,
    {
        let id = id.as_ref();
        let path = path.as_ref();

        if id.is_empty() {
            Err(IdError::EmptyId)
        } else if path.as_os_str().is_empty() {
            Err(IdError::MissingPath)
        } else {
            Ok(Self {
                id: id.into(),
                path: path.into(),
            })
        }
    }

    /// The ID of the SSH authentication.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The path of the PEM file or ssh-agent socket.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl FromStr for Id {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "{id}={path}".
        let (id, path) = s.split_once('=').ok_or(IdError::MissingPath)?;
        Self::new(id, path)
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { id, path } = self;
        write!(f, "{id}={}", path.display())
    }
}

/// Error returned when creating an [`Id`].
#[derive(Error, Debug)]
pub enum IdError {
    /// Given `id` was empty
    #[error("ssh auth ID cannot be empty")]
    EmptyId,

    /// Given `path` was empty, or, when parsing, the '=' was missing.
    #[error("non-default ssh auth requires a path to a PEM file or to the ssh-agent socket")]
    MissingPath,
}
