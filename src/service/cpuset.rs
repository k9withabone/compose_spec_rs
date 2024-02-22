//! Provides [`CpuSet`] for the `cpuset` field of [`Service`](super::Service).

use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter, Write},
    num::ParseIntError,
    str::FromStr,
};

use compose_spec_macros::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

/// CPUs in which to allow execution.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#cpuset)
#[derive(SerializeDisplay, DeserializeFromStr, Debug, Default, Clone, PartialEq, Eq)]
#[serde(expecting = "a comma-separated list (0,1), a range (0-3), or a combination (0-3,5,7-9)")]
pub struct CpuSet(pub BTreeSet<u64>);

impl CpuSet {
    /// Returns `true` if the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for CpuSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut iter = self.0.iter();

        let Some(cpu) = iter.next() else {
            return f.write_str("");
        };

        let mut range = (*cpu, *cpu);

        let mut first = true;
        for cpu in iter {
            let (start, end) = &mut range;
            if *cpu == *end + 1 {
                *end = *cpu;
            } else {
                write_range(f, first, *start, *end)?;
                first = false;
                range = (*cpu, *cpu);
            }
        }

        let (start, end) = range;
        write_range(f, first, start, end)
    }
}

/// Write range to a [`Formatter`].
fn write_range(f: &mut Formatter, first: bool, start: u64, end: u64) -> fmt::Result {
    if !first {
        f.write_char(',')?;
    }

    let mut buffer = itoa::Buffer::new();

    f.write_str(buffer.format(start))?;

    if start != end {
        f.write_char('-')?;
        f.write_str(buffer.format(end))?;
    }

    Ok(())
}

impl FromStr for CpuSet {
    type Err = ParseCpuSetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut inner = BTreeSet::new();

        for range in s.split_terminator(',') {
            if let Some((start, end)) = range.split_once('-') {
                let start: u64 = start.parse().map_err(parse_int_err(start))?;
                let end = end.parse().map_err(parse_int_err(end))?;
                inner.extend(start..=end);
            } else {
                let cpu = range.parse().map_err(parse_int_err(range))?;
                inner.insert(cpu);
            }
        }

        Ok(Self(inner))
    }
}

/// Closure which constructs a [`ParseCpuSetError`] from a [`ParseIntError`] and a `value`.
fn parse_int_err(value: &str) -> impl FnOnce(ParseIntError) -> ParseCpuSetError {
    let value = value.to_owned();
    |source| ParseCpuSetError { value, source }
}

/// Error returned when parsing a [`CpuSet`] from a string.
#[derive(Error, Debug)]
#[error("could not parse `{value}` as an integer")]
pub struct ParseCpuSetError {
    /// Value attempted to parse.
    value: String,
    /// Parse error.
    source: ParseIntError,
}

impl TryFrom<&str> for CpuSet {
    type Error = ParseCpuSetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BTreeSet<u64>> for CpuSet {
    fn from(value: BTreeSet<u64>) -> Self {
        Self(value)
    }
}

impl From<CpuSet> for BTreeSet<u64> {
    fn from(value: CpuSet) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, proptest};

    use super::*;

    mod display {
        use super::*;

        #[test]
        fn individual() {
            let test = CpuSet(BTreeSet::from([1, 3, 5]));
            assert_eq!(test.to_string(), "1,3,5");
        }

        #[test]
        fn range() {
            let test = CpuSet(BTreeSet::from([1, 2, 3]));
            assert_eq!(test.to_string(), "1-3");
        }

        #[test]
        fn combination() {
            let test = CpuSet(BTreeSet::from([1, 2, 3, 5, 7, 8, 9]));
            assert_eq!(test.to_string(), "1-3,5,7-9");
        }
    }

    mod from_str {
        use super::*;

        #[test]
        fn individual() {
            let test = CpuSet(BTreeSet::from([1, 3, 5]));
            assert_eq!(test, "1,3,5".parse().unwrap());
        }

        #[test]
        fn range() {
            let test = CpuSet(BTreeSet::from([1, 2, 3]));
            assert_eq!(test, "1-3".parse().unwrap());
        }

        #[test]
        fn combination() {
            let test = CpuSet(BTreeSet::from([1, 2, 3, 5, 7, 8, 9]));
            assert_eq!(test, "1-3,5,7-9".parse().unwrap());
        }
    }

    proptest! {
        #[test]
        fn to_string_no_panic(set: BTreeSet<u64>) {
            CpuSet(set).to_string();
        }

        #[test]
        fn parse_no_panic(string: String) {
            let _ = string.parse::<CpuSet>();
        }

        #[test]
        fn round_trip(set: BTreeSet<u64>) {
            let test = CpuSet(set);
            let test2 = test.to_string().parse().unwrap();
            prop_assert_eq!(test, test2);
        }
    }
}
