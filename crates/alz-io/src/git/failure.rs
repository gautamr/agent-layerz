//! Why a git call did not give its result.

use std::io;
use std::process::ExitStatus;
use std::time::Duration;

use alz_core::git::UnreadableOutput;
use thiserror::Error;

/// A git call that failed. Git's standard error is untrusted bytes: it is kept for the caller
/// to reduce with `FetchFailureLine` (spec §6 "A failed fetch") and is never printed from here.
#[derive(Debug, Error)]
pub enum GitFailure {
    /// The launcher process could not be started.
    #[error("cannot start git")]
    Spawn(#[source] io::Error),
    /// A pipe to or from git failed, or a thread for it could not be started.
    #[error("cannot talk to git")]
    Io(#[source] io::Error),
    /// The call outran its deadline; git's process group was ended.
    #[error("git did not finish within {} seconds", .after.as_secs())]
    TimedOut {
        /// The deadline that passed.
        after: Duration,
    },
    /// Git exited with a status other than success.
    #[error("git failed: {status}")]
    Exited {
        /// How git ended.
        status: ExitStatus,
        /// The first 64 KiB of git's standard error.
        stderr: Vec<u8>,
    },
    /// Git wrote more to standard output than the call allows.
    #[error("git wrote more output than alz accepts")]
    OutputTooLarge,
    /// Git succeeded but its output does not have the shape the call asks for.
    #[error("git succeeded, but {0}")]
    Unreadable(#[from] UnreadableOutput),
}
