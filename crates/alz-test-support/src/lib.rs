//! Test helpers for agent-layerz: scratch directories and the file kinds a test needs to plant.
//!
//! The helpers use only the standard library and the `mkfifo` tool, so no test depends on a
//! third-party temporary-directory crate. They never read the real home directory.

mod scratch_dir;

use std::io;
use std::path::PathBuf;

pub use scratch_dir::ScratchDir;

/// The path of the running test executable, so that a test can start itself again, for instance
/// under a pseudo-terminal.
///
/// # Errors
///
/// The error of finding the executable.
pub fn test_executable() -> io::Result<PathBuf> {
    std::env::current_exe()
}
