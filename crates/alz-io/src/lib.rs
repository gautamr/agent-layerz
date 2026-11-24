//! Every effect of agent-layerz: file system, processes and environment, each reached through
//! a capability type (decision-23). Only this crate calls `rustix` (decision-27).

mod account;
mod account_names;
mod env;
mod fs;
mod git;
mod resolve;
mod surroundings;
mod tool;

pub use account::{
    AccountFiles, FileStatus, Home, IgnoreReason, Loaded, ResolvedAllowState, ResolvedFolder,
    ResolvedUserConfig,
};
pub use account_names::claude_account_folders;
pub use env::{EnvSnapshot, WorkingDirectory};
pub use fs::{
    AdoptionBasis, Applied, CacheDir, CacheError, Changed, Dir, GitConfigSeal, IndexError,
    Published, ReadError, ReadFile, ReadOnlyDir, RepoError, RepoRoot, RepositoryPaths, TempEntry,
    WalkError,
};
pub use git::{
    Deadlines, GitFailure, GitRunner, GitRunnerError, KeyWrite, Launcher, ReadScope, RepositoryGit,
    SetupResult, Stopped, run_if_requested,
};
pub use resolve::{
    FetchedLayer, Fetcher, GitLayer, PinnedLayer, PinnedRead, PinnedTreeReader, ReadHow,
    StackResolver, locate_git_source, placement_of_directory, placement_of_source,
    placements_of_stack,
};
pub use surroundings::{
    client_root_of, observe_ancestors, observe_client_root, observe_workspace, resolve_below,
};
pub use tool::{Purpose, ToolError, ToolOutput, ToolRunner};
