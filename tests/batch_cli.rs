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
        }
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
    ]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let response = parse_json(&output);
    assert_eq!(response["schema_version"], json!(1));
    assert_eq!(response["operation"], json!("plan"));
    assert_eq!(response["ok"], json!(true));
    assert_eq!(response["plan"]["job"]["name"], json!("example"));
    assert_eq!(response["plan"]["job"]["command"], json!("echo hello"));
    assert_eq!(response["plan"]["job"]["memory"], json!("2000M"));
    assert_eq!(response["plan"]["job"]["time"], json!("2:0:0"));
    assert_eq!(response["plan"]["job"]["output"], json!("%x.out"));
    assert_eq!(response["plan"]["job"]["error"], json!("%x.err"));
    assert_eq!(response["plan"]["job"]["export"], json!("ALL"));
    assert_eq!(
        response["plan"]["slurm"]["arguments"],
        json!(["--cpus-per-task=8", "--partition=short", "--export=ALL"])
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
