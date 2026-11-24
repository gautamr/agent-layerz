//! How a program run on a terminal ended, and the exit code `pty-run` reports for it.

use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

/// The exit code of `pty-run` when it killed the program at its deadline (the code of `timeout`).
pub const DEADLINE_CODE: u8 = 124;

/// How the program ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It exited with this code.
    Exited(u8),
    /// A signal ended it.
    Signalled(u8),
    /// It was still running at the deadline and the driver killed its process group.
    KilledAtDeadline,
}

impl Outcome {
    /// What the standard library knows about a finished program.
    #[must_use]
    pub fn of(status: ExitStatus) -> Self {
        let low_byte = |number: i32| u8::try_from(number & 0xff).unwrap_or(u8::MAX);
        match (status.code(), status.signal()) {
            (Some(code), _) => Self::Exited(low_byte(code)),
            (None, Some(signal)) => Self::Signalled(low_byte(signal)),
            (None, None) => Self::Exited(u8::MAX),
        }
    }

    /// The exit code `pty-run` ends with: the program's own, `128 + signal` for a signal (as a
    /// shell reports it), and 124 for the deadline.
    #[must_use]
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Exited(code) => code,
            Self::Signalled(signal) => 128_u8.saturating_add(signal),
            Self::KilledAtDeadline => DEADLINE_CODE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEADLINE_CODE, Outcome};
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    #[test]
    fn reports_an_exit_code_a_signal_and_the_deadline_apart() {
        assert_eq!(
            Outcome::of(ExitStatus::from_raw(3 << 8)),
            Outcome::Exited(3)
        );
        assert_eq!(Outcome::of(ExitStatus::from_raw(2)), Outcome::Signalled(2));
        assert_eq!(Outcome::Exited(3).exit_code(), 3);
        assert_eq!(Outcome::Signalled(2).exit_code(), 130);
        assert_eq!(Outcome::KilledAtDeadline.exit_code(), DEADLINE_CODE);
    }
}
