use std::cmp::max;
use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration; // import without risk of name clashing

use log::warn;
use serde::Serialize;

pub const JSON_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobSpec {
    pub name: String,
    pub command: String,
    pub memory: String,
    pub time: String,
    pub output: String,
    pub error: String,
    pub export: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlurmPlan {
    pub executable: String,
    pub arguments: Vec<String>,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubmissionPlan {
    pub job: JobSpec,
    pub slurm: SlurmPlan,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubmissionResult {
    pub job_id: String,
    pub cluster: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SchedulerTestResult {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionError {
    pub kind: String,
    pub message: String,
    pub exit_code: Option<i32>,
    pub stderr: Option<String>,
}

impl SubmissionError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: "validation".to_string(),
            message: message.into(),
            exit_code: None,
            stderr: None,
        }
    }

    pub fn process(message: impl Into<String>, stderr: Option<String>) -> Self {
        Self {
            kind: "process".to_string(),
            message: message.into(),
            exit_code: None,
            stderr,
        }
    }

    pub fn slurm(exit_code: i32, stderr: Option<String>) -> Self {
        Self {
            kind: "slurm".to_string(),
            message: format!("Failed to submit job with exit code {exit_code}"),
            exit_code: Some(exit_code),
            stderr,
        }
    }

    pub fn output(message: impl Into<String>, stderr: Option<String>) -> Self {
        Self {
            kind: "output".to_string(),
            message: message.into(),
            exit_code: None,
            stderr,
        }
    }
}

#[derive(Debug)]
pub struct SbatchOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonResponse {
    pub schema_version: u8,
    pub operation: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<SubmissionPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission: Option<SubmissionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<SchedulerTestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
}

impl JsonResponse {
    pub fn plan(plan: SubmissionPlan) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "plan".to_string(),
            ok: true,
            plan: Some(plan),
            submission: None,
            test: None,
            error: None,
        }
    }

    pub fn submission(plan: SubmissionPlan, submission: SubmissionResult) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "submit".to_string(),
            ok: true,
            plan: Some(plan),
            submission: Some(submission),
            test: None,
            error: None,
        }
    }

    pub fn scheduler_test(plan: SubmissionPlan, test: SchedulerTestResult) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "test".to_string(),
            ok: true,
            plan: Some(plan),
            submission: None,
            test: Some(test),
            error: None,
        }
    }

    pub fn error(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "plan".to_string(),
            ok: false,
            plan: None,
            submission: None,
            test: None,
            error: Some(JsonError {
                kind: kind.into(),
                message: message.into(),
                exit_code: None,
                stderr: None,
            }),
        }
    }

    pub fn submission_error(plan: SubmissionPlan, error: SubmissionError) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "submit".to_string(),
            ok: false,
            plan: Some(plan),
            submission: None,
            test: None,
            error: Some(JsonError {
                kind: error.kind,
                message: error.message,
                exit_code: error.exit_code,
                stderr: error.stderr,
            }),
        }
    }

    pub fn scheduler_test_error(plan: SubmissionPlan, error: SubmissionError) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "test".to_string(),
            ok: false,
            plan: Some(plan),
            submission: None,
            test: None,
            error: Some(JsonError {
                kind: error.kind,
                message: error.message,
                exit_code: error.exit_code,
                stderr: error.stderr,
            }),
        }
    }
}

static SCRIPT_TEMPLATE: &str = r#"$shebang$
#SBATCH --job-name=$name$
#SBATCH --mem=$memory$
#SBATCH --time=$time$
#SBATCH --error=$error$
#SBATCH --output=$output$
$set$

$cmd$
"#;

#[allow(clippy::too_many_arguments)]
pub fn make_submission_script(
    shebang: &str,
    set: &str,
    name: &str,
    memory: &str,
    time: &str,
    error: &str,
    output: &str,
    cmd: &str,
) -> String {
    let mut set_line = String::new();
    if !set.is_empty() {
        let _ = write!(set_line, "set -{set}");
    }
    let script = SCRIPT_TEMPLATE
        .replace("$shebang$", shebang)
        .replace("$name$", name)
        .replace("$memory$", memory)
        .replace("$time$", time)
        .replace("$error$", error)
        .replace("$output$", output)
        .replace("$cmd$", cmd)
        .replace("$set$", &set_line);

    if memory == "0" {
        warn!("Memory provided was 0; using cluster default. Use `scontrol show config | grep -i 'DefMem'` to check the default memory.");
        script
            .split_inclusive('\n')
            .filter(|line| !line.contains("--mem"))
            .collect::<Vec<&str>>()
            .concat()
    } else {
        script
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_submission_plan(
    shebang: &str,
    set: &str,
    name: &str,
    memory: &str,
    time: &str,
    error: &str,
    output: &str,
    command: &str,
    remainder: &[String],
    export: &str,
    test_only: bool,
) -> SubmissionPlan {
    let script = make_submission_script(shebang, set, name, memory, time, error, output, command);
    let effective_export = effective_export(remainder, export);

    let mut arguments = Vec::with_capacity(remainder.len() + usize::from(test_only));
    let mut test_only_seen = false;
    for argument in remainder {
        if is_test_only_argument(argument) {
            if test_only_seen {
                continue;
            }
            test_only_seen = true;
        }
        arguments.push(argument.clone());
    }

    if !arguments.iter().any(|arg| arg.starts_with("--export")) {
        arguments.push(format!("--export={export}"));
    }

    if test_only && !test_only_seen {
        arguments.push("--test-only".to_string());
    }

    SubmissionPlan {
        job: JobSpec {
            name: name.to_string(),
            command: command.to_string(),
            memory: memory.to_string(),
            time: time.to_string(),
            output: output.to_string(),
            error: error.to_string(),
            export: effective_export,
        },
        slurm: SlurmPlan {
            executable: "sbatch".to_string(),
            arguments,
            script,
        },
    }
}

pub fn prepare_machine_submission(
    plan: &SubmissionPlan,
) -> Result<SubmissionPlan, SubmissionError> {
    let mut arguments = Vec::with_capacity(plan.slurm.arguments.len() + 1);
    let mut parsable_seen = false;

    for argument in &plan.slurm.arguments {
        if is_quiet_argument(argument) {
            return Err(SubmissionError::validation(
                "JSON submission cannot use --quiet because it suppresses the job identifier",
            ));
        }

        if is_parsable_argument(argument) {
            if parsable_seen {
                continue;
            }
            parsable_seen = true;
        }

        arguments.push(argument.clone());
    }

    if !parsable_seen {
        arguments.push("--parsable".to_string());
    }

    let mut machine_plan = plan.clone();
    machine_plan.slurm.arguments = arguments;
    Ok(machine_plan)
}

pub fn prepare_machine_test(plan: &SubmissionPlan) -> Result<SubmissionPlan, SubmissionError> {
    if plan
        .slurm
        .arguments
        .iter()
        .any(|argument| is_quiet_argument(argument))
    {
        return Err(SubmissionError::validation(
            "JSON scheduler tests cannot use --quiet because it suppresses scheduler feedback",
        ));
    }

    Ok(plan.clone())
}

fn is_quiet_argument(argument: &str) -> bool {
    argument == "-Q" || argument == "--quiet" || argument.starts_with("--quiet=")
}

fn is_parsable_argument(argument: &str) -> bool {
    argument == "--parsable" || argument == "--parsable2"
}

fn is_test_only_argument(argument: &str) -> bool {
    argument == "--test-only"
}

pub fn run_sbatch(plan: &SubmissionPlan) -> Result<SbatchOutput, SubmissionError> {
    let mut child = Command::new(&plan.slurm.executable)
        .args(&plan.slurm.arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SubmissionError::process(format!("Failed to spawn sbatch process: {error}"), None)
        })?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            SubmissionError::process("Failed to connect to sbatch process stdin", None)
        })?;
        stdin
            .write_all(plan.slurm.script.as_bytes())
            .map_err(|error| {
                SubmissionError::process(
                    format!("Failed to write to sbatch process stdin: {error}"),
                    None,
                )
            })?;
    }

    let output = child.wait_with_output().map_err(|error| {
        SubmissionError::process(format!("Failed to execute sbatch process: {error}"), None)
    })?;

    Ok(SbatchOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub fn submit_sbatch(plan: &SubmissionPlan) -> Result<SubmissionResult, SubmissionError> {
    let output = run_sbatch(plan)?;
    if let Some(error) = classify_sbatch_failure(&output) {
        return Err(error);
    }

    parse_submission_output(&output.stdout, non_empty_trimmed(&output.stderr))
}

pub fn test_sbatch(plan: &SubmissionPlan) -> Result<SchedulerTestResult, SubmissionError> {
    let output = run_sbatch(plan)?;
    if let Some(error) = classify_scheduler_test_failure(&output) {
        return Err(error);
    }

    Ok(SchedulerTestResult {
        stdout: output.stdout.trim_end().to_string(),
        stderr: output.stderr.trim_end().to_string(),
    })
}

pub fn classify_sbatch_failure(output: &SbatchOutput) -> Option<SubmissionError> {
    let stderr = non_empty_trimmed(&output.stderr);
    match output.status.code() {
        Some(0) => None,
        Some(exit_code) => Some(SubmissionError::slurm(exit_code, stderr)),
        None => Some(SubmissionError::process(
            "sbatch process terminated by signal",
            stderr,
        )),
    }
}

pub fn classify_scheduler_test_failure(output: &SbatchOutput) -> Option<SubmissionError> {
    let stderr = non_empty_trimmed(&output.stderr);
    match output.status.code() {
        Some(0) => None,
        Some(exit_code) => Some(SubmissionError {
            kind: "slurm".to_string(),
            message: format!("Scheduler test failed with exit code {exit_code}"),
            exit_code: Some(exit_code),
            stderr,
        }),
        None => Some(SubmissionError::process(
            "Scheduler test process terminated by signal",
            stderr,
        )),
    }
}

pub fn parse_submission_output(
    stdout: &str,
    stderr: Option<String>,
) -> Result<SubmissionResult, SubmissionError> {
    let output = stdout.trim();
    if output.is_empty() {
        return Err(SubmissionError::output(
            "sbatch returned empty output",
            stderr,
        ));
    }

    if output.chars().any(char::is_whitespace) {
        return Err(SubmissionError::output(
            "sbatch returned malformed parsable output",
            stderr,
        ));
    }

    let mut fields = output.split(';');
    let job_id = fields.next().unwrap_or_default();
    let cluster = fields.next();

    if job_id.is_empty() || cluster == Some("") || fields.next().is_some() {
        return Err(SubmissionError::output(
            "sbatch returned malformed parsable output",
            stderr,
        ));
    }

    Ok(SubmissionResult {
        job_id: job_id.to_string(),
        cluster: cluster.map(str::to_string),
    })
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn effective_export(remainder: &[String], default: &str) -> String {
    let mut export = default.to_string();
    let mut index = 0;

    while index < remainder.len() {
        if let Some(value) = remainder[index].strip_prefix("--export=") {
            export = value.to_string();
        } else if remainder[index] == "--export" {
            if let Some(value) = remainder.get(index + 1) {
                export = value.to_string();
                index += 1;
            }
        }
        index += 1;
    }

    export
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
            return format!("0:{remainder}");
        }

        let secs = remainder % 60;
        remainder /= 60;

        if remainder < 60 {
            // less than an hour
            return format!("{remainder}:{secs}");
        }

        let mins = remainder % 60;
        remainder /= 60;

        format!("{remainder}:{mins}:{secs}")
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
    fn test_make_submission_script() {
        let shebang = "#/bin/bash";
        let set = "eux";
        let name = "job";
        let memory = "1M";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --mem={memory}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}
set -{set}

{cmd}
"#
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_make_submission_script_no_set() {
        let shebang = "#/bin/bash";
        let set = "";
        let name = "job";
        let memory = "1M";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --mem={memory}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}


{cmd}
"#
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_make_submission_script_mem_is_zero() {
        let shebang = "#/bin/bash";
        let set = "";
        let name = "job";
        let memory = "0";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}


{cmd}
"#
        );
        assert_eq!(actual, expected)
    }
}
