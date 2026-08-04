use std::cmp::max;
use std::fmt::Write as _;
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
pub struct JsonError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonResponse {
    pub schema_version: u8,
    pub operation: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<SubmissionPlan>,
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
            error: None,
        }
    }

    pub fn error(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            operation: "plan".to_string(),
            ok: false,
            plan: None,
            error: Some(JsonError {
                kind: kind.into(),
                message: message.into(),
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

    let mut arguments = remainder.to_vec();
    if !arguments.iter().any(|arg| arg.starts_with("--export")) {
        arguments.push(format!("--export={export}"));
    }

    if test_only {
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
            export: export.to_string(),
        },
        slurm: SlurmPlan {
            executable: "sbatch".to_string(),
            arguments,
            script,
        },
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
