//! How alz runs git (spec §6 "Git subprocesses"), as pure decisions: the runner that carries
//! them out belongs to `alz-io`.

mod environment;
mod failure;

pub use environment::{CloneTransport, GitOperation, child_environment};
pub use failure::FetchFailureLine;
