use anyhow::Result;
use clap::Parser;
use env_logger::Builder;
use log::LevelFilter;

use ssubmit::format_number;

use crate::cli::Cli;

mod cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    // setup logger
    let log_lvl = if args.quiet {
        LevelFilter::Error
    } else {
        LevelFilter::Info
    };
    let mut log_builder = Builder::new();
    log_builder
        .filter(None, log_lvl)
        .format_module_path(false)
        .init();

    let memory = format_number(args.memory.0);

    println!("Job name is {}", args.name);
    println!("Requesting {} memory", memory);
    println!("Running command: {:?}", args.command);

    Ok(())
}
