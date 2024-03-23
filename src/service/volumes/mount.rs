//! Provides the long volume [`Mount`] syntax for [`Service`](crate::Service)
//! [`Volumes`](super::Volumes).
//!
//! [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)

use std::{
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Not,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{service::ByteValue, Extensions, Identifier};

use super::{AbsolutePath, HostPath, SELinux, ShortOptions, ShortVolume};

/// Long volume mount syntax for a [`Service`](crate::Service) container.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Mount {
    /// Named or anonymous volume.
    Volume(Volume),

    /// Bind mount from host to container.
    Bind(Bind),

    /// Temporary file system.
    Tmpfs(Tmpfs),

    /// Named pipe.
    #[serde(rename = "npipe")]
    NamedPipe(NamedPipe),

    /// Cluster.
    Cluster(Cluster),
}

impl Mount {
    /// Type of the mount as a static string slice.
    ///
    /// Corresponds to the `type` field in the
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
    /// for the long volume syntax.
    #[doc(alias = "type")]
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Volume(_) => "volume",
            Self::Bind(_) => "bind",
            Self::Tmpfs(_) => "tmpfs",
            Self::NamedPipe(_) => "npipe",
            Self::Cluster(_) => "cluster",
        }
    }

    /// Returns `true` if the mount is a [`Volume`].
    ///
    /// [`Volume`]: Mount::Volume
    #[must_use]
    pub fn is_volume(&self) -> bool {
        matches!(self, Self::Volume(..))
    }

    /// Returns [`Some`] if the mount is a [`Volume`].
    ///
    /// [`Volume`]: Mount::Volume
    #[must_use]
    pub fn as_volume(&self) -> Option<&Volume> {
        if let Self::Volume(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the mount is a [`Bind`] mount.
    ///
    /// [`Bind`]: Mount::Bind
    #[must_use]
    pub fn is_bind(&self) -> bool {
        matches!(self, Self::Bind(..))
    }

    /// Returns [`Some`] if the mount is [`Bind`] mount.
    ///
    /// [`Bind`]: Mount::Bind
    #[must_use]
    pub fn as_bind(&self) -> Option<&Bind> {
        if let Self::Bind(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the mount is a [`Tmpfs`].
    ///
    /// [`Tmpfs`]: Mount::Tmpfs
    #[must_use]
    pub fn is_tmpfs(&self) -> bool {
        matches!(self, Self::Tmpfs(..))
    }

    /// Returns [`Some`] if the mount is a [`Tmpfs`].
    ///
    /// [`Tmpfs`]: Mount::Tmpfs
    #[must_use]
    pub fn as_tmpfs(&self) -> Option<&Tmpfs> {
        if let Self::Tmpfs(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the mount is a [`NamedPipe`].
    ///
    /// [`NamedPipe`]: Mount::NamedPipe
    #[must_use]
    pub fn is_named_pipe(&self) -> bool {
        matches!(self, Self::NamedPipe(..))
    }

    /// Returns [`Some`] if the mount is a [`NamedPipe`].
    ///
    /// [`NamedPipe`]: Mount::NamedPipe
    #[must_use]
    pub fn as_named_pipe(&self) -> Option<&NamedPipe> {
        if let Self::NamedPipe(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the mount is a [`Cluster`].
    ///
    /// [`Cluster`]: Mount::Cluster
    #[must_use]
    pub fn is_cluster(&self) -> bool {
        matches!(self, Self::Cluster(..))
    }

    /// Returns [`Some`] if the mount is a [`Cluster`].
    ///
    /// [`Cluster`]: Mount::Cluster
    #[must_use]
    pub fn as_cluster(&self) -> Option<&Cluster> {
        if let Self::Cluster(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// [`Common`] mount options.
    #[must_use]
    pub fn common(&self) -> &Common {
        match self {
            Self::Volume(mount) => &mount.common,
            Self::Bind(mount) => &mount.common,
            Self::Tmpfs(mount) => &mount.common,
            Self::NamedPipe(mount) => &mount.common,
            Self::Cluster(mount) => &mount.common,
        }
    }

    /// Source of the mount, if it has one, converted to a string.
    #[must_use]
    pub fn source_to_string(&self) -> Option<String> {
        match self {
            Self::Volume(mount) => mount.source.as_ref().map(Identifier::to_string),
            Self::Bind(mount) => Some(mount.source.as_path().display().to_string()),
            Self::Tmpfs(_) => None,
            Self::NamedPipe(mount) => Some(mount.source.as_path().display().to_string()),
            Self::Cluster(mount) => Some(mount.source.clone()),
        }
    }

    /// Convert into the [`ShortVolume`] syntax if possible.
    ///
    /// # Errors
    ///
    /// Returns ownership if this long syntax cannot be represented as the short syntax.
    pub fn into_short(self) -> Result<ShortVolume, Self> {
        match self {
            Self::Volume(Volume {
                source,
                volume: None,
                common,
            }) if common.is_short_compatible() => Ok(ShortVolume {
                container_path: common.target,
                options: source.map(|source| ShortOptions {
                    source: source.into(),
                    read_only: common.read_only,
                    selinux: None,
                }),
            }),
            Self::Bind(Bind {
                source,
                bind,
                common,
            }) if bind.as_ref().map_or(true, BindOptions::is_short_compatible)
                && common.is_short_compatible() =>
            {
                Ok(ShortVolume {
                    container_path: common.target,
                    options: Some(ShortOptions {
                        source: source.into(),
                        read_only: common.read_only,
                        selinux: bind.and_then(|bind| bind.selinux),
                    }),
                })
            }
            _ => Err(self),
        }
    }
}

impl From<ShortVolume> for Mount {
    fn from(value: ShortVolume) -> Self {
        value.into_long()
    }
}

impl From<Volume> for Mount {
    fn from(value: Volume) -> Self {
        Self::Volume(value)
    }
}

impl From<Bind> for Mount {
    fn from(value: Bind) -> Self {
        Self::Bind(value)
    }
}

impl From<Tmpfs> for Mount {
    fn from(value: Tmpfs) -> Self {
        Self::Tmpfs(value)
    }
}

impl From<NamedPipe> for Mount {
    fn from(value: NamedPipe) -> Self {
        Self::NamedPipe(value)
    }
}

impl From<Cluster> for Mount {
    fn from(value: Cluster) -> Self {
        Self::Cluster(value)
    }
}

/// Volume [`Mount`] type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Volume {
    /// Name of the [`Volume`](crate::Volume) to mount.
    ///
    /// If [`None`] an anonymous volume is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Identifier>,

    /// Additional volume options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<VolumeOptions>,

    /// Common [`Mount`] options.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub common: Common,
}

impl Volume {
    /// Create a [`Volume`] [`Mount`] from [`Common`] mount options.
    #[must_use]
    pub fn new(common: Common) -> Self {
        Self {
            source: None,
            volume: None,
            common,
        }
    }
}

impl From<Common> for Volume {
    fn from(common: Common) -> Self {
        Self::new(common)
    }
}

/// Additional [`Volume`] [`Mount`] options.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq)]
pub struct VolumeOptions {
    /// Whether to disable copying of data from a container to the volume when it is created.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub nocopy: bool,

    /// Path inside the volume to mount instead of the volume root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<PathBuf>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl PartialEq for VolumeOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            nocopy,
            subpath,
            extensions,
        } = self;

        *nocopy == other.nocopy
            && *subpath == other.subpath
            && extensions.as_slice() == other.extensions.as_slice()
    }
}

impl Hash for VolumeOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            nocopy,
            subpath,
            extensions,
        } = self;

        nocopy.hash(state);
        subpath.hash(state);
        extensions.as_slice().hash(state);
    }
}

/// Bind [`Mount`] type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bind {
    /// Path on the host for the bind mount.
    pub source: HostPath,

    /// Additional bind mount options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<BindOptions>,

    /// Common [`Mount`] options.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub common: Common,
}

impl Bind {
    /// Create a [`Bind`] [`Mount`] from a `source` and [`Common`] mount options.
    #[must_use]
    pub fn new(source: HostPath, common: Common) -> Self {
        Self {
            source,
            bind: None,
            common,
        }
    }
}

impl From<(HostPath, Common)> for Bind {
    fn from((source, common): (HostPath, Common)) -> Self {
        Self::new(source, common)
    }
}

/// Additional [`Bind`] [`Mount`] options.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq)]
pub struct BindOptions {
    /// Propagation mode used for the bind mount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propagation: Option<BindPropagation>,

    /// Whether to create a directory at the source path on the host if it does not exist.
    ///
    /// Automatically implied by the [`ShortVolume`] syntax.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub create_host_path: bool,

    /// Whether to use SELinux relabeling on the `source`'s contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selinux: Option<SELinux>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl From<Option<BindPropagation>> for BindOptions {
    fn from(propagation: Option<BindPropagation>) -> Self {
        Self {
            propagation,
            ..Self::default()
        }
    }
}

impl From<BindPropagation> for BindOptions {
    fn from(propagation: BindPropagation) -> Self {
        Some(propagation).into()
    }
}

impl From<Option<SELinux>> for BindOptions {
    fn from(selinux: Option<SELinux>) -> Self {
        Self {
            selinux,
            ..Self::default()
        }
    }
}

impl From<SELinux> for BindOptions {
    fn from(selinux: SELinux) -> Self {
        Some(selinux).into()
    }
}

impl BindOptions {
    /// Returns `true` if these bind [`Mount`] options are compatible with the [`ShortVolume`]
    /// syntax.
    #[must_use]
    fn is_short_compatible(&self) -> bool {
        let Self {
            propagation,
            create_host_path,
            selinux: _,
            extensions,
        } = self;

        propagation.is_none() && *create_host_path && extensions.is_empty()
    }
}

impl PartialEq for BindOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            propagation,
            create_host_path,
            selinux,
            extensions,
        } = self;

        *propagation == other.propagation
            && *create_host_path == other.create_host_path
            && *selinux == other.selinux
            && extensions.as_slice() == other.extensions.as_slice()
    }
}

impl Hash for BindOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            propagation,
            create_host_path,
            selinux,
            extensions,
        } = self;

        propagation.hash(state);
        create_host_path.hash(state);
        selinux.hash(state);
        extensions.as_slice().hash(state);
    }
}

/// Types of [`Bind`] [`Mount`] propagation.
///
/// See [**mount**(2)](https://man7.org/linux/man-pages/man2/mount.2.html),
/// [**mount**(8)](https://man7.org/linux/man-pages/man8/mount.8.html),
/// and [**mount_namespaces**(7)](https://man7.org/linux/man-pages/man7/mount_namespaces.7.html#SHARED_SUBTREES).
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum BindPropagation {
    /// Private mount propagation.
    Private,
    /// Shared mount propagation.
    Shared,
    /// Slave mount propagation.
    Slave,
    /// Unbindable mount propagation.
    Unbindable,
    /// Recursive private mount propagation.
    #[default]
    RPrivate,
    /// Recursive shared mount propagation.
    RShared,
    /// Recursive slave mount propagation.
    RSlave,
    /// Recursive unbindable mount propagation.
    RUnbindable,
}

impl BindPropagation {
    /// Bind propagation as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Shared => "shared",
            Self::Slave => "slave",
            Self::Unbindable => "unbindable",
            Self::RPrivate => "rprivate",
            Self::RShared => "rshared",
            Self::RSlave => "rslave",
            Self::RUnbindable => "runbindable",
        }
    }
}

impl AsRef<str> for BindPropagation {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for BindPropagation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Temporary file system [`Mount`] type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tmpfs {
    /// Additional tmpfs options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tmpfs: Option<TmpfsOptions>,

    /// Common [`Mount`] options.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub common: Common,
}

impl Tmpfs {
    /// Create a [`Tmpfs`] [`Mount`] from [`Common`] mount options.
    #[must_use]
    pub fn new(common: Common) -> Self {
        Self {
            tmpfs: None,
            common,
        }
    }
}

impl From<Common> for Tmpfs {
    fn from(common: Common) -> Self {
        Self::new(common)
    }
}

/// Additional [`Tmpfs`] [`Mount`] options.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq)]
pub struct TmpfsOptions {
    /// Size of the tmpfs mount in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<ByteValue>,

    /// File mode for the tmpfs mount as Unix permission bits.
    ///
    /// Note that, when deserializing with [`serde_yaml`], octal numbers must start with `0o`, e.g.
    /// `0o555`, otherwise, they are interpreted as decimal numbers. Unfortunately, for
    /// serialization, there is no way to specify that a number should be serialized in octal form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl PartialEq for TmpfsOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            size,
            mode,
            extensions,
        } = self;

        *size == other.size
            && *mode == other.mode
            && extensions.as_slice() == other.extensions.as_slice()
    }
}

impl Hash for TmpfsOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            size,
            mode,
            extensions,
        } = self;

        size.hash(state);
        mode.hash(state);
        extensions.as_slice().hash(state);
    }
}

/// Named pipe [`Mount`] type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedPipe {
    /// Source of the named pipe on the host.
    pub source: HostPath,

    /// Common [`Mount`] options.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub common: Common,
}

impl From<(HostPath, Common)> for NamedPipe {
    fn from((source, common): (HostPath, Common)) -> Self {
        Self { source, common }
    }
}

/// Cluster [`Mount`] type.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cluster {
    /// Source of the cluster mount.
    pub source: String,

    /// Common [`Mount`] options.
    ///
    /// (De)serialized via flattening.
    #[serde(flatten)]
    pub common: Common,
}

impl From<(String, Common)> for Cluster {
    fn from((source, common): (String, Common)) -> Self {
        Self { source, common }
    }
}

/// Options common to all [`Mount`] types.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#long-syntax-5)
#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub struct Common {
    /// Path within the container of the mount.
    pub target: AbsolutePath,

    /// Whether the mount is set as read-only.
    #[serde(default, skip_serializing_if = "Not::not")]
    pub read_only: bool,

    /// Consistency requirements of the mount.
    ///
    /// Available values are platform specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency: Option<String>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Common {
    /// Create new [`Common`] [`Mount`] options from a `target`.
    #[must_use]
    pub fn new(target: AbsolutePath) -> Self {
        Self {
            target,
            read_only: false,
            consistency: None,
            extensions: Extensions::default(),
        }
    }

    /// Returns `true` if these common [`Mount`] options are compatible with the [`ShortVolume`]
    /// syntax.
    #[must_use]
    fn is_short_compatible(&self) -> bool {
        self.consistency.is_none() && self.extensions.is_empty()
    }
}

impl PartialEq for Common {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            target,
            read_only,
            consistency,
            extensions,
        } = self;

        *target == other.target
            && *read_only == other.read_only
            && *consistency == other.consistency
            && extensions.as_slice() == other.extensions.as_slice()
    }
}

impl Hash for Common {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            target,
            read_only,
            consistency,
            extensions,
        } = self;

        target.hash(state);
        read_only.hash(state);
        consistency.hash(state);
        extensions.as_slice().hash(state);
    }
}

impl From<AbsolutePath> for Common {
    fn from(target: AbsolutePath) -> Self {
        Self::new(target)
    }
}
