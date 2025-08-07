use byte_unit::{Byte, Unit};
use clap::Parser;
use log::info;
use regex::Regex;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

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
///
/// Start an interactive session with 5GB memory for 8 hours.
///
/// $ ssubmit --interactive -m 5G -t 8h interactiveJob
///
/// Start an interactive session with custom shell and additional SLURM options.
///
/// $ ssubmit --interactive -m 16G -t 4h DevSession --shell bash -- --partition=general --qos=normal
#[derive(Parser, Debug)]
#[clap(author, version, about, verbatim_doc_comment)]
pub struct Cli {
    /// Name of the job
    ///
    /// See `man sbatch | grep -A 2 'job-name='` for more details.
    pub name: String,
    /// Command to be executed by the job
    ///
    /// For batch jobs, this is required. For interactive jobs (--interactive),
    /// this is optional and defaults to starting a shell session.
    pub command: Option<String>,
    /// Options to be passed on to sbatch or salloc (for interactive jobs)
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
    /// Request an interactive job session instead of a batch job
    ///
    /// This will use `salloc` instead of `sbatch` and automatically start an interactive shell.
    /// The command argument becomes optional and defaults to the user's shell.
    #[arg(short = 'i', long)]
    pub interactive: bool,
    /// Shell to use for interactive sessions
    ///
    /// Only used when --interactive is specified. Defaults to the user's login shell.
    #[arg(long, default_value = "auto")]
    pub shell: String,
    /// Control which environment variables are exported to the job
    ///
    /// Passed directly to sbatch as --export=<value>. Use 'NONE' to export no variables,
    /// 'ALL' to export all variables, or specify specific variables like 'PATH,HOME'.
    #[arg(long, default_value = "ALL")]
    pub export: String,
}

/// Try to get shell path using 'which' command
fn get_shell_path_via_which(shell: &str) -> Option<String> {
    std::process::Command::new("which")
        .arg(shell)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
}

/// Try to detect shell using parent process information
fn get_shell_from_parent_process() -> Option<String> {
    let system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

    let my_pid = sysinfo::get_current_pid().ok()?;
    let current_process = system.process(my_pid)?;
    let parent_pid = current_process.parent()?;
    let parent_process = system.process(parent_pid)?;
    let parent_name = parent_process.name();

    // Check if it's a known shell and try to get its full path
    let shell_names = ["zsh", "bash", "fish", "tcsh", "csh", "sh"];
    for shell in &shell_names {
        if parent_name.eq_ignore_ascii_case(shell)
            || parent_name.starts_with(shell)
            || parent_name.contains(shell)
        {
            // Try to get the full path first
            if let Some(full_path) = get_shell_path_via_which(shell) {
                return Some(full_path);
            }

            // Fallback to shell name if we can't get full path
            return Some(shell.to_string());
        }
    }

    None
}

/// Try to get shell from SHELL environment variable
fn get_shell_from_env() -> Option<String> {
    std::env::var("SHELL")
        .ok()
        .filter(|shell_path| !shell_path.is_empty())
}

/// Try to find common shells using 'which'
fn find_available_shell() -> Option<String> {
    let common_shells = ["zsh", "bash", "sh"];
    for shell in &common_shells {
        if let Some(full_path) = get_shell_path_via_which(shell) {
            return Some(full_path);
        }
    }
    None
}

/// Get the user's current shell using parent process detection
/// Returns the full path when possible, otherwise falls back to shell name
fn get_user_shell() -> String {
    // Method 1: Try to detect shell from parent process (most direct)
    if let Some(shell) = get_shell_from_parent_process() {
        return shell;
    }

    // Method 2: Try SHELL environment variable as fallback (reliable for full path)
    if let Some(shell) = get_shell_from_env() {
        return shell;
    }

    // Method 3: Try to detect common shells with full paths
    if let Some(shell) = find_available_shell() {
        return shell;
    }

    // Final fallback: try to get bash path, or just return "bash"
    get_shell_path_via_which("bash").unwrap_or_else(|| "bash".to_string())
}

impl Cli {
    /// Validate the arguments and return the command to execute
    pub fn validate_and_get_command(&self) -> Result<String, String> {
        if self.interactive {
            // For interactive jobs, command is optional and defaults to shell
            Ok(self.command.clone().unwrap_or_else(|| {
                let shell = if self.shell == "auto" {
                    let sh = get_user_shell();
                    info!("Inferred shell for interactive session is {sh}");
                    sh
                } else {
                    self.shell.clone()
                };
                format!("srun --pty {shell} -l")
            }))
        } else {
            // For batch jobs, command is required
            self.command.clone().ok_or_else(|| {
                "Command is required for batch jobs. Use --interactive for interactive sessions.".to_string()
            })
        }
    }
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
        format!("{s}M")
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
    Ok(format!("{value}{unit}"))
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

    #[test]
    fn test_get_user_shell_returns_path() {
        // This test verifies that the function returns a valid shell (path or name)
        let shell = get_user_shell();

        // Should return either a full path or a known shell name
        let valid_shells = ["zsh", "bash", "fish", "tcsh", "csh", "sh"];
        let is_valid = shell.starts_with('/') || valid_shells.contains(&shell.as_str());

        assert!(
            is_valid,
            "Expected a valid shell path or name, got: {shell}"
        );
    }

    #[test]
    fn test_get_user_shell_not_empty() {
        let shell = get_user_shell();
        assert!(!shell.is_empty(), "Shell name should not be empty");
    }

    #[test]
    fn test_validate_and_get_command_interactive_no_command() {
        let cli = Cli {
            name: "test".to_string(),
            command: None,
            remainder: vec![],
            output: "%x.out".to_string(),
            error: "%x.err".to_string(),
            memory: "1G".to_string(),
            time: "1d".to_string(),
            shebang: "#!/usr/bin/env bash".to_string(),
            set: "euxo pipefail".to_string(),
            dry_run: false,
            test_only: false,
            interactive: true,
            shell: "zsh".to_string(),
            export: "ALL".to_string(),
        };

        let result = cli.validate_and_get_command().unwrap();
        assert_eq!(result, "srun --pty zsh -l");
    }

    #[test]
    fn test_validate_and_get_command_interactive_with_command() {
        let cli = Cli {
            name: "test".to_string(),
            command: Some("custom command".to_string()),
            remainder: vec![],
            output: "%x.out".to_string(),
            error: "%x.err".to_string(),
            memory: "1G".to_string(),
            time: "1d".to_string(),
            shebang: "#!/usr/bin/env bash".to_string(),
            set: "euxo pipefail".to_string(),
            dry_run: false,
            test_only: false,
            interactive: true,
            shell: "bash".to_string(),
            export: "ALL".to_string(),
        };

        let result = cli.validate_and_get_command().unwrap();
        assert_eq!(result, "custom command");
    }

    #[test]
    fn test_validate_and_get_command_batch_no_command() {
        let cli = Cli {
            name: "test".to_string(),
            command: None,
            remainder: vec![],
            output: "%x.out".to_string(),
            error: "%x.err".to_string(),
            memory: "1G".to_string(),
            time: "1d".to_string(),
            shebang: "#!/usr/bin/env bash".to_string(),
            set: "euxo pipefail".to_string(),
            dry_run: false,
            test_only: false,
            interactive: false,
            shell: "bash".to_string(),
            export: "ALL".to_string(),
        };

        let result = cli.validate_and_get_command();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Command is required for batch jobs. Use --interactive for interactive sessions."
        );
    }

    #[test]
    fn test_validate_and_get_command_batch_with_command() {
        let cli = Cli {
            name: "test".to_string(),
            command: Some("batch command".to_string()),
            remainder: vec![],
            output: "%x.out".to_string(),
            error: "%x.err".to_string(),
            memory: "1G".to_string(),
            time: "1d".to_string(),
            shebang: "#!/usr/bin/env bash".to_string(),
            set: "euxo pipefail".to_string(),
            dry_run: false,
            test_only: false,
            interactive: false,
            shell: "bash".to_string(),
            export: "ALL".to_string(),
        };

        let result = cli.validate_and_get_command().unwrap();
        assert_eq!(result, "batch command");
    }

    #[test]
    fn test_export_default_value() {
        let args = Cli::parse_from(["ssubmit", "test_job", "echo hello"]);
        assert_eq!(args.export, "ALL");
    }

    #[test]
    fn test_export_custom_value() {
        let args = Cli::parse_from(["ssubmit", "--export", "NONE", "test_job", "echo hello"]);
        assert_eq!(args.export, "NONE");
    }

    #[test]
    fn test_export_specific_variables() {
        let args = Cli::parse_from([
            "ssubmit",
            "--export",
            "PATH,HOME,USER",
            "test_job",
            "echo hello",
        ]);
        assert_eq!(args.export, "PATH,HOME,USER");
    }
}
