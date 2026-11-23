//! Test helpers for agent-layerz: scratch directories and the file kinds a test needs to plant.
//!
//! The helpers use only the standard library and the `mkfifo` tool, so no test depends on a
//! third-party temporary-directory crate. They never read the real home directory.

mod scratch_dir;

pub use scratch_dir::ScratchDir;
