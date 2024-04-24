//! Provides [`Volumes`] for the `volumes` field of [`Service`](super::Service).
//!
//! Each volume may be in [short](ShortVolume) or [long](Mount) syntax.
//!
//! [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#volumes)

pub mod mount;

use std::{
    borrow::{Borrow, Cow},
    fmt::{self, Display, Formatter, Write},
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, DeserializeTryFromString, SerializeDisplay};
use indexmap::IndexSet;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;

use crate::{impl_try_from, Identifier, InvalidIdentifierError, ShortOrLong};

pub use self::mount::Mount;
use self::mount::{Bind, BindOptions, Common, Volume};

/// [`Volume`](crate::Volume)s to mount within a [`Service`](super::Service) container.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#volumes)
pub type Volumes = IndexSet<ShortOrLong<ShortVolume, Mount>>;

/// Convert [`Volumes`] into an [`Iterator`] of volumes in the [`ShortVolume`] syntax.
///
/// If a volume [`Mount`] cannot be represented in the [`ShortVolume`] syntax, it is returned in the
/// [`Err`] variant of the item.
pub fn into_short_iter(volumes: Volumes) -> impl Iterator<Item = Result<ShortVolume, Mount>> {
    volumes.into_iter().map(|volume| match volume {
        ShortOrLong::Short(volume) => Ok(volume),
        ShortOrLong::Long(volume) => volume.into_short(),
    })
}

/// Convert [`Volumes`] into an [`Iterator`] of long syntax volume [`Mount`]s.
pub fn into_long_iter(volumes: Volumes) -> impl Iterator<Item = Mount> {
    volumes.into_iter().map(Into::into)
}

/// Short [`Service`](super::Service) container volume syntax.
///
/// (De)serializes from/to a string in the format `[{source}:]{container_path}[:{options}]`, where
/// `options` is a comma (,) separated list of bind mount options:
///
/// - `rw`: Read and write access, the default.
/// - `ro`: Read-only access, sets `options.read_only` to `true`.
/// - `z`: Shared SELinux relabeling, sets `options.selinux` to `Some(SELinux::Shared)`.
/// - `Z`: Private SELinux relabeling, sets `options.selinux` to `Some(SELinux::Private)`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#short-syntax-5)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(expecting = "a string in the format \"[{source}:]{container_path}[:{options}]\"")]
pub struct ShortVolume {
    /// Path within the container where the volume is mounted.
    pub container_path: AbsolutePath,

    /// Volume options, including an optional [`Source`].
    ///
    /// If [`None`] an anonymous volume is used.
    pub options: Option<ShortOptions>,
}

impl ShortVolume {
    /// Create a new [`ShortVolume`].
    #[must_use]
    pub const fn new(container_path: AbsolutePath) -> Self {
        Self {
            container_path,
            options: None,
        }
    }

    /// Convert the short volume syntax into the long volume [`Mount`] syntax.
    #[must_use]
    pub fn into_long(self) -> Mount {
        let Self {
            container_path: target,
            options,
        } = self;

        if let Some(ShortOptions {
            source,
            read_only,
            selinux,
        }) = options
        {
            let common = Common {
                read_only,
                ..target.into()
            };
            match source {
                Source::HostPath(source) => Mount::Bind(Bind {
                    source,
                    bind: Some(BindOptions {
                        create_host_path: true,
                        selinux,
                        ..BindOptions::default()
                    }),
                    common,
                }),
                Source::Volume(source) => Mount::Volume(Volume {
                    source: Some(source),
                    volume: None,
                    common,
                }),
            }
        } else {
            Mount::Volume(Common::new(target).into())
        }
    }
}

impl From<AbsolutePath> for ShortVolume {
    fn from(container_path: AbsolutePath) -> Self {
        Self::new(container_path)
    }
}

impl FromStr for ShortVolume {
    type Err = ParseShortVolumeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "[{source}:]{container_path}[:{options}]"
        let mut split = s.splitn(3, ':');
        let source_or_container = split.next().expect("split has at least one element");

        let Some(container_path) = split.next() else {
            let container_path = source_or_container;
            return parse_container_path(container_path).map(Self::new);
        };

        let source = source_or_container.parse()?;
        let container_path = parse_container_path(container_path)?;

        let Some(options) = split.next() else {
            return Ok(Self {
                container_path,
                options: Some(ShortOptions::new(source)),
            });
        };

        let mut read_only = None;
        let mut selinux = None;
        for option in options.split(',') {
            match option {
                "rw" => match read_only {
                    None => read_only = Some(false),
                    Some(true) => return Err(ParseShortVolumeError::ReadWriteAndReadOnly),
                    Some(false) => return Err(ParseShortVolumeError::DuplicateOption("rw")),
                },
                "ro" => match read_only {
                    None => read_only = Some(true),
                    Some(false) => return Err(ParseShortVolumeError::ReadWriteAndReadOnly),
                    Some(true) => return Err(ParseShortVolumeError::DuplicateOption("ro")),
                },
                "z" => match selinux {
                    None => selinux = Some(SELinux::Shared),
                    Some(SELinux::Private) => {
                        return Err(ParseShortVolumeError::SELinuxSharedAndPrivate);
                    }
                    Some(SELinux::Shared) => {
                        return Err(ParseShortVolumeError::DuplicateOption("z"));
                    }
                },
                "Z" => match selinux {
                    None => selinux = Some(SELinux::Private),
                    Some(SELinux::Shared) => {
                        return Err(ParseShortVolumeError::SELinuxSharedAndPrivate);
                    }
                    Some(SELinux::Private) => {
                        return Err(ParseShortVolumeError::DuplicateOption("Z"));
                    }
                },
                unknown => return Err(ParseShortVolumeError::UnknownOption(unknown.to_owned())),
            }
        }

        Ok(Self {
            container_path,
            options: Some(ShortOptions {
                source,
                read_only: read_only.unwrap_or_default(),
                selinux,
            }),
        })
    }
}

/// Parse `container_path` into an [`AbsolutePath`].
///
/// # Errors
///
/// Returns an error if the container path is not an absolute path.
fn parse_container_path(container_path: &str) -> Result<AbsolutePath, ParseShortVolumeError> {
    #[allow(clippy::map_err_ignore)]
    container_path
        .parse()
        .map_err(|_| ParseShortVolumeError::AbsoluteContainerPath(container_path.to_owned()))
}

/// Error returned when [parsing](ShortVolume::from_str()) [`ShortVolume`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseShortVolumeError {
    /// Error parsing [`Source`].
    #[error("error parsing volume source")]
    Source(#[from] ParseSourceError),

    /// Container path was not an [`AbsolutePath`].
    #[error("volume container path `{0}` is not absolute")]
    AbsoluteContainerPath(String),

    /// `rw` and `ro` set in options.
    #[error("cannot set both `rw` and `ro` in volume options")]
    ReadWriteAndReadOnly,

    /// `z` and `Z` set in options.
    #[error("cannot set both `z` and `Z` in volume options")]
    SELinuxSharedAndPrivate,

    /// Option set multiple times.
    #[error("volume option `{0}` set multiple times")]
    DuplicateOption(&'static str),

    /// Unknown volume option.
    #[error("unknown volume option `{0}`")]
    UnknownOption(String),
}

impl Display for ShortVolume {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            container_path,
            options,
        } = self;
        let container_path = container_path.as_path().display();

        // Format is "[{source}:]{container_path}[:{options}]"

        if let Some(ShortOptions {
            ref source,
            read_only,
            selinux,
        }) = *options
        {
            write!(f, "{source}:{container_path}")?;

            if read_only || selinux.is_some() {
                f.write_char(':')?;
                if read_only {
                    f.write_str("ro")?;
                    if selinux.is_some() {
                        f.write_char(',')?;
                    }
                }
                if let Some(selinux) = selinux {
                    selinux.fmt(f)?;
                }
            }

            Ok(())
        } else {
            container_path.fmt(f)
        }
    }
}

/// An [absolute](Path::is_absolute()) path.
#[derive(
    Serialize, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    /// Create an [`AbsolutePath`].
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not absolute.
    pub fn new<T>(path: T) -> Result<Self, AbsolutePathError>
    where
        T: AsRef<Path> + Into<PathBuf>,
    {
        if path.as_ref().is_absolute() {
            Ok(Self(path.into()))
        } else {
            Err(AbsolutePathError)
        }
    }

    /// Truncates `self` to [`self.as_path().parent()`].
    ///
    /// Returns `false` and does nothing if [`self.as_path().parent()`] is [`None`].
    ///
    /// [`self.as_path().parent()`]: Path::parent()
    pub fn pop(&mut self) -> bool {
        self.0.pop()
    }
}

/// Error returned when creating an [`AbsolutePath`].
///
/// Occurs if the path is not [absolute](Path::is_absolute()).
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("path is not absolute")]
pub struct AbsolutePathError;

/// Implement methods and traits for a [`PathBuf`] newtype.
///
/// The type must have a `new()` function which returns a [`Result<Self, Error>`].
macro_rules! path_impls {
    ($Ty:ident => $Error:ty) => {
        impl $Ty {
            /// Coerces to a [`Path`] slice.
            #[must_use]
            pub fn as_path(&self) -> &Path {
                self.0.as_path()
            }

            /// Extend `self` with `path`.
            ///
            /// If `path` is absolute, it replaces the current path.
            pub fn push<P: AsRef<Path>>(&mut self, path: P) {
                self.0.push(path);
            }

            /// Return a reference to the inner value.
            #[must_use]
            pub const fn as_inner(&self) -> &PathBuf {
                &self.0
            }

            /// Return the inner value.
            #[must_use]
            pub fn into_inner(self) -> PathBuf {
                self.0
            }
        }

        impl_try_from! {
            $Ty::new -> $Error,
            PathBuf, Box<Path>, &Path, Cow<'_, Path>, String, &str,
        }

        impl FromStr for $Ty {
            type Err = $Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.try_into()
            }
        }

        impl AsRef<PathBuf> for $Ty {
            fn as_ref(&self) -> &PathBuf {
                self.as_inner()
            }
        }

        impl AsRef<Path> for $Ty {
            fn as_ref(&self) -> &Path {
                self.as_path()
            }
        }

        impl Borrow<Path> for $Ty {
            fn borrow(&self) -> &Path {
                self.as_path()
            }
        }

        impl PartialEq<Path> for $Ty {
            fn eq(&self, other: &Path) -> bool {
                self.0.eq(other)
            }
        }

        impl From<$Ty> for PathBuf {
            fn from(value: $Ty) -> Self {
                value.into_inner()
            }
        }

        impl From<$Ty> for Box<Path> {
            fn from(value: $Ty) -> Self {
                value.into_inner().into()
            }
        }
    };
}

path_impls!(AbsolutePath => AbsolutePathError);

/// Options for the [`ShortVolume`] syntax.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#short-syntax-5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortOptions {
    /// Source of the container volume.
    pub source: Source,

    /// Whether the volume is set as read-only.
    pub read_only: bool,

    /// Whether to use SELinux relabeling on the volume's contents.
    pub selinux: Option<SELinux>,
}

impl ShortOptions {
    /// Create a new [`ShortOptions`].
    #[must_use]
    pub const fn new(source: Source) -> Self {
        Self {
            source,
            read_only: false,
            selinux: None,
        }
    }
}

impl From<Source> for ShortOptions {
    fn from(source: Source) -> Self {
        Self::new(source)
    }
}

/// Container volume source in the [`ShortVolume`] syntax.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#short-syntax-5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// Host path for using a bind mount.
    ///
    /// Relative paths are resolved from the parent directory of the [`Compose`](crate::Compose)
    /// file.
    HostPath(HostPath),

    /// Named [`Volume`](crate::Volume).
    Volume(Identifier),
}

impl Source {
    /// Parse [`Source`] from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the source is not a host path and the conversion into an [`Identifier`]
    /// fails.
    pub fn parse<T>(source: T) -> Result<Self, ParseSourceError>
    where
        T: AsRef<str> + TryInto<HostPath> + TryInto<Identifier>,
        <T as TryInto<HostPath>>::Error: Into<ParseSourceError>,
        <T as TryInto<Identifier>>::Error: Into<ParseSourceError>,
    {
        if source.as_ref().starts_with('.') || Path::new(source.as_ref()).is_absolute() {
            source.try_into().map(Self::HostPath).map_err(Into::into)
        } else {
            source.try_into().map(Self::Volume).map_err(Into::into)
        }
    }
}

/// Error returned when [parsing](Source::parse()) a [`Source`] from a string.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSourceError {
    /// Error parsing [`HostPath`].
    #[error("error parsing host path")]
    HostPath(#[from] HostPathError),

    /// Error parsing [`Identifier`].
    #[error("error parsing volume identifier")]
    Identifier(#[from] InvalidIdentifierError),
}

impl FromStr for Source {
    type Err = ParseSourceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Source {
    type Error = ParseSourceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for Source {
    type Error = ParseSourceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<HostPath> for Source {
    fn from(value: HostPath) -> Self {
        Self::HostPath(value)
    }
}

impl From<AbsolutePath> for Source {
    fn from(value: AbsolutePath) -> Self {
        HostPath::from(value).into()
    }
}

impl TryFrom<PathBuf> for Source {
    type Error = HostPathError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        HostPath::try_from(value).map(Into::into)
    }
}

impl TryFrom<Box<Path>> for Source {
    type Error = HostPathError;

    fn try_from(value: Box<Path>) -> Result<Self, Self::Error> {
        HostPath::try_from(value).map(Into::into)
    }
}

impl TryFrom<&Path> for Source {
    type Error = HostPathError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        HostPath::try_from(value).map(Into::into)
    }
}

impl TryFrom<Cow<'_, Path>> for Source {
    type Error = HostPathError;

    fn try_from(value: Cow<'_, Path>) -> Result<Self, Self::Error> {
        HostPath::try_from(value).map(Into::into)
    }
}

impl From<Identifier> for Source {
    fn from(value: Identifier) -> Self {
        Self::Volume(value)
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::HostPath(source) => source.as_path().display().fmt(f),
            Self::Volume(source) => source.fmt(f),
        }
    }
}

/// A path on the host.
///
/// Host paths must start with `.` or `..`,  or be [absolute](Path::is_absolute()).
#[derive(
    Serialize, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct HostPath(PathBuf);

impl HostPath {
    /// Create a [`HostPath`].
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not start with `.` or `..`,  or is not
    /// [absolute](Path::is_absolute()).
    pub fn new<T>(path: T) -> Result<Self, HostPathError>
    where
        T: AsRef<Path> + Into<PathBuf>,
    {
        if path.as_ref().is_absolute()
            || path.as_ref().components().next().is_some_and(|component| {
                matches!(component, Component::CurDir | Component::ParentDir)
            })
        {
            Ok(Self(path.into()))
        } else {
            Err(HostPathError)
        }
    }

    /// Truncates `self` to [`self.as_path().parent()`].
    ///
    /// Returns `false` and does nothing if [`self.as_path().parent()`] is [`None`] or is empty.
    ///
    /// [`self.as_path().parent()`]: Path::parent()
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use compose_spec::service::volumes::HostPath;
    ///
    /// let mut path = HostPath::new("./hello/world").unwrap();
    ///
    /// assert!(path.pop());
    /// assert_eq!(&path, Path::new("./hello"));
    ///
    /// assert!(path.pop());
    /// assert_eq!(&path, Path::new("."));
    ///
    /// assert!(!path.pop());
    /// assert_eq!(&path, Path::new("."));
    /// ```
    pub fn pop(&mut self) -> bool {
        !self
            .0
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty())
            && self.0.pop()
    }
}

/// Error returned when creating a [`HostPath`].
///
/// Occurs if the path does not start with '.' or '..', or is not [absolute](Path::is_absolute()).
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("volume host paths must start with `.` or `..`, or be absolute")]
pub struct HostPathError;

path_impls!(HostPath => HostPathError);

impl From<AbsolutePath> for HostPath {
    fn from(value: AbsolutePath) -> Self {
        Self(value.0)
    }
}

/// SELinux relabeling options.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#volumes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SELinux {
    /// `z`, share the bind mount among multiple containers.
    Shared,
    /// `Z`, bind mount is private and only accessible to one container.
    Private,
}

impl SELinux {
    /// SELinux relabeling option as a character.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::Shared => 'z',
            Self::Private => 'Z',
        }
    }

    /// SELinux relabeling option as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "z",
            Self::Private => "Z",
        }
    }
}

impl From<SELinux> for char {
    fn from(value: SELinux) -> Self {
        value.as_char()
    }
}

impl AsRef<str> for SELinux {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for SELinux {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.as_char())
    }
}

impl Serialize for SELinux {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_char(self.as_char())
    }
}

impl<'de> Deserialize<'de> for SELinux {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match char::deserialize(deserializer)? {
            'z' => Ok(Self::Shared),
            'Z' => Ok(Self::Private),
            char => Err(de::Error::invalid_value(
                Unexpected::Char(char),
                &"'z' or 'Z'",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::{
        arbitrary::{any, Arbitrary},
        option, prop_assert_eq, prop_compose, prop_oneof, proptest,
        strategy::{BoxedStrategy, Just, Strategy},
    };

    use crate::service::tests::path_no_colon;

    use super::*;

    impl Arbitrary for AbsolutePath {
        type Parameters = ();

        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            path_no_colon()
                .prop_map(|path| {
                    if path.is_absolute() {
                        Self(path)
                    } else {
                        Self(Path::new("/").join(path))
                    }
                })
                .boxed()
        }
    }

    mod short_volume {
        use super::*;

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<ShortVolume>();
            }

            #[test]
            fn round_trip(volume in short_volume()) {
                prop_assert_eq!(&volume, &volume.to_string().parse()?);
            }
        }
    }

    prop_compose! {
        fn short_volume()(
            container_path: AbsolutePath,
            options in option::of(short_options()),
        ) -> ShortVolume {
            ShortVolume {
                container_path,
                options,
            }
        }
    }

    prop_compose! {
        fn short_options()(
            source in source(),
            read_only: bool,
            selinux in option::of(selinux()),
        ) -> ShortOptions {
            ShortOptions {
                source,
                read_only,
                selinux
            }
        }
    }

    fn source() -> impl Strategy<Value = Source> {
        prop_oneof![
            host_path().prop_map_into(),
            any::<Identifier>().prop_map_into(),
        ]
    }

    fn host_path() -> impl Strategy<Value = HostPath> {
        path_no_colon().prop_flat_map(|path| {
            prop_oneof![Just("/"), Just("."), Just("..")]
                .prop_map(move |prefix| HostPath(Path::new(prefix).join(&path)))
        })
    }

    fn selinux() -> impl Strategy<Value = SELinux> {
        prop_oneof![Just(SELinux::Shared), Just(SELinux::Private)]
    }
}
