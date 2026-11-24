//! The commands of the specification. Each one reads what it needs through `alz-io`, decides with
//! `alz-core` and returns the exit code; `main` only dispatches.

pub mod init;
pub mod lint;

use alz_io::{EnvSnapshot, WorkingDirectory};

/// Where and with what environment a command runs, both taken once when the process started.
#[derive(Debug, Clone, Copy)]
pub struct Surroundings<'a> {
    pub env: &'a EnvSnapshot,
    pub directory: &'a WorkingDirectory,
}
