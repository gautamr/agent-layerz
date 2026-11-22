//! Renders what clap produced: help and version text on standard output, usage errors on
//! standard error, each sanitised (decision-26), and the exit code that goes with them.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Error;

use crate::terminal_text::printable;

/// The exit code for a usage error (decision-26).
const USAGE_ERROR: u8 = 2;

/// Prints clap's output to the stream clap chose, and returns the exit code: success for help
/// and version text, 2 for a usage error, 1 if the text could not be written.
pub fn clap_outcome(error: &Error) -> ExitCode {
    let text = printable(&error.render().to_string());
    let (written, code) = if error.use_stderr() {
        (
            io::stderr().write_all(text.as_bytes()),
            ExitCode::from(USAGE_ERROR),
        )
    } else {
        (io::stdout().write_all(text.as_bytes()), ExitCode::SUCCESS)
    };
    match written {
        Ok(()) => code,
        Err(_) => ExitCode::FAILURE,
    }
}
