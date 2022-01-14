use anyhow::Result;
use clap::Parser;
use ssubmit::format_number;

use crate::cli::Cli;

mod cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    let memory = format_number(args.memory.0);

    println!("Job name is {}", args.name);
    println!("Requesting {} memory", memory);
    println!("Running command: {:?}", args.command);

    Ok(())
}
