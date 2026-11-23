//! Every effect of agent-layerz: file system, processes and environment, each reached through
//! a capability type (decision-23). Only this crate calls `rustix` (decision-27).

mod env;
mod fs;
mod git;
mod resolve;

pub use env::EnvSnapshot;
pub use fs::{
    CacheDir, CacheError, Dir, IndexError, Published, ReadError, ReadOnlyDir, RepositoryPaths,
    TempEntry, WalkError,
};
pub use git::{
    Deadlines, GitFailure, GitRunner, GitRunnerError, Launcher, ReadScope, run_if_requested,
};
pub use resolve::{FetchedLayer, Fetcher, GitLayer, ReadHow, locate_git_source};
