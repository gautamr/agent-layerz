//! Resolving a source to a directory (spec §5.2, §5.3, §6): the drivers that carry out alz-core's
//! decisions with a directory handle, the cache and the git runner.

mod clone;
pub(crate) mod folders;
mod locate;
mod pinned_tree;
mod placement;
mod stack;

pub use clone::{FetchedLayer, Fetcher, GitLayer, ReadHow};
pub use locate::locate_git_source;
pub use pinned_tree::{PinnedLayer, PinnedRead, PinnedTreeReader};
pub(crate) use placement::path_of;
pub use placement::{placement_of_directory, placement_of_source, placements_of_stack};
pub use stack::StackResolver;
