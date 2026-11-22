//! The `alz` binary: parses the command line, runs the command and maps the outcome to an exit
//! code. Only this crate decides the exit code (decision-26).

mod cli;
mod report;
mod terminal_text;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(Cli {}) => ExitCode::SUCCESS,
        Err(error) => report::clap_outcome(&error),
    }
}
