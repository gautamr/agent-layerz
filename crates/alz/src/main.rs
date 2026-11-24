//! The `alz` binary: parses the command line, runs the command and maps the outcome to an exit
//! code. Only this crate decides the exit code (decision-26).

mod cli;
mod command;
mod internal_failure;
mod report;
mod terminal_text;

use std::io;
use std::process::ExitCode;

use alz_io::{EnvSnapshot, WorkingDirectory};
use clap::Parser;

use crate::cli::{AccountArgs, AccountCommand, Cli, Command};
use crate::command::Surroundings;
use crate::report::Rendered;

fn main() -> ExitCode {
    // First, before the command line is read: a copy of alz that the git runner started runs git
    // in a new session and does nothing else (TASK-70).
    if let Some(exit_code) = alz_io::run_if_requested() {
        return exit_code;
    }
    internal_failure::install_hook();
    // The environment and the working directory are read once, here, and passed down.
    let env = EnvSnapshot::capture();
    let directory = WorkingDirectory::capture();
    internal_failure::exit_code_of(|| run(&env, &directory))
}

fn run(env: &EnvSnapshot, directory: &WorkingDirectory) -> ExitCode {
    match Cli::try_parse() {
        Ok(Cli {
            command: Command::Build(args),
        }) => ExitCode::from(command::build::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Check(args),
        }) => ExitCode::from(command::check::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Diff,
        }) => ExitCode::from(command::diff::run(
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Init(args),
        }) => ExitCode::from(command::init::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Lint(args),
        }) => ExitCode::from(command::lint::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Ls(args),
        }) => ExitCode::from(command::ls::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Doctor,
        }) => ExitCode::from(command::doctor::run(
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command: Command::Explain(args),
        }) => ExitCode::from(command::explain::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command:
                Command::Account(AccountArgs {
                    command: AccountCommand::Which(args),
                }),
        }) => ExitCode::from(command::account::which::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Ok(Cli {
            command:
                Command::Account(AccountArgs {
                    command: AccountCommand::Allow(args),
                }),
        }) => ExitCode::from(command::account::allow::run(
            &args,
            Surroundings { env, directory },
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Err(error) => {
            ExitCode::from(Rendered::of(error).emit(&mut io::stdout(), &mut io::stderr()))
        }
    }
}
