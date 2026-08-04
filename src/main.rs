use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::process::Command;

use ssubmit::{
    classify_sbatch_failure, make_submission_plan, prepare_machine_submission,
    prepare_machine_test, run_sbatch, submit_sbatch, test_sbatch, JsonResponse, SubmissionError,
};

use crate::cli::Cli;

mod cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    // setup logger
    let mut log_builder = Builder::new();
    log_builder
        .filter(None, LevelFilter::Info)
        .format_module_path(false)
        .init();

    if args.json && args.interactive {
        return emit_json_error("JSON mode does not support interactive jobs");
    }

    // Validate and get the command to execute
    let command = match args.validate_and_get_command() {
        Ok(command) => command,
        Err(error) if args.json => return emit_json_error(error),
        Err(error) => return Err(anyhow!(error)),
    };

    if args.interactive {
        handle_interactive_job(&args, &command)
    } else {
        handle_batch_job(&args, &command)
    }
}

fn emit_json_response(response: JsonResponse) -> Result<()> {
    let output = serde_json::to_string(&response).context("Failed to render JSON response")?;
    println!("{output}");
    Ok(())
}

fn emit_json_error(message: impl Into<String>) -> Result<()> {
    let message = message.into();
    let response = JsonResponse::error("validation", message.clone());
    emit_json_response(response)?;
    Err(anyhow!("{}", message))
}

fn emit_json_submission_error(plan: ssubmit::SubmissionPlan, error: SubmissionError) -> Result<()> {
    let message = error.message.clone();
    if let Some(stderr) = error.stderr.as_deref() {
        eprintln!("{stderr}");
    }
    emit_json_response(JsonResponse::submission_error(plan, error))?;
    Err(anyhow!("{}", message))
}

fn emit_json_scheduler_test_error(
    plan: ssubmit::SubmissionPlan,
    error: SubmissionError,
) -> Result<()> {
    let message = error.message.clone();
    if let Some(stderr) = error.stderr.as_deref() {
        eprintln!("{stderr}");
    }
    let response = JsonResponse::scheduler_test_error(plan, error);
    emit_json_response(response)?;
    Err(anyhow!("{}", message))
}

fn human_submission_error(error: &SubmissionError) -> String {
    match &error.stderr {
        Some(stderr) => format!("{}: {stderr}", error.message),
        None => error.message.clone(),
    }
}

fn handle_batch_job(args: &Cli, command: &str) -> Result<()> {
    let plan = make_submission_plan(
        &args.shebang,
        &args.set,
        &args.name,
        &args.memory,
        &args.time,
        &args.error,
        &args.output,
        command,
        &args.remainder,
        &args.export,
        args.test_only,
    );

    if args.json {
        let test_only = plan.slurm.arguments.iter().any(|arg| arg == "--test-only");
        if args.dry_run {
            return emit_json_response(JsonResponse::plan(plan));
        }

        if test_only {
            let test_plan = match prepare_machine_test(&plan) {
                Ok(plan) => plan,
                Err(error) => return emit_json_scheduler_test_error(plan, error),
            };
            return match test_sbatch(&test_plan) {
                Ok(result) => emit_json_response(JsonResponse::scheduler_test(test_plan, result)),
                Err(error) => emit_json_scheduler_test_error(test_plan, error),
            };
        }

        let machine_plan = match prepare_machine_submission(&plan) {
            Ok(plan) => plan,
            Err(error) => return emit_json_submission_error(plan, error),
        };
        return match submit_sbatch(&machine_plan) {
            Ok(result) => emit_json_response(JsonResponse::submission(machine_plan, result)),
            Err(error) => emit_json_submission_error(machine_plan, error),
        };
    }

    if args.dry_run {
        info!("Dry run requested. Nothing submitted");
        let sbatch_opts = plan.slurm.arguments.join(" ");
        if sbatch_opts.is_empty() {
            println!("sbatch <script>")
        } else {
            println!("sbatch {sbatch_opts} <script>")
        }
        println!(
            "=====<script>=====\n{}=====<script>=====",
            plan.slurm.script
        );
    } else {
        let test_only = plan.slurm.arguments.iter().any(|arg| arg == "--test-only");
        let sbatch_output =
            run_sbatch(&plan).map_err(|error| anyhow!(human_submission_error(&error)))?;

        if let Some(failure) = classify_sbatch_failure(&sbatch_output) {
            let message = human_submission_error(&failure);
            error!("{message}");
            return Err(anyhow!("{}", message));
        }

        if test_only {
            for line in sbatch_output.stderr.lines() {
                // the relevant line will be something like sbatch: Job 123456 to start at ...
                if line.starts_with("sbatch: Job") {
                    info!("{}", line);
                    break;
                }
            }
        } else {
            info!("{}", sbatch_output.stdout.trim_end())
        }
    }

    Ok(())
}

fn handle_interactive_job(args: &Cli, command: &str) -> Result<()> {
    let mut salloc_args = vec![
        "--job-name".to_string(),
        args.name.clone(),
        "--mem".to_string(),
        args.memory.clone(),
        "--time".to_string(),
        args.time.clone(),
    ];

    // Add any additional options from remainder
    salloc_args.extend(args.remainder.clone());

    // Parse the command into separate arguments for salloc
    // Split on whitespace but preserve quoted strings
    let command_parts: Vec<&str> = command.split_whitespace().collect();
    salloc_args.extend(command_parts.iter().map(|s| s.to_string()));

    if args.dry_run {
        info!("Dry run requested. Nothing submitted");
        let salloc_cmd = format!("salloc {}", salloc_args.join(" "));
        println!("{salloc_cmd}");
    } else if args.test_only {
        // For test-only, we can use salloc --test-only but it won't show as much info
        let mut test_args = salloc_args.clone();
        test_args.insert(0, "--test-only".to_string());

        let salloc_output = Command::new("salloc")
            .args(&test_args)
            .output()
            .context("Failed to execute salloc --test-only")?;

        match salloc_output.status.code() {
            Some(0) => {
                info!("Interactive job would be scheduled");
                if !salloc_output.stdout.is_empty() {
                    info!("{}", String::from_utf8_lossy(&salloc_output.stdout));
                }
                if !salloc_output.stderr.is_empty() {
                    info!("{}", String::from_utf8_lossy(&salloc_output.stderr));
                }
            }
            Some(c) => {
                error!(
                    "Failed to test interactive job with exit code {c} and stderr {}",
                    String::from_utf8_lossy(&salloc_output.stderr)
                );
            }
            None => return Err(anyhow!("Process terminated by signal")),
        }
    } else {
        info!("Starting interactive job: {}", args.name);
        let exit_status = Command::new("salloc")
            .args(&salloc_args)
            .status()
            .context("Failed to execute salloc")?;

        if !exit_status.success() {
            return Err(anyhow!("Interactive job failed"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test CLI struct
    fn create_test_cli(export: &str, remainder: Vec<String>) -> Cli {
        Cli {
            name: "test_job".to_string(),
            command: Some("echo hello".to_string()),
            remainder,
            output: "%x.out".to_string(),
            error: "%x.err".to_string(),
            memory: "1G".to_string(),
            time: "1d".to_string(),
            shebang: "#!/usr/bin/env bash".to_string(),
            set: "euxo pipefail".to_string(),
            dry_run: true, // Use dry_run to avoid actually running sbatch
            json: false,
            test_only: false,
            interactive: false,
            shell: "bash".to_string(),
            export: export.to_string(),
        }
    }

    #[test]
    fn test_export_default_all() {
        let cli = create_test_cli("ALL", vec![]);
        let _command = cli.validate_and_get_command().unwrap();

        // Test that handle_batch_job would add --export=ALL
        let _script = ssubmit::make_submission_script(
            &cli.shebang,
            &cli.set,
            &cli.name,
            &cli.memory,
            &cli.time,
            &cli.error,
            &cli.output,
            &_command,
        );

        let mut sbatch_opts = cli.remainder.clone();
        let has_export = sbatch_opts.iter().any(|arg| arg.starts_with("--export"));
        if !has_export {
            sbatch_opts.push(format!("--export={}", cli.export));
        }

        assert!(sbatch_opts.contains(&"--export=ALL".to_string()));
    }

    #[test]
    fn test_export_none() {
        let cli = create_test_cli("NONE", vec![]);
        let _command = cli.validate_and_get_command().unwrap();

        let mut sbatch_opts = cli.remainder.clone();
        let has_export = sbatch_opts.iter().any(|arg| arg.starts_with("--export"));
        if !has_export {
            sbatch_opts.push(format!("--export={}", cli.export));
        }

        assert!(sbatch_opts.contains(&"--export=NONE".to_string()));
    }

    #[test]
    fn test_export_specific_variables() {
        let cli = create_test_cli("PATH,HOME,USER", vec![]);
        let _command = cli.validate_and_get_command().unwrap();

        let mut sbatch_opts = cli.remainder.clone();
        let has_export = sbatch_opts.iter().any(|arg| arg.starts_with("--export"));
        if !has_export {
            sbatch_opts.push(format!("--export={}", cli.export));
        }

        assert!(sbatch_opts.contains(&"--export=PATH,HOME,USER".to_string()));
    }

    #[test]
    fn test_export_user_override_via_remainder() {
        let cli = create_test_cli("ALL", vec!["--export=NONE".to_string()]);

        let mut sbatch_opts = cli.remainder.clone();
        let has_export = sbatch_opts.iter().any(|arg| arg.starts_with("--export"));
        if !has_export {
            sbatch_opts.push(format!("--export={}", cli.export));
        }

        // Should not add the default --export=ALL since user specified --export=NONE
        assert!(sbatch_opts.contains(&"--export=NONE".to_string()));
        assert!(!sbatch_opts.contains(&"--export=ALL".to_string()));
        assert_eq!(sbatch_opts.len(), 1); // Only the user-specified export
    }
}
