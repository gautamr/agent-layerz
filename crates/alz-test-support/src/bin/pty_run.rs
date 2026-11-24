//! `pty-run [--deadline-ms N] [--] program [argument...]`: runs the program with a
//! pseudo-terminal as its standard input and this process's own standard output and standard
//! error. What arrives on this process's standard input is typed into the terminal, and its end is
//! the terminal's end of file. The exit code is the program's, `128 + signal` for a signal, 124
//! when the program was killed at the deadline and 125 for a usage or set-up error (a program that
//! cannot be started may end the driver by the terminal's hangup signal instead, after the error
//! is written).

use std::io::Write;
use std::process::ExitCode;

use alz_test_support::pty::{Invocation, run};

/// The exit code for an error of `pty-run` itself.
const DRIVER_ERROR: u8 = 125;

/// Writes one line to standard error; nowhere is left to report a failure to write it.
fn report(message: &dyn std::fmt::Display) {
    writeln!(std::io::stderr(), "pty-run: {message}").unwrap_or_default();
}

fn main() -> ExitCode {
    match Invocation::parse(std::env::args_os().skip(1)) {
        Err(error) => {
            report(&error);
            ExitCode::from(DRIVER_ERROR)
        }
        Ok(invocation) => match run(&invocation, std::io::stdin()) {
            Ok(outcome) => ExitCode::from(outcome.exit_code()),
            // The failure holds the terminal open while it is reported, and closes it at the end.
            Err(failure) => {
                report(&failure);
                ExitCode::from(DRIVER_ERROR)
            }
        },
    }
}
