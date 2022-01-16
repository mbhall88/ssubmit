use std::cmp::max;
use std::fmt;
use std::ops::{Div, Mul};
use std::str::FromStr;
use std::time::Duration;

use log::warn;
use regex::Regex;
use thiserror::Error;

static PREFIXES: &[MetricSuffix] = &[
    MetricSuffix::Kilo,
    MetricSuffix::Mega,
    MetricSuffix::Giga,
    MetricSuffix::Tera,
];
static KILO: f64 = 1000.0;

pub fn format_number(amount: u64) -> String {
    let mut value = amount as f64;
    let mut prefix = 0;
    while value >= KILO && prefix < PREFIXES.len() {
        value /= KILO;
        prefix += 1;
    }
    if prefix == 0 {
        warn!("Memory provided was less than 1KB; defaulting to 1KB...");
        prefix = 1;
        value = 1.0;
    }
    format!("{:.0}{}", value.ceil(), PREFIXES[prefix - 1])
}
/// A metric suffix is a unit suffix used to indicate the multiples of (in this case) base pairs.
/// For instance, the metric suffix 'Kb' refers to kilobases. Therefore, 6.9kb means 6900 base pairs.
#[derive(PartialEq, Debug)]
pub enum MetricSuffix {
    Base,
    Kilo,
    Mega,
    Giga,
    Tera,
}

impl FromStr for MetricSuffix {
    type Err = CliError;

    /// Parses a string into a `MetricSuffix`.
    fn from_str(suffix: &str) -> Result<Self, Self::Err> {
        let suffix_lwr = suffix.to_lowercase();
        let metric_suffix = match suffix_lwr.as_str() {
            s if "b".contains(s) => MetricSuffix::Base,
            s if "kb".contains(s) => MetricSuffix::Kilo,
            s if "mb".contains(s) => MetricSuffix::Mega,
            s if "gb".contains(s) => MetricSuffix::Giga,
            s if "tb".contains(s) => MetricSuffix::Tera,
            _ => {
                return Err(CliError::InvalidMetricSuffix(suffix.to_string()));
            }
        };
        Ok(metric_suffix)
    }
}

impl fmt::Display for MetricSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sfx = match self {
            MetricSuffix::Base => "B",
            MetricSuffix::Kilo => "K",
            MetricSuffix::Mega => "M",
            MetricSuffix::Giga => "G",
            MetricSuffix::Tera => "T",
        };
        write!(f, "{}", sfx)
    }
}

/// Allow for multiplying a `f64` by a `MetricSuffix`.
impl Mul<MetricSuffix> for f64 {
    type Output = Self;

    fn mul(self, rhs: MetricSuffix) -> Self::Output {
        match rhs {
            MetricSuffix::Base => self,
            MetricSuffix::Kilo => self * 1_000.0,
            MetricSuffix::Mega => self * 1_000_000.0,
            MetricSuffix::Giga => self * 1_000_000_000.0,
            MetricSuffix::Tera => self * 1_000_000_000_000.0,
        }
    }
}

/// A collection of custom errors relating to the command line interface for this package.
#[derive(Error, Debug, PartialEq)]
pub enum CliError {
    /// Indicates that a string cannot be parsed into a [`MetricSuffix`](#metricsuffix).
    #[error("{0} is not a valid metric suffix")]
    InvalidMetricSuffix(String),

    /// Indicates that a string cannot be parsed into a [`Memory`](#genomesize).
    #[error("{0} is not a valid genome size. Valid forms include 4gb, 3000, 8.7Kb etc.")]
    InvalidMemoryString(String),
}

/// An object for collecting together methods for working with the genome size parameter for this
/// package.
#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub struct Memory(pub u64);

/// Allow for comparison of a `u64` and a `Memory`.
impl PartialEq<u64> for Memory {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl From<Memory> for u64 {
    fn from(g: Memory) -> Self {
        g.0
    }
}

impl FromStr for Memory {
    type Err = CliError;

    /// Parses a string into a `Memory`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.to_lowercase();
        let re = Regex::new(r"(?P<size>[0-9]*\.?[0-9]+)(?P<sfx>\w*)$").unwrap();
        let captures = match re.captures(text.as_str()) {
            Some(cap) => cap,
            None => return Err(CliError::InvalidMemoryString(s.to_string())),
        };
        let size = captures
            .name("size")
            .unwrap()
            .as_str()
            .parse::<f64>()
            .unwrap();
        let metric_suffix = MetricSuffix::from_str(captures.name("sfx").unwrap().as_str())?;

        Ok(Memory((size * metric_suffix) as u64))
    }
}

/// Allow for dividing a `u64` by a `Memory`.
impl Div<Memory> for u64 {
    type Output = f64;

    fn div(self, rhs: Memory) -> Self::Output {
        (self as f64) / (rhs.0 as f64)
    }
}

pub trait SlurmTime {
    fn to_slurm_time(&self) -> String;
}

impl SlurmTime for Duration {
    fn to_slurm_time(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }

        let mut remainder = max(self.as_secs(), 1);

        if remainder < 60 {
            // less than a minute
            return format!("0:{}", remainder);
        }

        let secs = remainder % 60;
        remainder /= 60;

        if remainder < 60 {
            // less than an hour
            return format!("{}:{}", remainder, secs);
        }

        let mins = remainder % 60;
        remainder /= 60;

        format!("{}:{}:{}", remainder, mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_to_slurm_zero() {
        let one_milli = Duration::from_millis(0);

        let actual = one_milli.to_slurm_time();
        let expected = "0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_second() {
        let one_milli = Duration::from_millis(6);

        let actual = one_milli.to_slurm_time();
        let expected = "0:1";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_minute() {
        let secs = Duration::from_secs(6);

        let actual = secs.to_slurm_time();
        let expected = "0:6";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_hour() {
        let secs = Duration::from_secs(64);

        let actual = secs.to_slurm_time();
        let expected = "1:4";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_minutes_with_remainder() {
        let secs = Duration::from_secs(666);

        let actual = secs.to_slurm_time();
        let expected = "11:6";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_even_minutes() {
        let secs = Duration::from_secs(60);

        let actual = secs.to_slurm_time();
        let expected = "1:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_even_hours() {
        let secs = Duration::from_secs(60 * 60 * 4);

        let actual = secs.to_slurm_time();
        let expected = "4:0:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_hours_with_remainder() {
        let secs = Duration::from_secs(9042);

        let actual = secs.to_slurm_time();
        let expected = "2:30:42";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_over_a_day() {
        let secs = Duration::from_secs(561677);

        let actual = secs.to_slurm_time();
        let expected = "156:1:17";

        assert_eq!(actual, expected)
    }

    #[test]
    fn integer_only_returns_integer() {
        let actual = Memory::from_str("6").unwrap();
        let expected = 6;

        assert_eq!(actual, expected);
    }

    #[test]
    fn float_only_returns_integer() {
        let actual = Memory::from_str("6.5").unwrap();
        let expected = 6;

        assert_eq!(actual, expected);
    }

    #[test]
    fn int_and_suffix_returns_multiplied_int() {
        let actual = Memory::from_str("5mb").unwrap();
        let expected = 5_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn float_and_suffix_returns_multiplied_float_as_int() {
        let actual = Memory::from_str("5.4kB").unwrap();
        let expected = 5_400;

        assert_eq!(actual, expected);
    }

    #[test]
    fn float_without_leading_int_and_suffix_returns_multiplied_float_as_int() {
        let actual = Memory::from_str(".77G").unwrap();
        let expected = 770_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn int_and_tera_suffix_returns_multiplied_int() {
        let actual = Memory::from_str("7TB").unwrap();
        let expected = 7_000_000_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn int_and_base_suffix_returns_int_without_scaling() {
        let actual = Memory::from_str("7B").unwrap();
        let expected = 7;

        assert_eq!(actual, expected);
    }

    #[test]
    fn invalid_suffix_returns_err() {
        let genome_size = String::from(".77uB");
        let actual = Memory::from_str(genome_size.as_str()).unwrap_err();
        let expected = CliError::InvalidMetricSuffix(String::from("ub"));

        assert_eq!(actual, expected);
    }

    #[test]
    fn empty_string_returns_error() {
        let actual = Memory::from_str("").unwrap_err();
        let expected = CliError::InvalidMemoryString(String::from(""));

        assert_eq!(actual, expected);
    }

    #[test]
    fn suffix_with_no_size_returns_error() {
        let actual = Memory::from_str("gb");

        assert!(actual.is_err());
    }

    #[test]
    fn metric_suffix_from_lower() {
        let sfx = "g";

        assert_eq!(MetricSuffix::from_str(sfx).unwrap(), MetricSuffix::Giga)
    }

    #[test]
    fn metric_suffix_from_upper() {
        let sfx = "KB";

        assert_eq!(MetricSuffix::from_str(sfx).unwrap(), MetricSuffix::Kilo)
    }

    #[test]
    fn metric_suffix_to_string() {
        let s = MetricSuffix::Tera;

        assert_eq!(s.to_string(), "T".to_string())
    }

    #[test]
    fn format_number_giga() {
        let number = Memory::from_str("1G").unwrap().0;

        assert_eq!(number, 1_000_000_000);

        assert_eq!(format_number(number), "1G")
    }

    #[test]
    fn format_number_less_than_kilo() {
        let number = Memory::from_str("16.7").unwrap().0;

        assert_eq!(number, 16);

        assert_eq!(format_number(number), "1K")
    }

    #[test]
    fn format_number_float_converts_down() {
        let number = Memory::from_str("0.56m").unwrap().0;

        assert_eq!(number, 560_000);

        assert_eq!(format_number(number), "560K")
    }

    #[test]
    fn format_number_converts_up() {
        let number = Memory::from_str("5000kb").unwrap().0;

        assert_eq!(number, 5_000_000);

        assert_eq!(format_number(number), "5M")
    }
    #[test]
    fn format_number_limited_precision_when_rounds_up() {
        let number = Memory::from_str("5001kb").unwrap().0;

        assert_eq!(number, 5_001_000);

        assert_eq!(format_number(number), "6M")
    }

    #[test]
    fn format_number_limited_precision_when_converting_down() {
        let number = Memory::from_str("50.7M").unwrap().0;

        assert_eq!(number, 50_700_000);

        assert_eq!(format_number(number), "51M")
    }
}
