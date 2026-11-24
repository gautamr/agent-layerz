//! Test helpers for agent-layerz: scratch directories and the file kinds a test needs to plant.
//!
//! The helpers use only the standard library and the `mkfifo` tool, so no test depends on a
//! third-party temporary-directory crate. They never read the real home directory.

pub mod pty;
mod scratch_dir;
mod tree;

use std::io;
use std::path::PathBuf;

pub use scratch_dir::ScratchDir;
pub use tree::{TreeFile, copy_tree, folder_names, read_tree};

/// The path of the running test executable, so that a test can start itself again, for instance
/// under a pseudo-terminal.
///
/// # Errors
///
/// The error of finding the executable.
pub fn test_executable() -> io::Result<PathBuf> {
    std::env::current_exe()
}

/// Starts `program` with `args` and `environment` added to this process's own, with no standard
/// streams, and gives the child to wait for or to signal.
///
/// # Errors
///
/// The error of starting the program.
pub fn start_silently(
    program: &std::path::Path,
    args: &[&str],
    environment: &[(&str, &std::ffi::OsStr)],
) -> io::Result<std::process::Child> {
    std::process::Command::new(program)
        .args(args)
        .envs(environment.iter().copied())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

/// Whether the tests run as the superuser. The superuser reads and writes whatever its mode says,
/// so a test that takes a permission away cannot show what it claims and returns early.
#[must_use]
pub fn running_as_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .is_ok_and(|output| output.stdout.trim_ascii() == b"0")
}
