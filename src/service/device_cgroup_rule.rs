//! Provides [`DeviceCgroupRule`] for the `device_cgroup_rules` field of [`Service`](super::Service).

use std::{
    fmt::{self, Display, Formatter, Write},
    num::ParseIntError,
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

/// Device cgroup rule for a [`Service`](super::Service) container.
///
/// (De)serializes from/to a string in the format the Linux kernel specifies in the
/// [Device Whitelist Controller](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/devices.html)
/// documentation, e.g. `a 7:* rwm`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#device_cgroup_rules)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceCgroupRule {
    /// Device type: character, block, or all.
    pub kind: DeviceKind,

    /// Device major number.
    pub major: MajorMinorNumber,

    /// Device minor number.
    pub minor: MajorMinorNumber,

    /// Device read permissions.
    pub read: bool,

    /// Device write permissions.
    pub write: bool,

    /// Device mknod permissions.
    pub mknod: bool,
}

impl FromStr for DeviceCgroupRule {
    type Err = ParseDeviceCgroupRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The format is "{kind} {major}:{minor} {permissions}".

        let mut split = s.splitn(3, ' ');

        let kind = split
            .next()
            .ok_or(ParseDeviceCgroupRuleError::Empty)?
            .parse()?;

        let (major, minor) = split
            .next()
            .and_then(|s| s.split_once(':'))
            .ok_or(ParseDeviceCgroupRuleError::MajorMinorNumbersMissing)?;
        let major = major.parse()?;
        let minor = minor.parse()?;

        let permissions = split.next().unwrap_or_default();

        let mut read = false;
        let mut write = false;
        let mut mknod = false;

        for permission in permissions.chars() {
            match permission {
                'r' => read = true,
                'w' => write = true,
                'm' => mknod = true,
                unknown => return Err(ParseDeviceCgroupRuleError::Permission(unknown)),
            }
        }

        Ok(Self {
            kind,
            major,
            minor,
            read,
            write,
            mknod,
        })
    }
}

impl TryFrom<&str> for DeviceCgroupRule {
    type Error = ParseDeviceCgroupRuleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when parsing a [`DeviceCgroupRule`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseDeviceCgroupRuleError {
    /// Device cgroup rule was empty.
    #[error("device cgroup rule cannot be empty")]
    Empty,

    /// Error parsing [`DeviceKind`].
    #[error("invalid device kind")]
    DeviceKind(#[from] ParseDeviceKindError),

    /// Device major and minor numbers were missing or not in the expected format.
    #[error("device cgroup rule missing major minor numbers")]
    MajorMinorNumbersMissing,

    /// Error parsing [`MajorMinorNumber`].
    #[error("error parsing device major minor number")]
    MajorMinorNumber(#[from] ParseIntError),

    /// Invalid device permission given.
    #[error(
        "invalid device access permission `{0}`, must be `r` (read), `w` (write), or `m` (mknod)"
    )]
    Permission(char),
}

impl Display for DeviceCgroupRule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            kind,
            major,
            minor,
            read,
            write,
            mknod,
        } = self;

        // The format is "{kind} {major}:{minor} {permissions}".

        write!(f, "{kind} {major}:{minor}")?;

        if *read || *write || *mknod {
            f.write_char(' ')?;
        }
        if *read {
            f.write_char('r')?;
        }
        if *write {
            f.write_char('w')?;
        }
        if *mknod {
            f.write_char('m')?;
        }

        Ok(())
    }
}

/// Device types for [`DeviceCgroupRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// All device types.
    All,

    /// Character device.
    Char,

    /// Block device.
    Block,
}

impl DeviceKind {
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

impl TryFrom<char> for DeviceKind {
    type Error = ParseDeviceKindError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'a' => Ok(Self::All),
            'c' => Ok(Self::Char),
            'b' => Ok(Self::Block),
            unknown => Err(ParseDeviceKindError(unknown.into())),
        }
    }
}

impl FromStr for DeviceKind {
    type Err = ParseDeviceKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(Self::All),
            "c" => Ok(Self::Char),
            "b" => Ok(Self::Block),
            unknown => Err(ParseDeviceKindError(unknown.to_owned())),
        }
    }
}

impl TryFrom<&str> for DeviceKind {
    type Error = ParseDeviceKindError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Error returned when attempting to parse [`DeviceKind`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("invalid device kind `{0}`, must be `a` (all), `c` (char), or `b` (block)")]
pub struct ParseDeviceKindError(String);

impl Display for DeviceKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.as_char())
    }
}

impl From<DeviceKind> for char {
    fn from(value: DeviceKind) -> Self {
        value.as_char()
    }
}

/// Device major or minor number for [`DeviceCgroupRule`].
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

    #[test]
    fn from_str() {
        let rule = DeviceCgroupRule {
            kind: DeviceKind::Char,
            major: MajorMinorNumber::Integer(1),
            minor: MajorMinorNumber::Integer(3),
            read: true,
            write: false,
            mknod: true,
        };
        assert_eq!(rule, "c 1:3 mr".parse().unwrap());

        let rule = DeviceCgroupRule {
            kind: DeviceKind::All,
            major: MajorMinorNumber::Integer(7),
            minor: MajorMinorNumber::All,
            read: true,
            write: true,
            mknod: true,
        };
        assert_eq!(rule, "a 7:* rmw".parse().unwrap());
    }

    #[test]
    fn display() {
        let rule = DeviceCgroupRule {
            kind: DeviceKind::Char,
            major: MajorMinorNumber::Integer(1),
            minor: MajorMinorNumber::Integer(3),
            read: true,
            write: false,
            mknod: true,
        };
        assert_eq!(rule.to_string(), "c 1:3 rm");

        let rule = DeviceCgroupRule {
            kind: DeviceKind::All,
            major: MajorMinorNumber::Integer(7),
            minor: MajorMinorNumber::All,
            read: true,
            write: true,
            mknod: true,
        };
        assert_eq!(rule.to_string(), "a 7:* rwm");
    }

    proptest! {
        #[test]
        fn parse_no_panic(string: String) {
            let _ = string.parse::<DeviceCgroupRule>();
        }

        #[test]
        fn to_string_no_panic(rule in device_cgroup_rule()) {
            rule.to_string();
        }

        #[test]
        fn round_trip(rule in device_cgroup_rule()) {
            prop_assert_eq!(rule, rule.to_string().parse()?);
        }
    }

    prop_compose! {
        fn device_cgroup_rule()(
            kind in device_kind(),
            major in major_minor_number(),
            minor in major_minor_number(),
            read: bool,
            write: bool,
            mknod: bool,
        ) -> DeviceCgroupRule {
            DeviceCgroupRule {
                kind,
                major,
                minor,
                read,
                write,
                mknod,
            }
        }
    }

    fn device_kind() -> impl Strategy<Value = DeviceKind> {
        prop_oneof![
            Just(DeviceKind::All),
            Just(DeviceKind::Char),
            Just(DeviceKind::Block),
        ]
    }

    fn major_minor_number() -> impl Strategy<Value = MajorMinorNumber> {
        prop_oneof![
            1 => Just(MajorMinorNumber::All),
            u16::MAX.into() => any::<u16>().prop_map_into(),
        ]
    }
}
