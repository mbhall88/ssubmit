use clap::{Parser};
use regex::Regex;

use ssubmit::{Memory, SlurmTime};

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
    /// Specify the real memory required per node. e.g., 4.3kb, 7G, 9000, 4.1MB
    ///
    /// Note, floating point numbers will be rounded up. e.g., 10.1G will request 11G.
    /// This is because sbatch only allows integers. See `man sbatch | grep -A 4 'mem='`
    /// for the full details.
    #[arg(short, long = "mem", value_name = "size[units]", default_value = "1G")]
    pub memory: Memory,
    /// Time limit for the job. e.g. 5d, 10h, 45m21s (case insensitive)
    ///
    /// Run `man sbatch | grep -A 7 'time=<'` for more details. If a single digit is passed, it will
    /// be passed straight to sbatch (i.e. minutes). However, 5m5 will be considered 5 minutes and
    /// 5 seconds.
    #[arg(short, long, value_parser = parse_time, default_value = "1w")]
    pub time: String,
    /// The shell shebang for the submission script
    #[arg(short = 'S', long, default_value = "#!/usr/bin/env bash")]
    pub shebang: String,
    /// Options for the set command in the shell script
    ///
    /// For example, to exit when the command exits with a non-zero code and to treat unset
    /// variables as an error during substitution, pass 'eu'. Pass '' or "" to set nothing
    #[arg(short, long, default_value = "euxo pipefail", allow_hyphen_values = true)]
    pub set: String,
    /// Print the sbatch command and submission script would be executed, but do not execute them
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    /// Return an estimate of when the job would be scheduled to run given the current
    /// queue. No job is actually submitted. [sbatch --test-only]
    #[arg(short = 'T', long)]
    pub test_only: bool,
}

fn parse_time(s: &str) -> Result<String, String> {
    let slurm_time_re = Regex::new(r"^(?:(?P<days>\d+)-)?(?:(?P<hours>\d+):)?(?:(?P<minutes>\d+):)?(?P<seconds>\d+)?$").map_err(|e| e.to_string())?;

    if slurm_time_re.is_match(s) {
        return Ok(s.to_string());
    }

    match duration_str::parse(s) {
        Ok(dur) => Ok(dur.to_slurm_time()),
        Err(e) => Err(format!("{s} is not a valid time: {e}")),
    }
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
}
