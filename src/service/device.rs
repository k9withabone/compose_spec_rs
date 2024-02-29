//! Provides [`Device`] and [`CgroupRule`] for the `devices` and `device_cgroup_rules` fields of
//! [`Service`](super::Service).

use std::{
    fmt::{self, Display, Formatter, Write},
    num::ParseIntError,
    path::PathBuf,
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

/// Device mapping from the host to the [`Service`](super::Service) container.
///
/// (De)serializes from/to a string in the format `{host_path}:{container_path}[:{permissions}]`
/// e.g. `/host:/container:rwm`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Device {
    /// Path on the host of the device.
    pub host_path: PathBuf,

    /// Path inside the container to bind mount the device to.
    pub container_path: PathBuf,

    /// Device cgroup permissions.
    pub permissions: Permissions,
}

impl FromStr for Device {
    type Err = ParseDeviceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is "{host_path}:{container_path}[:{permissions}]"
        let mut split = s.splitn(3, ':');

        let host_path = split.next().ok_or(ParseDeviceError::Empty)?.into();
        let container_path = split
            .next()
            .ok_or(ParseDeviceError::ContainerPathMissing)?
            .into();
        let permissions = split.next().unwrap_or_default().parse()?;

        Ok(Self {
            host_path,
            container_path,
            permissions,
        })
    }
}

impl TryFrom<&str> for Device {
    type Error = ParseDeviceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`Device`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseDeviceError {
    /// Given device was an empty string.
    #[error("device cannot be an empty string")]
    Empty,

    /// Device was missing a container path.
    #[error("device must have a container path")]
    ContainerPathMissing,

    /// Error parsing [`Permissions`].
    #[error("error parsing device permissions")]
    Permissions(#[from] ParsePermissionsError),
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            host_path,
            container_path,
            permissions,
        } = self;

        write!(f, "{}:{}", host_path.display(), container_path.display())?;

        if permissions.any() {
            write!(f, ":{permissions}")?;
        }

        Ok(())
    }
}

/// [`Device`] or [`DeviceCgroupRule`](super::DeviceCgroupRule) access permissions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permissions {
    /// Device read permissions.
    pub read: bool,

    /// Device write permissions.
    pub write: bool,

    /// Device mknod permissions.
    pub mknod: bool,
}

impl Permissions {
    /// Create [`Permissions`] where all fields are `true`.
    #[must_use]
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
            mknod: true,
        }
    }

    /// Returns `true` if any of the permissions are `true`.
    #[must_use]
    pub fn any(self) -> bool {
        let Self { read, write, mknod } = self;
        read || write || mknod
    }
}

impl FromStr for Permissions {
    type Err = ParsePermissionsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut read = false;
        let mut write = false;
        let mut mknod = false;

        for permission in s.chars() {
            match permission {
                'r' => read = true,
                'w' => write = true,
                'm' => mknod = true,
                unknown => return Err(ParsePermissionsError(unknown)),
            }
        }

        Ok(Self { read, write, mknod })
    }
}

impl TryFrom<&str> for Permissions {
    type Error = ParsePermissionsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing [`Permissions`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("invalid device permission `{0}`, must be `r` (read), `w` (write), or `m` (mknod)")]
pub struct ParsePermissionsError(char);

impl Display for Permissions {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self { read, write, mknod } = *self;

        if read {
            f.write_char('r')?;
        }
        if write {
            f.write_char('w')?;
        }
        if mknod {
            f.write_char('m')?;
        }

        Ok(())
    }
}

/// Device cgroup rule for a [`Service`](super::Service) container.
///
/// (De)serializes from/to a string in the format the Linux kernel specifies in the
/// [Device Whitelist Controller](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/devices.html)
/// documentation, e.g. `a 7:* rwm`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_cgroup_rules)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CgroupRule {
    /// Device type: character, block, or all.
    pub kind: Kind,

    /// Device major number.
    pub major: MajorMinorNumber,

    /// Device minor number.
    pub minor: MajorMinorNumber,

    /// Device permissions.
    pub permissions: Permissions,
}

impl FromStr for CgroupRule {
    type Err = ParseCgroupRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The format is "{kind} {major}:{minor} {permissions}".

        let mut split = s.splitn(3, ' ');

        let kind = split.next().ok_or(ParseCgroupRuleError::Empty)?.parse()?;

        let (major, minor) = split
            .next()
            .and_then(|s| s.split_once(':'))
            .ok_or(ParseCgroupRuleError::MajorMinorNumbersMissing)?;
        let major = major.parse()?;
        let minor = minor.parse()?;

        let permissions = split.next().unwrap_or_default().parse()?;

        Ok(Self {
            kind,
            major,
            minor,
            permissions,
        })
    }
}

impl TryFrom<&str> for CgroupRule {
    type Error = ParseCgroupRuleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`CgroupRule`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseCgroupRuleError {
    /// Device cgroup rule was empty.
    #[error("device cgroup rule cannot be empty")]
    Empty,

    /// Error parsing [`Kind`].
    #[error("invalid device kind")]
    Kind(#[from] ParseKindError),

    /// Device major and minor numbers were missing or not in the expected format.
    #[error("device cgroup rule missing major minor numbers")]
    MajorMinorNumbersMissing,

    /// Error parsing [`MajorMinorNumber`].
    #[error("error parsing device major minor number")]
    MajorMinorNumber(#[from] ParseIntError),

    /// Error parsing [`Permissions`].
    #[error("error parsing device cgroup rule permissions")]
    Permissions(#[from] ParsePermissionsError),
}

impl Display for CgroupRule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            kind,
            major,
            minor,
            permissions,
        } = self;

        // The format is "{kind} {major}:{minor} {permissions}".

        write!(f, "{kind} {major}:{minor}")?;

        if permissions.any() {
            write!(f, " {permissions}")?;
        }

        Ok(())
    }
}

/// Device types for [`CgroupRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// All device types.
    All,

    /// Character device.
    Char,

    /// Block device.
    Block,
}

impl Kind {
    /// The character the device type corresponds to.
    #[must_use]
    pub fn as_char(self) -> char {
        match self {
            Self::All => 'a',
            Self::Char => 'c',
            Self::Block => 'b',
        }
    }
}

impl TryFrom<char> for Kind {
    type Error = ParseKindError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'a' => Ok(Self::All),
            'c' => Ok(Self::Char),
            'b' => Ok(Self::Block),
            unknown => Err(ParseKindError(unknown.into())),
        }
    }
}

impl FromStr for Kind {
    type Err = ParseKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(Self::All),
            "c" => Ok(Self::Char),
            "b" => Ok(Self::Block),
            unknown => Err(ParseKindError(unknown.to_owned())),
        }
    }
}

impl TryFrom<&str> for Kind {
    type Error = ParseKindError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when attempting to parse [`Kind`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("invalid device kind `{0}`, must be `a` (all), `c` (char), or `b` (block)")]
pub struct ParseKindError(String);

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.as_char())
    }
}

impl From<Kind> for char {
    fn from(value: Kind) -> Self {
        value.as_char()
    }
}

/// Device major or minor number for [`CgroupRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MajorMinorNumber {
    /// All major/minor numbers (*).
    All,

    /// A specific major/minor number.
    Integer(u16),
}

impl PartialEq<u16> for MajorMinorNumber {
    fn eq(&self, other: &u16) -> bool {
        match self {
            Self::All => false,
            Self::Integer(num) => num.eq(other),
        }
    }
}

impl From<u16> for MajorMinorNumber {
    fn from(value: u16) -> Self {
        Self::Integer(value)
    }
}

impl FromStr for MajorMinorNumber {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == "*" {
            Ok(Self::All)
        } else {
            s.parse().map(Self::Integer)
        }
    }
}

impl TryFrom<&str> for MajorMinorNumber {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for MajorMinorNumber {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::All => f.write_char('*'),
            Self::Integer(num) => Display::fmt(num, f),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::{
        arbitrary::any,
        prop_assert_eq, prop_compose, prop_oneof, proptest,
        strategy::{Just, Strategy},
    };

    use super::*;

    mod device {
        use crate::service::tests::path_no_colon;

        use super::*;

        #[test]
        fn from_str() {
            let device = Device {
                host_path: "/host".into(),
                container_path: "/container".into(),
                permissions: Permissions {
                    read: true,
                    write: true,
                    mknod: false,
                },
            };
            assert_eq!(device, "/host:/container:rw".parse().unwrap());
        }

        #[test]
        fn display() {
            let device = Device {
                host_path: "/host".into(),
                container_path: "/container".into(),
                permissions: Permissions {
                    read: true,
                    write: true,
                    mknod: false,
                },
            };
            assert_eq!(device.to_string(), "/host:/container:rw");
        }

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<Device>();
            }

            #[test]
            fn to_string_no_panic(device in device()) {
                device.to_string();
            }

            #[test]
            fn round_trip(device in device()) {
                prop_assert_eq!(&device, &device.to_string().parse()?);
            }
        }

        prop_compose! {
            fn device()(
                host_path in path_no_colon(),
                container_path in path_no_colon(),
                permissions in permissions()
            ) -> Device {
                Device { host_path, container_path, permissions }
            }
        }
    }

    mod permissions {
        use super::*;

        #[test]
        fn from_str() {
            assert_eq!(Permissions::default(), "".parse().unwrap());
            assert_eq!(
                Permissions {
                    read: true,
                    write: true,
                    mknod: true
                },
                "rwm".parse().unwrap(),
            );
        }

        #[test]
        fn display() {
            assert!(Permissions::default().to_string().is_empty());
            assert_eq!(
                Permissions {
                    read: true,
                    write: true,
                    mknod: true
                }
                .to_string(),
                "rwm",
            );
        }

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<Permissions>();
            }

            #[test]
            fn to_string_no_panic(permissions in permissions()) {
                permissions.to_string();
            }

            #[test]
            fn round_trip(permissions in permissions()) {
                prop_assert_eq!(permissions, permissions.to_string().parse()?);
            }
        }
    }

    mod cgroup_rule {
        use super::*;

        #[test]
        fn from_str() {
            let rule = CgroupRule {
                kind: Kind::Char,
                major: MajorMinorNumber::Integer(1),
                minor: MajorMinorNumber::Integer(3),
                permissions: Permissions {
                    read: true,
                    write: false,
                    mknod: true,
                },
            };
            assert_eq!(rule, "c 1:3 mr".parse().unwrap());

            let rule = CgroupRule {
                kind: Kind::All,
                major: MajorMinorNumber::Integer(7),
                minor: MajorMinorNumber::All,
                permissions: Permissions::all(),
            };
            assert_eq!(rule, "a 7:* rmw".parse().unwrap());
        }

        #[test]
        fn display() {
            let rule = CgroupRule {
                kind: Kind::Char,
                major: MajorMinorNumber::Integer(1),
                minor: MajorMinorNumber::Integer(3),
                permissions: Permissions {
                    read: true,
                    write: false,
                    mknod: true,
                },
            };
            assert_eq!(rule.to_string(), "c 1:3 rm");

            let rule = CgroupRule {
                kind: Kind::All,
                major: MajorMinorNumber::Integer(7),
                minor: MajorMinorNumber::All,
                permissions: Permissions::all(),
            };
            assert_eq!(rule.to_string(), "a 7:* rwm");
        }

        proptest! {
            #[test]
            fn parse_no_panic(string: String) {
                let _ = string.parse::<CgroupRule>();
            }

            #[test]
            fn to_string_no_panic(rule in cgroup_rule()) {
                rule.to_string();
            }

            #[test]
            fn round_trip(rule in cgroup_rule()) {
                prop_assert_eq!(rule, rule.to_string().parse()?);
            }
        }

        prop_compose! {
            fn cgroup_rule()(
                kind in kind(),
                major in major_minor_number(),
                minor in major_minor_number(),
                permissions in permissions(),
            ) -> CgroupRule {
                CgroupRule {
                    kind,
                    major,
                    minor,
                    permissions,
                }
            }
        }

        fn kind() -> impl Strategy<Value = Kind> {
            prop_oneof![Just(Kind::All), Just(Kind::Char), Just(Kind::Block)]
        }

        fn major_minor_number() -> impl Strategy<Value = MajorMinorNumber> {
            prop_oneof![
                1 => Just(MajorMinorNumber::All),
                u16::MAX.into() => any::<u16>().prop_map_into(),
            ]
        }
    }

    prop_compose! {
        fn permissions()(read: bool, write: bool, mknod: bool) -> Permissions {
            Permissions { read, write, mknod }
        }
    }
}
