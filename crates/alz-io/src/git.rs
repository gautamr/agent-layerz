//! Running git for a layer source (spec §6 "Git subprocesses", decision-24): typed calls, a git
//! version floor, deadlines that end the whole process group, and no controlling terminal.

mod call;
mod deadlines;
mod detach;
mod executable;
mod failure;
mod repository;
mod runner;
mod spawn;

pub use call::ReadScope;
pub use deadlines::Deadlines;
pub use detach::{Launcher, run_if_requested};
pub use failure::GitFailure;
pub use repository::{KeyWrite, RepositoryGit, SetupResult, Stopped};
pub use runner::{GitRunner, GitRunnerError};
