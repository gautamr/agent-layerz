//! The `alz` binary: parses the command line, runs the command and maps the outcome to an exit
//! code. Only this crate decides the exit code (decision-26).

mod cli;
mod internal_failure;
mod report;
mod terminal_text;

use std::io;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::report::Rendered;

fn main() -> ExitCode {
    internal_failure::install_hook();
    internal_failure::exit_code_of(run)
}

fn run() -> ExitCode {
    match Cli::try_parse() {
        Ok(Cli {}) => ExitCode::SUCCESS,
        Err(error) => {
            ExitCode::from(Rendered::of(error).emit(&mut io::stdout(), &mut io::stderr()))
        }
    }
}
