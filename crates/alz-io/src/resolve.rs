//! Resolving a source to a directory (spec §5.2, §5.3, §6): the drivers that carry out alz-core's
//! decisions with a directory handle, the cache and the git runner.

mod clone;
mod folders;
mod locate;
mod pinned_tree;
mod stack;

pub use clone::{FetchedLayer, Fetcher, GitLayer, ReadHow};
pub use locate::locate_git_source;
pub use pinned_tree::{PinnedLayer, PinnedRead, PinnedTreeReader};
pub use stack::StackResolver;
