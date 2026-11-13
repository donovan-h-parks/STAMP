//! Command-line interface to STAMP.

mod args;
mod commands;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
