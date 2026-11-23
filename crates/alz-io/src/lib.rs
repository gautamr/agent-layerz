//! Every effect of agent-layerz: file system, processes and environment, each reached through
//! a capability type (decision-23). Only this crate calls `rustix` (decision-27).

mod env;
mod fs;
mod git;

pub use env::EnvSnapshot;
pub use fs::{ReadError, ReadOnlyDir};
pub use git::{
    Deadlines, GitFailure, GitRunner, GitRunnerError, Launcher, ReadScope, run_if_requested,
};
