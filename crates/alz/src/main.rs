//! The `alz` binary: parses the command line, runs the command and maps the outcome to an exit
//! code. Only this crate decides the exit code (decision-26).

mod cli;
mod command;
mod internal_failure;
mod report;
mod terminal_text;

use std::io;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::report::Rendered;

fn main() -> ExitCode {
    // First, before the command line is read: a copy of alz that the git runner started runs git
    // in a new session and does nothing else (TASK-70).
    if let Some(exit_code) = alz_io::run_if_requested() {
        return exit_code;
    }
    internal_failure::install_hook();
    internal_failure::exit_code_of(run)
}

fn run() -> ExitCode {
    match Cli::try_parse() {
        Ok(Cli {
            command: Command::Lint(args),
        }) => ExitCode::from(command::lint::run(
            &args,
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Err(error) => {
            ExitCode::from(Rendered::of(error).emit(&mut io::stdout(), &mut io::stderr()))
        }
    }
}
