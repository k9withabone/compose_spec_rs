//! Provides [`Healthcheck`] for the `healthcheck` field of [`Service`](super::Service).

use std::{
    fmt::{self, Formatter},
    iter,
    time::Duration,
};

use serde::{
    de::{self, value::SeqAccessDeserializer, DeserializeSeed, MapAccess, SeqAccess},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{serde::duration_option, ExtensionKey, Extensions};

/// A check that is run to determine whether the [`Service`](super::Service) container is "healthy".
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#healthcheck)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Healthcheck {
    /// Set or override healthcheck options.
    Command(Command),

    /// Disable the container image's healthcheck.
    Disable,
}

impl From<Command> for Healthcheck {
    fn from(value: Command) -> Self {
        Self::Command(value)
    }
}

impl Serialize for Healthcheck {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Command(command) => command.serialize(serializer),
            Self::Disable => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("disable", &true)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Healthcheck {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(Visitor)
    }
}

/// [`de::Visitor`] for deserializing [`Healthcheck`].
struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Healthcheck;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a map defining a healthcheck")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut disable = None;
        let mut test = None;
        let mut interval = None;
        let mut timeout = None;
        let mut retries = None;
        let mut start_period = None;
        let mut start_interval = None;
        let mut extensions = Extensions::new();

        while let Some(field) = map.next_key()? {
            match field {
                Field::Disable => {
                    check_duplicate(&disable, "disable")?;
                    disable = map.next_value().map(Some)?;
                }
                Field::Test => {
                    check_duplicate(&test, "test")?;
                    map.next_value_seed(DisableOrTest {
                        disable: &mut disable,
                        test: &mut test,
                    })?;
                }
                Field::Interval => {
                    check_duplicate(&interval, "interval")?;
                    interval = map.next_value::<DurationOption>()?.0;
                }
                Field::Timeout => {
                    check_duplicate(&timeout, "timeout")?;
                    timeout = map.next_value::<DurationOption>()?.0;
                }
                Field::Retries => {
                    check_duplicate(&retries, "retries")?;
                    retries = map.next_value().map(Some)?;
                }
                Field::StartPeriod => {
                    check_duplicate(&start_period, "start_period")?;
                    start_period = map.next_value::<DurationOption>()?.0;
                }
                Field::StartInterval => {
                    check_duplicate(&start_interval, "start_interval")?;
                    start_interval = map.next_value::<DurationOption>()?.0;
                }
                Field::Extension(extension) => {
                    if extensions.insert(extension, map.next_value()?).is_some() {
                        return Err(de::Error::custom("duplicate extension key"));
                    }
                }
            }
        }

        if disable.unwrap_or_default() {
            if test.is_none()
                && interval.is_none()
                && timeout.is_none()
                && retries.is_none()
                && start_period.is_none()
                && start_interval.is_none()
                && extensions.is_empty()
            {
                Ok(Healthcheck::Disable)
            } else {
                Err(de::Error::custom(
                    "a disabled healthcheck cannot have other options set",
                ))
            }
        } else {
            Ok(Healthcheck::Command(Command {
                test,
                interval,
                timeout,
                retries,
                start_period,
                start_interval,
                extensions,
            }))
        }
    }
}

/// Checks whether `option` is already set and returns an error if so.
fn check_duplicate<T, E: de::Error>(option: &Option<T>, field: &'static str) -> Result<(), E> {
    if option.is_none() {
        Ok(())
    } else {
        Err(E::duplicate_field(field))
    }
}

/// Fields of [`Healthcheck`] / [`Command`].
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    /// `disable` / [`Healthcheck::Disable`]
    Disable,

    /// `test`
    Test,

    /// `interval`
    Interval,

    /// `timeout`
    Timeout,

    /// `retries`
    Retries,

    /// `start_period`
    StartPeriod,

    /// `start_interval`
    StartInterval,

    /// Extension key.
    Extension(ExtensionKey),
}

/// Helper for deserializing the `test` field of [`Healthcheck`] / [`Command`].
///
/// If `test` is set to `NONE`, then [`Healthcheck::Disable`] is deserialized.
struct DisableOrTest<'a> {
    /// `disable` field.
    disable: &'a mut Option<bool>,

    /// `test` field.
    test: &'a mut Option<Test>,
}

impl<'a, 'de> DeserializeSeed<'de> for DisableOrTest<'a> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'a, 'de> de::Visitor<'de> for DisableOrTest<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(
            "a string or a sequence of strings, the first being `CMD`, `CMD-SHELL`, or `NONE`",
        )
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        *self.test = Some(v.into());
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        match seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &"`CMD`, `CMD-SHELL`, or `NONE`"))?
        {
            TestKind::Cmd => {
                *self.test = Vec::deserialize(SeqAccessDeserializer::new(seq))
                    .map(Test::Command)
                    .map(Some)?;

                Ok(())
            }
            TestKind::CmdShell => {
                *self.test = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &"a shell argument"))
                    .map(Test::ShellCommand)
                    .map(Some)?;

                if seq.next_element::<String>()?.is_none() {
                    Ok(())
                } else {
                    Err(de::Error::invalid_length(3, &"a single shell argument"))
                }
            }
            TestKind::None => {
                if self.disable.unwrap_or_default() {
                    Err(de::Error::custom(
                        "cannot set `disable` to `true` and `test` to `NONE` \
                            or set `test` to `NONE` multiple times",
                    ))
                } else {
                    *self.disable = Some(true);

                    if seq.next_element::<String>()?.is_none() {
                        Ok(())
                    } else {
                        Err(de::Error::invalid_length(2, &"no arguments"))
                    }
                }
            }
        }
    }
}

/// Possible first values in [`Test`] sequence.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(variant_identifier, rename_all = "SCREAMING-KEBAB-CASE")]
enum TestKind {
    /// [`Test::Command`]
    Cmd,

    /// [`Test::ShellCommand`]
    CmdShell,

    /// [`Healthcheck::Disable`]
    None,
}

/// Wrapper for deserializing [`Option<Duration>`] with [`duration_option::deserialize()`].
#[derive(Deserialize)]
#[serde(transparent)]
struct DurationOption(#[serde(with = "duration_option")] Option<Duration>);

/// [`Healthcheck`] command configuration.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#healthcheck)
#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Command {
    /// The command run to check container health.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<Test>,

    /// The time between subsequent healthchecks.
    ///
    /// Default is 30 seconds.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub interval: Option<Duration>,

    /// How long before a healthcheck is considered failed.
    ///
    /// Default is 30 seconds.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub timeout: Option<Duration>,

    /// How many consecutive healthchecks have to fail before the container is considered unhealthy.
    ///
    /// Default is 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retries: Option<u64>,

    /// Initialization time for the container before healthcheck failure is counted.
    ///
    /// Default is 0 seconds.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub start_period: Option<Duration>,

    /// Time between healthchecks during initialization defined by `start_period`.
    ///
    /// Default is 5 seconds.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "duration_option"
    )]
    pub start_interval: Option<Duration>,

    /// Extension values, which are (de)serialized via flattening.
    ///
    /// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md)
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Command run to check container health as part of a [`Healthcheck`].
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#healthcheck)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Test {
    /// A command and its arguments.
    Command(Vec<String>),

    /// Command run with the container's default shell (`/bin/sh -c` for Linux).
    ShellCommand(String),
}

impl From<Vec<String>> for Test {
    fn from(value: Vec<String>) -> Self {
        Self::Command(value)
    }
}

impl From<String> for Test {
    fn from(value: String) -> Self {
        Self::ShellCommand(value)
    }
}

impl Serialize for Test {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Command(test) => {
                serializer.collect_seq(iter::once("CMD").chain(test.iter().map(String::as_str)))
            }
            Self::ShellCommand(test) => test.serialize(serializer),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use proptest::{
        arbitrary::any,
        option, prop_assert_eq, prop_compose, prop_oneof, proptest,
        strategy::{Just, Strategy},
    };

    use crate::duration::tests::duration_truncated;

    use super::*;

    #[test]
    fn disable() {
        assert_eq!(
            Healthcheck::Disable,
            serde_yaml::from_str("disable: true").unwrap(),
        );

        assert_eq!(
            Healthcheck::Disable,
            serde_yaml::from_str("test: [NONE]").unwrap(),
        );

        assert!(
            serde_yaml::from_str::<Healthcheck>("disable: true\ntest: [NONE]")
                .unwrap_err()
                .to_string()
                .contains("cannot set `disable`"),
        );

        assert!(
            serde_yaml::from_str::<Healthcheck>("disable: true\ninterval: 3s")
                .unwrap_err()
                .to_string()
                .contains("disabled"),
        );
    }

    #[test]
    fn test_cmd() {
        assert_eq!(
            Healthcheck::Command(Command {
                test: Some(Test::Command(vec!["test".to_owned()])),
                ..Command::default()
            }),
            serde_yaml::from_str("test: [CMD, test]").unwrap(),
        );
    }

    #[test]
    fn test_cmd_shell() {
        let healthcheck = Healthcheck::Command(Command {
            test: Some(Test::ShellCommand("test".to_owned())),
            ..Command::default()
        });
        assert_eq!(
            healthcheck,
            serde_yaml::from_str("test: [CMD-SHELL, test]").unwrap(),
        );
        assert_eq!(healthcheck, serde_yaml::from_str("test: test").unwrap(),);

        assert!(serde_yaml::from_str::<Healthcheck>("test: [CMD-SHELL]")
            .unwrap_err()
            .to_string()
            .contains('1'));

        assert!(
            serde_yaml::from_str::<Healthcheck>("test: [CMD-SHELL, test, test]")
                .unwrap_err()
                .to_string()
                .contains('3')
        );
    }

    proptest! {
        #[test]
        fn round_trip(healthcheck in healthcheck()) {
            let string = serde_yaml::to_string(&healthcheck)?;
            prop_assert_eq!(healthcheck, serde_yaml::from_str(&string)?);
        }
    }

    fn healthcheck() -> impl Strategy<Value = Healthcheck> {
        prop_oneof![
            1 => Just(Healthcheck::Disable),
            3 => command().prop_map_into(),
        ]
    }

    prop_compose! {
        fn command()(
            test in option::of(test()),
            interval in option::of(duration_truncated()),
            timeout in option::of(duration_truncated()),
            retries: Option<u64>,
            start_period in option::of(duration_truncated()),
            start_interval in option::of(duration_truncated()),
        ) -> Command {
            Command {
                test,
                interval,
                timeout,
                retries,
                start_period,
                start_interval,
                extensions: Extensions::new(),
            }
        }
    }

    fn test() -> impl Strategy<Value = Test> {
        prop_oneof![
            any::<String>().prop_map_into(),
            any::<Vec<String>>().prop_map_into(),
        ]
    }
}
