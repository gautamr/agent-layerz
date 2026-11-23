//! File-system access behind capability types (decision-27).

mod cache_dir;
mod cache_error;
mod dir;
mod read_below;
mod read_error;
mod read_only_dir;
mod repo_root;
mod scan;
mod walk;
mod walk_error;
mod write_below;

pub use cache_dir::{CacheDir, Published, RepositoryPaths, TempEntry};
pub use cache_error::{CacheError, IndexError};
pub use dir::Dir;
pub use read_error::ReadError;
pub use read_only_dir::ReadOnlyDir;
pub use repo_root::{Applied, RepoError, RepoRoot};
pub use walk_error::WalkError;
