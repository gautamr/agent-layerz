//! File-system access behind capability types (decision-27).

mod read_error;
mod read_only_dir;

pub use read_error::ReadError;
pub use read_only_dir::ReadOnlyDir;
