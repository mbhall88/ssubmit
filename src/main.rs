use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::io::Write;
use std::process::{Command, Stdio};

use ssubmit::{format_number, make_submission_script};

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

    let memory = format_number(args.memory.0);

    let script = make_submission_script(
        &args.shebang,
        &args.set,
        &args.name,
        &memory,
        &args.time,
        &args.error,
        &args.output,
        &args.command,
    );

    if args.dry_run {
        info!("Dry run requested. Nothing submitted");
        let sbatch_opts: String = args.remainder.join(" ");
        if sbatch_opts.is_empty() {
            println!("sbatch <script>")
        } else {
            println!("sbatch {} <script>", sbatch_opts)
        }
        println!("=====<script>=====\n{script}=====<script>=====");
    } else {
        let mut sbatch_child = Command::new("sbatch")
            .args(&args.remainder)
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
            Some(0) => info!(
                "{}",
                String::from_utf8_lossy(&sbatch_output.stdout).trim_end()
            ),
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
