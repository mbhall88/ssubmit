use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::io::Write;
use std::process::{Command, Stdio};

use ssubmit::make_submission_script;

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

    // Validate and get the command to execute
    let command = args.validate_and_get_command().map_err(|e| anyhow!(e))?;

    if args.interactive {
        handle_interactive_job(&args, &command)
    } else {
        handle_batch_job(&args, &command)
    }
}

fn handle_batch_job(args: &Cli, command: &str) -> Result<()> {
    let script = make_submission_script(
        &args.shebang,
        &args.set,
        &args.name,
        &args.memory,
        &args.time,
        &args.error,
        &args.output,
        command,
    );

    let mut sbatch_opts = args.remainder.clone();

    let test_only = if args.test_only {
        sbatch_opts.push("--test-only".to_string());
        true
    } else {
        let mut test_only = false;
        for arg in &args.remainder {
            if arg == "--test-only" {
                test_only = true;
                break;
            }
        }
        test_only
    };

    if args.dry_run {
        info!("Dry run requested. Nothing submitted");
        let sbatch_opts: String = sbatch_opts.join(" ");
        if sbatch_opts.is_empty() {
            println!("sbatch <script>")
        } else {
            println!("sbatch {sbatch_opts} <script>")
        }
        println!("=====<script>=====\n{script}=====<script>=====");
    } else {
        let mut sbatch_child = Command::new("sbatch")
            .args(&sbatch_opts)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn sbatch process")?;

        {
            let stdin = sbatch_child
                .stdin
                .as_mut()
                .context("Failed to connect to stdio of sbatch process")?;
            stdin
                .write_all(script.as_bytes())
                .context("Failed to write to sbatch process' stdin")?;
        }
        let sbatch_output = sbatch_child
            .wait_with_output()
            .context("Failed to execute sbatch")?;

        match sbatch_output.status.code() {
            Some(0) => {
                if test_only {
                    for line in String::from_utf8_lossy(&sbatch_output.stderr).lines() {
                        // the relevant line will be something like sbatch: Job 123456 to start at ...
                        if line.starts_with("sbatch: Job") {
                            info!("{}", line);
                            break;
                        }
                    }
                } else {
                    info!(
                        "{}",
                        String::from_utf8_lossy(&sbatch_output.stdout).trim_end()
                    )
                };
            }
            Some(c) => {
                error!(
                    "Failed to submit job with exit code {c} and stderr {}",
                    String::from_utf8_lossy(&sbatch_output.stderr)
                );
            }
            None => return Err(anyhow!("Process terminated by signal")),
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

    // Add the srun command
    salloc_args.push(command.to_string());

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
