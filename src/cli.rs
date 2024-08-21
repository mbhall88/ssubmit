use byte_unit::{Byte, Unit};
use clap::Parser;
use regex::Regex;

use ssubmit::SlurmTime;

const SSUBMIT_SHEBANG: &str = "SSUBMIT_SHEBANG";
const SSUBMIT_MEMORY: &str = "SSUBMIT_MEMORY";
const SSUUBMIT_TIME: &str = "SSUBMIT_TIME";
const SSUBMIT_SET: &str = "SSUBMIT_SET";

/// Submit sbatch jobs without having to create a submission script
///
/// -----------
/// # EXAMPLES
/// -----------
///
/// Submit a simple rsync command with a 600MB memory limit.
///
/// $ ssubmit -m 600m rsync_my_data "rsync -az src/ dest/"
///
/// Submit a command that involves piping the output into another command. sbatch options
/// are passed after a `--`.
///
/// $ ssubmit -m 4G align "minimap2 -t 8 ref.fa reads.fq | samtools sort -o sorted.bam" -- -c 8
#[derive(Parser, Debug)]
#[clap(author, version, about, verbatim_doc_comment)]
pub struct Cli {
    /// Name of the job
    ///
    /// See `man sbatch | grep -A 2 'job-name='` for more details.
    pub name: String,
    /// Command to be executed by the job
    pub command: String,
    /// Options to be passed on to sbatch
    #[arg(raw = true, last = true, allow_hyphen_values = true)]
    pub remainder: Vec<String>,
    /// File to write job stdout to. (See `man sbatch | grep -A 3 'output='`)
    ///
    /// Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.
    #[arg(short, long, default_value = "%x.out")]
    pub output: String,
    /// File to write job stderr to. (See `man sbatch | grep -A 3 'error='`)
    ///
    /// Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.
    #[arg(short, long, default_value = "%x.err")]
    pub error: String,
    /// Specify the real memory required per node. e.g., 4.3kb, 7 Gb, 9000, 4.1MB become 5KB, 7000M,
    /// 9000M, and 5M, respectively.
    ///
    /// If no unit is specified, megabytes will be used, as per the sbatch default. The value will
    /// be rounded up to the nearest megabyte. If the value is less than 1M, it will be rounded up
    /// to the nearest kilobyte.
    /// See `man sbatch | grep -A 4 'mem='` for the full details.
    #[arg(short, long = "mem", value_name = "size[unit]", default_value = "1G", value_parser = parse_memory, env = SSUBMIT_MEMORY)]
    pub memory: String,
    /// Time limit for the job. e.g. 5d, 10h, 45m21s (case-insensitive)
    ///
    /// Run `man sbatch | grep -A 7 'time=<'` for more details. If a single digit is passed, it will
    /// be passed straight to sbatch (i.e. minutes). However, 5m5 will be considered 5 minutes and
    /// 5 seconds.
    #[arg(short, long, value_parser = parse_time, default_value = "1d", env = SSUUBMIT_TIME)]
    pub time: String,
    /// The shell shebang for the submission script
    #[arg(short = 'S', long, default_value = "#!/usr/bin/env bash", env = SSUBMIT_SHEBANG)]
    pub shebang: String,
    /// Options for the set command in the shell script
    ///
    /// For example, to exit when the command exits with a non-zero code and to treat unset
    /// variables as an error during substitution, pass 'eu'. Pass '' or "" to set nothing
    #[arg(
        short,
        long,
        default_value = "euxo pipefail",
        allow_hyphen_values = true,
        env = SSUBMIT_SET
    )]
    pub set: String,
    /// Print the sbatch command and submission script that would be executed, but do not execute them
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    /// Return an estimate of when the job would be scheduled to run given the current
    /// queue. No job is actually submitted. [sbatch --test-only]
    #[arg(short = 'T', long)]
    pub test_only: bool,
}

/// Parse a time string into a slurm time format
///
/// # Examples
///
/// ```
/// use ssubmit::parse_time;
///
/// let s = "5m3s";
/// let actual = parse_time(s).unwrap();
/// let expected = "5:3";
/// assert_eq!(actual, expected)
/// ```
fn parse_time(s: &str) -> Result<String, String> {
    let slurm_time_re = Regex::new(
        r"^(?:(?P<days>\d+)-)?(?:(?P<hours>\d+):)?(?:(?P<minutes>\d+):)?(?P<seconds>\d+)?$",
    )
    .map_err(|e| e.to_string())?;

    if slurm_time_re.is_match(s) {
        return Ok(s.to_string());
    }

    match duration_str::parse(s) {
        Ok(dur) => Ok(dur.to_slurm_time()),
        Err(e) => Err(format!("{s} is not a valid time: {e}")),
    }
}

/// Parse a memory size string into a slurm memory format
///
/// # Examples
///
/// ```
/// use ssubmit::parse_memory;
///
/// let s = "4mb";
/// let actual = parse_memory(s).unwrap();
/// let expected = "4M";
/// assert_eq!(actual, expected)
/// ```
fn parse_memory(s: &str) -> Result<String, String> {
    if s == "0" {
        return Ok(s.to_string());
    }

    let s = if s.chars().all(|c| !c.is_ascii_alphabetic()) {
        format!("{}M", s)
    } else {
        s.to_string()
    };

    let ignore_case = true;
    let bytes = Byte::parse_str(s, ignore_case).map_err(|e| e.to_string())?;
    let mb = bytes.get_adjusted_unit(Unit::MB);
    let (value, unit) = match mb.get_value() {
        f if f < 1.0 => {
            let kb = bytes.get_adjusted_unit(Unit::KB);
            (kb.get_value(), "K")
        }
        f => (f, "M"),
    };
    // round up value to the nearest integer
    let value = value.ceil() as u64;
    Ok(format!("{}{}", value, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_milliseconds() {
        let s = "4ms";

        let actual = parse_time(s).unwrap();
        let expected = "0:1";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_seconds() {
        let s = "4s";

        let actual = parse_time(s).unwrap();
        let expected = "0:4";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_minutes() {
        let s = "4m";

        let actual = parse_time(s).unwrap();
        let expected = "4:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_hours_in_minutes() {
        let s = "400m";

        let actual = parse_time(s).unwrap();
        let expected = "6:40:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_hours() {
        let s = "3H";

        let actual = parse_time(s).unwrap();
        let expected = "3:0:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_hours_and_minutes() {
        let s = "3h46min";

        let actual = parse_time(s).unwrap();
        let expected = "3:46:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_hours_and_minutes_with_space() {
        let s = "3h 46min";

        let actual = parse_time(s).unwrap();
        let expected = "3:46:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_days_and_seconds() {
        let s = "1d4s";

        let actual = parse_time(s).unwrap();
        let expected = "24:0:4";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_slurm_minute_second_format_no_parsing() {
        let s = "3:45";

        let actual = parse_time(s).unwrap();
        let expected = "3:45";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_slurm_hours_minute_second_format_no_parsing() {
        let s = "1:3:45";

        let actual = parse_time(s).unwrap();
        let expected = "1:3:45";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_slurm_days_hours_format_no_parsing() {
        let s = "1-12";

        let actual = parse_time(s).unwrap();
        let expected = "1-12";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_slurm_days_hours_minutes_format_no_parsing() {
        let s = "1-12:30";

        let actual = parse_time(s).unwrap();
        let expected = "1-12:30";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_slurm_days_hours_minutes_seconds_format_no_parsing() {
        let s = "1-12:30:12";

        let actual = parse_time(s).unwrap();
        let expected = "1-12:30:12";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_no_units() {
        let s = "3";

        let actual = parse_time(s).unwrap();
        let expected = "3";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_zero() {
        let s = "0";

        let actual = parse_time(s).unwrap();
        let expected = "0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_time_float_not_supported() {
        let s = "1.5d";

        let actual = parse_time(s).unwrap_err();

        assert!(actual.starts_with("1.5d is not a valid time:"))
    }

    #[test]
    fn test_parse_time_missing_unit_is_seconds() {
        let s = "5m3";

        let actual = parse_time(s).unwrap();
        let expected = "5:3";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_parse_memory_kilobytes() {
        let s = "4kb";
        let actual = parse_memory(s).unwrap();
        let expected = "4K";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_kilobytes_over_megabyte_rounds() {
        let s = "4000kb";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_megabytes() {
        let s = "4mb";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_megabytes_single_m() {
        let s = "4m";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_megabytes_single_m_upper() {
        let s = "4M";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_megabytes_single_m_upper_space() {
        let s = "4 M";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_megabytes_one_upper_one_lower_space() {
        let s = "4 Mb";
        let actual = parse_memory(s).unwrap();
        let expected = "4M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_gigabytes() {
        let s = "5g";
        let actual = parse_memory(s).unwrap();
        let expected = "5000M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_terabytes() {
        let s = "1t";
        let actual = parse_memory(s).unwrap();
        let expected = "1000000M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_float() {
        let s = "1.5gb";
        let actual = parse_memory(s).unwrap();
        let expected = "1500M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_float_round_up() {
        let s = "50.7mb";
        let actual = parse_memory(s).unwrap();
        let expected = "51M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_float_round_up_if_below_half() {
        let s = "50.1mb";
        let actual = parse_memory(s).unwrap();
        let expected = "51M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_always_round_up() {
        let s = "5001kb";
        let actual = parse_memory(s).unwrap();
        let expected = "6M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_float_less_than_megabyte_returns_kilobyte() {
        let s = "0.56M";
        let actual = parse_memory(s).unwrap();
        let expected = "560K";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_no_unit() {
        let s = "5000";
        let actual = parse_memory(s).unwrap();
        let expected = "5000M";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_memory_less_than_kilobyte() {
        let s = "50b";
        let actual = parse_memory(s).unwrap();
        let expected = "1K";
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_memory_invalid_unit() {
        let s = "5z";
        let actual = parse_memory(s).unwrap_err();
        assert!(actual.starts_with("the character 'z' is incorrect"));
    }

    #[test]
    fn parse_memory_zero() {
        let s = "0";
        let actual = parse_memory(s).unwrap();
        let expected = "0";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cli_parse_remainder() {
        let args = Cli::parse_from(["ssubmit", "name", "command", "--", "-c", "8"]);

        let actual = args.remainder.join(" ");
        let expected = "-c 8";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cli_parse_set_shebang_with_environment_variable() {
        let shebang = "#!/bin/zsh";
        unsafe {
            std::env::set_var(SSUBMIT_SHEBANG, shebang);
        }

        let args = Cli::parse_from(["ssubmit", "name", "command"]);

        let actual = args.shebang;
        let expected = shebang;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cli_parse_set_memory_with_environment_variable() {
        let memory = "4M";
        unsafe {
            std::env::set_var(SSUBMIT_MEMORY, memory);
        }

        let args = Cli::parse_from(["ssubmit", "name", "command"]);

        let actual = args.memory;
        let expected = memory;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cli_parse_set_time_with_environment_variable() {
        let time = "1:0";
        unsafe {
            std::env::set_var(SSUUBMIT_TIME, time);
        }

        let args = Cli::parse_from(["ssubmit", "name", "command"]);

        let actual = args.time;
        let expected = time;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cli_parse_set_with_environment_variable() {
        let set = "eu";
        unsafe {
            std::env::set_var(SSUBMIT_SET, set);
        }

        let args = Cli::parse_from(["ssubmit", "name", "command"]);

        let actual = args.set;
        let expected = set;
        assert_eq!(actual, expected);
    }
}
