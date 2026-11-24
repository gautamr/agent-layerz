//! File-system access behind capability types (decision-27).

mod cache_dir;
mod cache_error;
mod cache_view;
mod dir;
mod read_below;
mod read_error;
mod read_only_dir;
mod repo_root;
mod scan;
pub(crate) mod walk;
mod walk_error;
mod write_below;

pub use cache_dir::{CacheDir, Published, RepositoryPaths, TempEntry};
pub use cache_error::{CacheError, IndexError};
pub use cache_view::{CacheTrust, CacheView};
pub use dir::Dir;
pub use read_below::ReadFile;
pub use read_error::ReadError;
pub use read_only_dir::ReadOnlyDir;
pub use repo_root::{AdoptionBasis, Applied, Changed, GitConfigSeal, RepoError, RepoRoot};
pub use walk_error::WalkError;
