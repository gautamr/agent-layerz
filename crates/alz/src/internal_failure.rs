//! A bug in alz, a panic, ends the run with a fixed message on standard error and exit code 70
//! (decision-26). The message never carries the panic's payload or location: either may hold
//! untrusted text, and the run's own output stays the only text a person reads.

use std::io::{self, Write};
use std::panic::{self, UnwindSafe};
use std::process::ExitCode;

/// Exit code of an internal failure (decision-26).
const EXIT_CODE: u8 = 70;

/// The one line printed for an internal failure.
const MESSAGE: &str = "alz: internal error: this is a bug in alz, please report it\n";

/// Replaces the default panic message, which prints the payload and location, with [`MESSAGE`].
pub fn install_hook() {
    panic::set_hook(Box::new(|_| {
        // A panic hook cannot report that it failed to print; the exit code of 70 still tells.
        write_message(&mut io::stderr()).unwrap_or_default();
    }));
}

/// Runs the command and returns its exit code, or 70 if it panicked. This is the process
/// boundary: `main` is the one place that decides the exit code, so it is the one place a panic is
/// caught, never as control flow inside the program.
pub fn exit_code_of(run: impl FnOnce() -> ExitCode + UnwindSafe) -> ExitCode {
    panic::catch_unwind(run).unwrap_or(ExitCode::from(EXIT_CODE))
}

fn write_message(stream: &mut impl Write) -> io::Result<()> {
    stream.write_all(MESSAGE.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::{MESSAGE, exit_code_of, write_message};

    #[test]
    fn a_panic_ends_the_run_with_exit_code_seventy() {
        let code = exit_code_of(|| panic!("a bug"));
        assert_eq!(code, ExitCode::from(70));
    }

    #[test]
    fn a_run_that_returns_keeps_its_exit_code() {
        assert_eq!(exit_code_of(|| ExitCode::from(2)), ExitCode::from(2));
    }

    #[test]
    fn the_message_is_one_fixed_line_without_the_panic_payload() {
        let mut stderr = Vec::new();
        write_message(&mut stderr).expect("a vector accepts writes");
        assert_eq!(String::from_utf8_lossy(&stderr), MESSAGE);
        assert_eq!(MESSAGE.lines().count(), 1);
    }
}
