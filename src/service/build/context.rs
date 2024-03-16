//! Provides [`Context`] for [`Build`](super::Build).

use std::{
    borrow::Cow,
    convert::Infallible,
    ffi::OsString,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

use crate::serde::FromStrVisitor;

/// Path to a directory containing a Dockerfile/Containerfile, or a URL to a git repository.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/build.md#context)
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Context {
    /// An absolute or relative path to the Compose file's parent folder.
    ///
    /// This path must be a directory and must contain a `Dockerfile`/`Containerfile`.
    Path(PathBuf),

    #[allow(rustdoc::bare_urls)]
    /// A URL context.
    ///
    /// Git URLs accept context configuration in their fragment section, separated by a colon (:).
    /// The first part represents the reference that Git checks out, and can be either a branch,
    /// a tag, or a remote reference. The second part represents a subdirectory inside the
    /// repository that is used as a build context.
    ///
    /// For example: "https://github.com/example/example.git#branch_or_tag:subdirectory"
    ///
    /// Other types of contexts can be defined in the `additional_contexts` field of the
    /// long [`Build`](super::Build) syntax by using alternative schemes such as `docker-image://`
    /// or `oci-layout://`.
    Url(Url),
}

impl Context {
    /// Parse [`Context`] from a string.
    ///
    /// If the given string can be parsed as a [`Url`], [`Context::Url`] is returned.
    /// Otherwise, it is converted into a [`PathBuf`] and [`Context::Path`] is returned.
    pub fn parse<T>(context: T) -> Self
    where
        T: AsRef<str> + Into<PathBuf>,
    {
        context
            .as_ref()
            .parse()
            .map_or_else(|_| Self::Path(context.into()), Self::Url)
    }

    /// Returns [`Some`] if a path.
    #[must_use]
    pub fn as_path(&self) -> Option<&PathBuf> {
        if let Self::Path(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns [`Some`] if a URL.
    #[must_use]
    pub fn as_url(&self) -> Option<&Url> {
        if let Self::Url(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if a URL that points to a git repository.
    ///
    /// Requires that the URL has an "http" or "https" scheme, and the path to end in ".git".
    #[must_use]
    pub fn is_git_repo_url(&self) -> bool {
        self.as_url().is_some_and(|url| {
            let scheme = url.scheme();
            (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
                && Path::new(url.path())
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("git"))
        })
    }

    /// If a git repository URL that has a fragment component, returns [`Some`] with a string slice
    /// of the branch or tag specified.
    #[must_use]
    pub fn branch_or_tag(&self) -> Option<&str> {
        if !self.is_git_repo_url() {
            return None;
        }

        // url format is "https://github.com/example/example.git#branch_or_tag:subdirectory"
        let fragment = self.as_url()?.fragment()?;
        fragment
            .split_once(':')
            .unzip()
            .0
            .map_or(Some(fragment), Some)
    }

    /// If a git repository URL that has a fragment component, returns [`Some`] with a string slice
    /// of the subdirectory specified.
    #[must_use]
    pub fn subdirectory(&self) -> Option<&str> {
        if !self.is_git_repo_url() {
            return None;
        }

        // url format is "https://github.com/example/example.git#branch_or_tag:subdirectory"
        self.as_url()?.fragment()?.split_once(':').unzip().1
    }

    /// Convert into a [`String`].
    ///
    /// # Errors
    ///
    /// Returns ownership on error.
    /// Error occurs if a path which does not contain valid Unicode data.
    pub fn into_string(self) -> Result<String, Self> {
        match self {
            Self::Path(path) => OsString::from(path)
                .into_string()
                .map_err(|path| Self::Path(path.into())),
            Self::Url(url) => Ok(url.into()),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::Path(".".into())
    }
}

impl From<&str> for Context {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for Context {
    fn from(value: String) -> Self {
        Self::parse(value)
    }
}

impl From<Box<str>> for Context {
    fn from(value: Box<str>) -> Self {
        Self::parse(value.into_string())
    }
}

impl From<Cow<'_, str>> for Context {
    fn from(value: Cow<str>) -> Self {
        value
            .parse()
            .map_or_else(|_| Self::Path(value.into_owned().into()), Self::Url)
    }
}

impl From<&Path> for Context {
    fn from(value: &Path) -> Self {
        Self::Path(value.to_owned())
    }
}

impl From<Cow<'_, Path>> for Context {
    fn from(value: Cow<Path>) -> Self {
        Self::Path(value.into_owned())
    }
}

impl From<PathBuf> for Context {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<Url> for Context {
    fn from(value: Url) -> Self {
        Self::Url(value)
    }
}

impl FromStr for Context {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<'de> Deserialize<'de> for Context {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromStrVisitor::new("a string representing a path or URL").deserialize(deserializer)
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Path(path) => path.display().fmt(f),
            Self::Url(url) => url.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_components() {
        let context = Context::Url(
            "https://github.com/example/example.git#branch_or_tag:subdirectory"
                .parse()
                .unwrap(),
        );
        assert_eq!(context.branch_or_tag(), Some("branch_or_tag"));
        assert_eq!(context.subdirectory(), Some("subdirectory"));
    }
}
