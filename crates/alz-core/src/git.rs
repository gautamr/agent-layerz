//! How alz runs git (spec §6 "Git subprocesses"), as pure decisions: the runner that carries
//! them out belongs to `alz-io`.

mod environment;

pub use environment::{CloneTransport, GitOperation, child_environment};
