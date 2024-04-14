//! Provides [`UserOrGroup`] for the `user` and `group_add` fields of [`Service`](super::Service).

use std::fmt::{self, Display, Formatter};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use serde::{de, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{common::key_impls, serde::forward_visitor};

use crate::impl_from_str;

/// User or group inside a [`Service`](super::Service) container.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum UserOrGroup {
    /// User Id (UID) or Group Id (GID).
    Id(u32),

    /// A named user or group.
    Name(Name),
}

impl UserOrGroup {
    /// Parse a [`UserOrGroup`] from a string.
    ///
    /// If an unsigned integer, the string is parsed into an [`Id`](Self::Id), otherwise it is converted into
    /// a [`Name`].
    ///
    /// # Errors
    ///
    /// Returns an error if not an unsigned integer and the conversion into a [`Name`] fails.
    pub fn parse<T>(user_or_group: T) -> Result<Self, T::Error>
    where
        T: AsRef<str> + TryInto<Name>,
    {
        user_or_group.as_ref().parse().map_or_else(
            |_| user_or_group.try_into().map(Self::Name),
            |id| Ok(Self::Id(id)),
        )
    }

    /// Returns `true` if the user or group is an [`Id`](Self::Id).
    #[must_use]
    pub const fn is_id(&self) -> bool {
        matches!(self, Self::Id(..))
    }

    /// Returns [`Some`] if [`Id`](Self::Id).
    #[must_use]
    pub const fn as_id(&self) -> Option<u32> {
        if let Self::Id(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns `true` if the user or group is a [`Name`](Self::Name).
    #[must_use]
    pub const fn is_name(&self) -> bool {
        matches!(self, Self::Name(..))
    }

    /// Returns [`Some`] if [`Name`](Self::Name).
    #[must_use]
    pub const fn as_name(&self) -> Option<&Name> {
        if let Self::Name(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<u32> for UserOrGroup {
    fn from(value: u32) -> Self {
        Self::Id(value)
    }
}

impl From<Name> for UserOrGroup {
    fn from(value: Name) -> Self {
        Self::Name(value)
    }
}

impl_from_str!(UserOrGroup => InvalidNameError);

impl Display for UserOrGroup {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Id(id) => Display::fmt(id, f),
            Self::Name(name) => Display::fmt(name, f),
        }
    }
}

impl From<UserOrGroup> for String {
    fn from(value: UserOrGroup) -> Self {
        match value {
            UserOrGroup::Id(id) => id.to_string(),
            UserOrGroup::Name(name) => name.into(),
        }
    }
}

impl<'de> Deserialize<'de> for UserOrGroup {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

/// [`de::Visitor`] for deserializing [`UserOrGroup`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = UserOrGroup;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an integer or string")
    }

    forward_visitor! {
        visit_u32,
        visit_i8: i8,
        visit_i16: i16,
        visit_i32: i32,
        visit_i64: i64,
        visit_i128: i128,
        visit_u8: u8,
        visit_u16: u16,
        visit_u64: u64,
        visit_u128: u128,
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(v.into())
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.parse().map(UserOrGroup::Name).map_err(E::custom)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        v.try_into().map(UserOrGroup::Name).map_err(E::custom)
    }
}

/// [`UserOrGroup`] name.
///
/// User and group names must:
///
/// - Not be empty.
/// - Only contain ASCII letters (a-z, A-Z), digits (0-9), underscores (_), and dashes (-),
///   with an optional dollar sign ($) at the end.
/// - Not start with a dash (-).
/// - Not be fully numeric.
/// - Be 32 characters or shorter.
///
/// See [**useradd**(8)](https://man7.org/linux/man-pages/man8/useradd.8.html) and
/// [**groupadd**(8)](https://man7.org/linux/man-pages/man8/groupadd.8.html) for details.
#[derive(
    SerializeDisplay, DeserializeTryFromString, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Name(Box<str>);

impl Name {
    /// Create a new [`Name`], validating the string.
    ///
    /// # Errors
    ///
    /// Returns an error if the name:
    ///
    /// - Is empty.
    /// - Does not contain only ASCII letters (a-z, A-Z), digits (0-9), underscores (_), and
    ///   dashes (-), with an optional dollar sign ($) at the end.
    /// - Starts with a dash (-).
    /// - Is fully numeric.
    /// - Is longer than 32 characters.
    pub fn new<T>(name: T) -> Result<Self, InvalidNameError>
    where
        T: AsRef<str> + Into<Box<str>>,
    {
        let name_str = name.as_ref();

        if name_str.is_empty() {
            return Err(InvalidNameError::Empty);
        }

        let mut fully_numeric = true;
        for (n, char) in name_str.chars().enumerate() {
            match char {
                'a'..='z' | 'A'..='Z' | '_' | '-' | '$' => {
                    fully_numeric = false;
                    if char == '$' && n != name_str.len() - 1 {
                        return Err(InvalidNameError::DollarSign);
                    }
                }
                '0'..='9' => {}
                invalid => return Err(InvalidNameError::Character(invalid)),
            }
        }

        if fully_numeric {
            Err(InvalidNameError::Numeric)
        } else if name_str.starts_with('-') {
            Err(InvalidNameError::Start)
        } else if name_str.len() > 32 {
            Err(InvalidNameError::Length)
        } else {
            Ok(Self(name.into()))
        }
    }
}

/// Error returned when parsing a [`Name`] from a string.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidNameError {
    /// User/group name was empty.
    #[error("user and group names cannot be empty")]
    Empty,

    /// User/group name contained an invalid character.
    #[error(
        "invalid user or group name character `{0}`, names may only contain \
            ASCII letters (a-z, A-Z), digits (0-9), underscores (_), and dashes (-), \
            with an optional dollar sign ($) at the end"
    )]
    Character(char),

    /// User/group name contained a dollar sign ($) not at the end.
    #[error("user and group names may only have a dollar sign ($) at the end")]
    DollarSign,

    /// User/group name contained only digits (0-9).
    #[error("user and group names cannot be fully numeric")]
    Numeric,

    /// User/group name started with a dash (-).
    #[error("user and group names cannot start with a dash (-)")]
    Start,

    /// User/group name was longer than 32 characters.
    #[error("user and group names may only be up to 32 characters long")]
    Length,
}

key_impls!(Name => InvalidNameError);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    mod name {
        use pomsky_macro::pomsky;
        use proptest::proptest;

        use super::*;

        const NAME: &str = pomsky! {
            let middle = [ascii_alnum '_' '-'];

            [ascii_alpha '_'] ( middle{0,31} | middle{0,30} '$' )
        };

        proptest! {
            #[test]
            fn no_panic(string: String) {
                let _ = Name::new(string);
            }

            #[test]
            fn valid(name in NAME) {
                Name::new(name)?;
            }
        }

        #[test]
        fn dollar_sign() {
            Name::new("test$").unwrap();

            assert_eq!(Name::new("te$t").unwrap_err(), InvalidNameError::DollarSign);
        }

        #[test]
        fn numeric_err() {
            assert_eq!(Name::new("1000").unwrap_err(), InvalidNameError::Numeric);
        }
    }
}
