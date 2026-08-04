#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{json, Value};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct FakeSbatch {
    directory: PathBuf,
    args_path: PathBuf,
    script_path: PathBuf,
    invoked_path: PathBuf,
    stdout: String,
    stderr: String,
    exit_code: i32,
    terminate_by_signal: bool,
}

impl FakeSbatch {
    fn new(stdout: &str, stderr: &str, exit_code: i32) -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let directory = env::temp_dir().join(format!(
            "ssubmit-batch-cli-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("create fake sbatch directory");

        let args_path = directory.join("args");
        let script_path = directory.join("script");
        let invoked_path = directory.join("invoked");
        let sbatch_path = directory.join("sbatch");
        let script = r#"#!/bin/sh
set -eu
: > "$SSUBMIT_FAKE_INVOKED"
printf '%s\n' "$@" > "$SSUBMIT_FAKE_ARGS"
cat > "$SSUBMIT_FAKE_SCRIPT"
printf '%s' "$SSUBMIT_FAKE_STDOUT"
printf '%s' "$SSUBMIT_FAKE_STDERR" >&2
if [ "${SSUBMIT_FAKE_SIGNAL:-0}" = 1 ]; then
    kill -TERM $$
fi
exit "$SSUBMIT_FAKE_EXIT"
"#;
        fs::write(&sbatch_path, script).expect("write fake sbatch");
        fs::set_permissions(&sbatch_path, fs::Permissions::from_mode(0o755))
            .expect("make fake sbatch executable");

        Self {
            directory,
            args_path,
            script_path,
            invoked_path,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            exit_code,
            terminate_by_signal: false,
        }
    }

    fn terminating_by_signal(stdout: &str, stderr: &str) -> Self {
        let mut fake = Self::new(stdout, stderr, 0);
        fake.terminate_by_signal = true;
        fake
    }

    fn command(&self) -> Command {
        let path = format!("{}:/usr/bin:/bin", self.directory.display());
        let mut command = Command::new(env!("CARGO_BIN_EXE_ssubmit"));
        command
            .env_clear()
            .env("PATH", path)
            .env("SSUBMIT_FAKE_ARGS", &self.args_path)
            .env("SSUBMIT_FAKE_SCRIPT", &self.script_path)
            .env("SSUBMIT_FAKE_INVOKED", &self.invoked_path)
            .env("SSUBMIT_FAKE_STDOUT", &self.stdout)
            .env("SSUBMIT_FAKE_STDERR", &self.stderr)
            .env("SSUBMIT_FAKE_EXIT", self.exit_code.to_string());
        if self.terminate_by_signal {
            command.env("SSUBMIT_FAKE_SIGNAL", "1");
        }
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("run ssubmit")
    }

    fn recorded_args(&self) -> String {
        fs::read_to_string(&self.args_path).expect("read fake sbatch arguments")
    }

    fn recorded_script(&self) -> String {
        fs::read_to_string(&self.script_path).expect("read fake sbatch script")
    }
}

fn parse_json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).expect("parse one JSON response")
}

fn assert_required_fields(value: &Value, required: &Value, label: &str) {
    let object = value.as_object().expect("schema value must be an object");
    for property in required.as_array().expect("schema required properties") {
        let property = property.as_str().expect("schema property name");
        assert!(
            object.contains_key(property),
            "{label} is missing required property {property}"
        );
    }
}

fn assert_required_string_fields(value: &Value, schema: &Value, label: &str) {
    let object = value.as_object().expect("schema value must be an object");
    let required = schema["required"]
        .as_array()
        .expect("schema required properties");
    for property in required {
        let property = property.as_str().expect("schema property name");
        assert!(
            object.get(property).and_then(Value::as_str).is_some(),
            "{label}.{property} must be a string"
        );
    }
}

fn assert_matches_schema(response: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/ssubmit-output-v1.schema.json"))
            .expect("parse committed JSON schema");

    // Keep this dependency-free for the project's Rust 1.58 MSRV. These
    // assertions mirror every draft-07 constraint used by schema v1.
    assert_required_fields(response, &schema["required"], "response");
    assert_eq!(
        response["schema_version"],
        schema["properties"]["schema_version"]["const"]
    );

    let operation = response["operation"]
        .as_str()
        .expect("response.operation must be a string");
    let operations = schema["properties"]["operation"]["enum"]
        .as_array()
        .expect("schema operation enum");
    assert!(
        operations
            .iter()
            .any(|value| value.as_str() == Some(operation)),
        "unsupported response operation {operation}"
    );
    let ok = response["ok"]
        .as_bool()
        .expect("response.ok must be a boolean");

    if ok {
        match operation {
            "plan" | "test" => assert!(response["plan"].is_object()),
            "submit" => assert!(response["submission"].is_object()),
            _ => unreachable!("operation enum was checked above"),
        }
        if response["plan"].is_object() {
            assert_required_fields(
                &response["plan"],
                &schema["definitions"]["plan"]["required"],
                "plan",
            );
            assert_required_string_fields(
                &response["plan"]["job"],
                &schema["definitions"]["job"],
                "plan.job",
            );
            assert_required_fields(
                &response["plan"]["slurm"],
                &schema["definitions"]["slurm"]["required"],
                "plan.slurm",
            );
            assert!(
                response["plan"]["slurm"]["executable"].is_string(),
                "plan.slurm.executable must be a string"
            );
            assert!(
                response["plan"]["slurm"]["script"].is_string(),
                "plan.slurm.script must be a string"
            );
            assert!(
                response["plan"]["slurm"]["arguments"]
                    .as_array()
                    .expect("plan.slurm.arguments must be an array")
                    .iter()
                    .all(Value::is_string),
                "plan.slurm.arguments must contain only strings"
            );
        }
        if response["submission"].is_object() {
            assert_required_fields(
                &response["submission"],
                &schema["definitions"]["submission"]["required"],
                "submission",
            );
            assert!(
                response["submission"]["job_id"].is_string(),
                "submission.job_id must be a string"
            );
            assert!(
                response["submission"]["cluster"].is_null()
                    || response["submission"]["cluster"].is_string(),
                "submission.cluster must be null or a string"
            );
        }
    } else {
        assert_required_string_fields(&response["error"], &schema["definitions"]["error"], "error");
        if response["error"].get("exit_code").is_some() {
            assert!(response["error"]["exit_code"].is_i64());
        }
        if response["error"].get("stderr").is_some() {
            assert!(response["error"]["stderr"].is_string());
        }
    }
}

impl Drop for FakeSbatch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn successful_batch_submission_preserves_text_output_and_submits_script() {
    let fake = FakeSbatch::new("Submitted batch job 1234\n", "", 0);

    let output = fake.run(&[
        "--mem",
        "2G",
        "--time",
        "2h",
        "example",
        "echo hello",
        "--",
        "--partition=short",
    ]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Submitted batch job 1234"));
    assert_eq!(fake.recorded_args(), "--partition=short\n--export=ALL\n");

    let script = fake.recorded_script();
    assert!(script.contains("#SBATCH --job-name=example\n"));
    assert!(script.contains("#SBATCH --mem=2000M\n"));
    assert!(script.contains("#SBATCH --time=2:0:0\n"));
    assert!(script.ends_with("echo hello\n"));
}

#[test]
fn rejected_batch_submission_returns_non_zero_and_preserves_slurm_diagnostics() {
    let fake = FakeSbatch::new("", "sbatch: error: Invalid partition name\n", 1);

    let output = fake.run(&["example", "echo hello"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid partition name"));
    assert!(stderr.contains("exit code 1"));
}

#[test]
fn dry_run_prints_plan_without_invoking_sbatch() {
    let fake = FakeSbatch::new("unexpected output", "unexpected error", 99);

    let output = fake.run(&["--dry-run", "example", "echo hello"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sbatch --export=ALL <script>"));
    assert!(stdout.contains("#SBATCH --job-name=example"));
    assert!(!Path::new(&fake.invoked_path).exists());
    assert!(!Path::new(&fake.args_path).exists());
    assert!(!Path::new(&fake.script_path).exists());
}

#[test]
fn documented_time_environment_variable_controls_the_submission_script() {
    let fake = FakeSbatch::new("unused", "unused", 0);

    let output = fake
        .command()
        .env("SSUBMIT_TIME", "3h")
        .args(["--dry-run", "example", "echo hello"])
        .output()
        .expect("run ssubmit");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("#SBATCH --time=3:0:0"));
}

#[test]
fn json_dry_run_returns_a_versioned_plan_without_invoking_sbatch() {
    let fake = FakeSbatch::new("unexpected output", "unexpected error", 99);

    let output = fake.run(&[
        "--dry-run",
        "--json",
        "--mem",
        "2G",
        "--time",
        "2h",
        "example",
        "echo hello",
        "--",
        "--cpus-per-task=8",
        "--partition=short",
        "--export=NONE",
    ]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["schema_version"], json!(1));
    assert_eq!(response["operation"], json!("plan"));
    assert_eq!(response["ok"], json!(true));
    assert_eq!(response["plan"]["job"]["name"], json!("example"));
    assert_eq!(response["plan"]["job"]["command"], json!("echo hello"));
    assert_eq!(response["plan"]["job"]["memory"], json!("2000M"));
    assert_eq!(response["plan"]["job"]["time"], json!("2:0:0"));
    assert_eq!(response["plan"]["job"]["output"], json!("%x.out"));
    assert_eq!(response["plan"]["job"]["error"], json!("%x.err"));
    assert_eq!(response["plan"]["job"]["export"], json!("NONE"));
    assert_eq!(
        response["plan"]["slurm"]["arguments"],
        json!(["--cpus-per-task=8", "--partition=short", "--export=NONE"])
    );
    assert_eq!(response["plan"]["slurm"]["executable"], json!("sbatch"));
    assert!(response["plan"]["slurm"]["script"]
        .as_str()
        .expect("plan script")
        .contains("#SBATCH --job-name=example"));
    assert!(!Path::new(&fake.invoked_path).exists());
}

#[test]
fn json_interactive_request_returns_a_structured_validation_error() {
    let fake = FakeSbatch::new("unexpected output", "unexpected error", 99);

    let output = fake.run(&[
        "--dry-run",
        "--json",
        "--interactive",
        "interactive-example",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("JSON mode does not support interactive jobs"));
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["schema_version"], json!(1));
    assert_eq!(response["operation"], json!("plan"));
    assert_eq!(response["ok"], json!(false));
    assert_eq!(response["error"]["kind"], json!("validation"));
    assert!(response["error"]["message"]
        .as_str()
        .expect("validation error message")
        .contains("interactive"));
    assert!(!Path::new(&fake.invoked_path).exists());
}

#[test]
fn json_submission_returns_job_id_without_a_cluster() {
    let fake = FakeSbatch::new("987654\n", "", 0);

    let output = fake.run(&["--json", "example", "echo hello"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim_start().starts_with('{'));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["schema_version"], json!(1));
    assert_eq!(response["operation"], json!("submit"));
    assert_eq!(response["ok"], json!(true));
    assert_eq!(response["submission"]["job_id"], json!("987654"));
    assert!(response["submission"]["cluster"].is_null());
    assert_eq!(
        response["plan"]["slurm"]["arguments"],
        json!(["--export=ALL", "--parsable"])
    );
    assert_eq!(fake.recorded_args(), "--export=ALL\n--parsable\n");
    assert!(fake
        .recorded_script()
        .contains("#SBATCH --job-name=example"));
}

#[test]
fn json_submission_parses_cluster_and_deduplicates_parsable_options() {
    let fake = FakeSbatch::new("42;gpu-cluster\n", "", 0);

    let output = fake.run(&[
        "--json",
        "example",
        "echo hello",
        "--",
        "--partition=short",
        "--parsable",
        "--parsable",
    ]);

    assert!(output.status.success());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["submission"]["job_id"], json!("42"));
    assert_eq!(response["submission"]["cluster"], json!("gpu-cluster"));
    assert_eq!(
        response["plan"]["slurm"]["arguments"],
        json!(["--partition=short", "--parsable", "--export=ALL"])
    );
    assert_eq!(
        fake.recorded_args(),
        "--partition=short\n--parsable\n--export=ALL\n"
    );
}

#[test]
fn json_submission_rejected_by_slurm_returns_structured_error() {
    let fake = FakeSbatch::new("", "sbatch: error: Invalid partition name\n", 1);

    let output = fake.run(&["--json", "example", "echo hello"]);

    assert!(!output.status.success());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["operation"], json!("submit"));
    assert_eq!(response["ok"], json!(false));
    assert_eq!(response["error"]["kind"], json!("slurm"));
    assert_eq!(response["error"]["exit_code"], json!(1));
    assert_eq!(
        response["error"]["stderr"],
        json!("sbatch: error: Invalid partition name")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid partition name"));
    assert!(String::from_utf8_lossy(&output.stdout)
        .trim()
        .ends_with('}'));
}

#[test]
fn json_submission_rejects_empty_and_malformed_success_output() {
    for (stdout, expected_message) in [
        ("", "empty output"),
        ("123 bad\n", "malformed parsable output"),
        ("123;cluster;extra\n", "malformed parsable output"),
    ] {
        let fake = FakeSbatch::new(stdout, "", 0);
        let output = fake.run(&["--json", "example", "echo hello"]);

        assert!(
            !output.status.success(),
            "output {stdout:?} unexpectedly passed"
        );
        let response = parse_json(&output);
        assert_matches_schema(&response);
        assert_eq!(response["operation"], json!("submit"));
        assert_eq!(response["ok"], json!(false));
        assert_eq!(response["error"]["kind"], json!("output"));
        assert!(response["error"]["message"]
            .as_str()
            .expect("output parsing message")
            .contains(expected_message));
    }
}

#[test]
fn json_submission_reports_launch_failure_without_invoking_sbatch() {
    let fake = FakeSbatch::new("unused", "unused", 0);

    let output = fake
        .command()
        .env("PATH", "/usr/bin:/bin")
        .args(["--json", "example", "echo hello"])
        .output()
        .expect("run ssubmit without sbatch");

    assert!(!output.status.success());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["error"]["kind"], json!("process"));
    assert!(!Path::new(&fake.invoked_path).exists());
}

#[test]
fn json_submission_reports_signal_termination() {
    let fake = FakeSbatch::terminating_by_signal("", "sbatch: interrupted\n");

    let output = fake.run(&["--json", "example", "echo hello"]);

    assert!(!output.status.success());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["error"]["kind"], json!("process"));
    assert!(response["error"]["message"]
        .as_str()
        .expect("signal error message")
        .contains("signal"));
    assert_eq!(response["error"]["stderr"], json!("sbatch: interrupted"));
}

#[test]
fn json_submission_rejects_quiet_passthrough_option() {
    let fake = FakeSbatch::new("unexpected", "unexpected", 0);

    let output = fake.run(&["--json", "example", "echo hello", "--", "--quiet"]);

    assert!(!output.status.success());
    let response = parse_json(&output);
    assert_matches_schema(&response);
    assert_eq!(response["error"]["kind"], json!("validation"));
    assert!(response["error"]["message"]
        .as_str()
        .expect("quiet validation message")
        .contains("quiet"));
    assert!(!Path::new(&fake.invoked_path).exists());
}
