//! Utilities for working with [`Duration`]s.
//!
//! # String Format
//!
//! The compose-spec defines a
//! [string format for durations](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md#specifying-durations)
//! as `{value}{unit}`, where the unit may be one of the "Unit" values in the following table.
//!
//! | Unit | Name         | Equivalent Seconds            |  
//! |:----:|--------------|:------------------------------|
//! | `us` | Microseconds | 1 × 10<sup>-6</sup>, 0.000001 |
//! | `ms` | Milliseconds | 1 × 10<sup>-3</sup>, 0.001    |
//! | `s`  | Seconds      | 1                             |
//! | `m`  | Minutes      | 60                            |
//! | `h`  | Hours        | 3600                          |
//!
//! Values may be combined without a separator.
//!
//! ```text
//! 10ms
//! 40s
//! 1m30s
//! 1h5m30s20ms
//! ```

use std::time::Duration;

use thiserror::Error;

/// Number of microseconds in a millisecond.
const MICROSECONDS_PER_MILLISECOND: u32 = 1_000;

/// Number of seconds in a minute.
const SECONDS_PER_MINUTE: u64 = 60;

/// Number of minutes in an hour.
const MINUTES_PER_HOUR: u64 = 60;

/// Number of seconds in an hour.
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;

/// Convert a [`Duration`] to a [`String`] in the
/// [compose-spec duration format](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md#specifying-durations).
///
/// See the [module documentation](self) for more details on the format.
#[must_use]
pub fn to_string(duration: Duration) -> String {
    let mut string = String::new();

    let mut seconds = duration.as_secs();

    let hours = seconds / SECONDS_PER_HOUR;
    push_value(&mut string, hours, "h");
    seconds %= SECONDS_PER_HOUR;

    let minutes = seconds / SECONDS_PER_MINUTE;
    push_value(&mut string, minutes, "m");
    seconds %= SECONDS_PER_MINUTE;

    push_value(&mut string, seconds, "s");

    let mut microseconds = duration.subsec_micros();

    let milliseconds = microseconds / MICROSECONDS_PER_MILLISECOND;
    push_value(&mut string, milliseconds.into(), "ms");
    microseconds %= MICROSECONDS_PER_MILLISECOND;

    push_value(&mut string, microseconds.into(), "us");

    if string.is_empty() {
        string.push_str("0s");
    }

    string
}

/// Push "{value}{unit}" to the string if it's not zero.
fn push_value(string: &mut String, value: u64, unit: &str) {
    if value != 0 {
        string.push_str(itoa::Buffer::new().format(value));
        string.push_str(unit);
    }
}

/// Parse a [`Duration`] from a string in the
/// [compose-spec duration format](https://github.com/compose-spec/compose-spec/blob/master/11-extension.md#specifying-durations).
///
/// See the [module documentation](self) for more details on the format.
///
/// # Errors
///
/// Returns an error if the input is an empty string, does not contain ASCII digits (0-9), contains
/// an invalid unit (valid units: "h", "m", "s", "ms", "us"), or the value is too large for a
/// [`Duration`].
#[allow(clippy::missing_panics_doc)]
pub fn parse(mut s: &str) -> Result<Duration, ParseDurationError> {
    if s.is_empty() {
        return Err(ParseDurationError::Empty);
    }

    let mut duration = Duration::ZERO;

    while !s.is_empty() {
        let split = s
            .find(|char: char| !char.is_ascii_digit())
            .ok_or_else(|| ParseDurationError::MissingUnit(s.to_owned()))?;
        let (value, rest) = s.split_at(split);
        if value.is_empty() {
            return Err(ParseDurationError::NoDigits(s.to_owned()));
        }
        let value: u64 = value.parse().expect("value is ASCII digits only");

        let (unit, rest) = rest
            .find(|char: char| char.is_ascii_digit())
            .map_or((rest, ""), |split| rest.split_at(split));
        s = rest;

        match unit {
            "h" => {
                let value = value
                    .checked_mul(SECONDS_PER_HOUR)
                    .ok_or(ParseDurationError::Overflow)?;
                add_duration(&mut duration, Duration::from_secs(value))?;
            }
            "m" => {
                let value = value
                    .checked_mul(SECONDS_PER_MINUTE)
                    .ok_or(ParseDurationError::Overflow)?;
                add_duration(&mut duration, Duration::from_secs(value))?;
            }
            "s" => add_duration(&mut duration, Duration::from_secs(value))?,
            "ms" => add_duration(&mut duration, Duration::from_millis(value))?,
            "us" => add_duration(&mut duration, Duration::from_micros(value))?,
            unit => return Err(ParseDurationError::InvalidUnit(unit.to_owned())),
        }
    }

    Ok(duration)
}

/// Add `rhs` to `duration`.
///
/// # Errors
///
/// Returns an error if an overflow occurs.
fn add_duration(duration: &mut Duration, rhs: Duration) -> Result<(), ParseDurationError> {
    *duration = duration
        .checked_add(rhs)
        .ok_or(ParseDurationError::Overflow)?;
    Ok(())
}

/// Error returned when [parsing](parse()) a [`Duration`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseDurationError {
    /// Input was an empty string.
    #[error("cannot parse a duration from an empty string")]
    Empty,

    /// Input was missing a unit.
    #[error("duration `{0}` does not have a unit")]
    MissingUnit(String),

    /// Input did not contain any ASCII digits (0-9).
    #[error("duration `{0}` does not contain ASCII digits (0-9)")]
    NoDigits(String),

    /// Value was too large for [`Duration`].
    #[error("an overflow occurred")]
    Overflow,

    /// Input contained an invalid duration unit.
    ///
    /// Duration unit must be "h", "m", "s", "ms", or "us".
    #[error("`{0}` is not a valid duration unit, must be `h`, `m`, `s`, `ms`, or `us`")]
    InvalidUnit(String),
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, proptest};

    use super::*;

    mod to_string {
        use super::*;

        #[test]
        fn hours() {
            let test = Duration::from_secs(3 * SECONDS_PER_HOUR);
            assert_eq!(to_string(test), "3h");
        }

        #[test]
        fn minutes() {
            let test = Duration::from_secs(3 * SECONDS_PER_MINUTE);
            assert_eq!(to_string(test), "3m");
        }

        #[test]
        fn seconds() {
            let test = Duration::from_secs(3);
            assert_eq!(to_string(test), "3s");
        }

        #[test]
        fn milliseconds() {
            let test = Duration::from_millis(3);
            assert_eq!(to_string(test), "3ms");
        }

        #[test]
        fn microseconds() {
            let test = Duration::from_micros(3);
            assert_eq!(to_string(test), "3us");
        }

        #[test]
        fn combination() {
            let test = Duration::from_secs((3 * SECONDS_PER_HOUR) + (3 * SECONDS_PER_MINUTE) + 3);
            assert_eq!(to_string(test), "3h3m3s");
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn hours() {
            let test = Duration::from_secs(3 * SECONDS_PER_HOUR);
            assert_eq!(parse("3h").unwrap(), test);
        }

        #[test]
        fn minutes() {
            let test = Duration::from_secs(3 * SECONDS_PER_MINUTE);
            assert_eq!(parse("3m").unwrap(), test);
        }

        #[test]
        fn seconds() {
            let test = Duration::from_secs(3);
            assert_eq!(parse("3s").unwrap(), test);
        }

        #[test]
        fn milliseconds() {
            let test = Duration::from_millis(3);
            assert_eq!(parse("3ms").unwrap(), test);
        }

        #[test]
        fn microseconds() {
            let test = Duration::from_micros(3);
            assert_eq!(parse("3us").unwrap(), test);
        }

        #[test]
        fn combination() {
            let test = Duration::from_secs((3 * SECONDS_PER_HOUR) + (3 * SECONDS_PER_MINUTE) + 3);
            assert_eq!(parse("3h3m3s").unwrap(), test);
        }

        #[test]
        fn missing_unit_err() {
            assert_eq!(
                parse("42").unwrap_err(),
                ParseDurationError::MissingUnit(String::from("42")),
            );
        }

        #[test]
        fn no_digits_err() {
            assert_eq!(
                parse(" ").unwrap_err(),
                ParseDurationError::NoDigits(String::from(" ")),
            );
        }
    }

    proptest! {
        /// Test [`to_string()`] doesn't panic or error.
        #[test]
        fn to_string_no_panic(duration: Duration) {
            let _ = to_string(duration);
        }

        /// Test [`parse()`] doesn't panic.
        #[test]
        fn parse_no_panic(duration: String) {
            let _ = parse(&duration);
        }

        /// Test round tripping [`to_string()`] and [`parse()`] works.
        #[test]
        fn round_trip(secs: u64, micros in ..=(u32::MAX / 1000)) {
            let test = Duration::new(secs, micros * 1000);
            let test2 = parse(&to_string(test))?;
            prop_assert_eq!(test, test2);
        }
    }
}
