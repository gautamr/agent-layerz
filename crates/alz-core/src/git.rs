//! How alz runs git (spec §6 "Git subprocesses"), as pure decisions: the runner that carries
//! them out belongs to `alz-io`.

mod batch;
mod environment;
mod failure;
mod object_kind;
mod tree_entry;
mod version;

pub use batch::{ObjectCheck, ObjectContent};
pub use environment::{CloneTransport, GitOperation, child_environment};
pub use failure::FetchFailureLine;
pub use object_kind::{ObjectKind, UnreadableOutput};
pub use tree_entry::{EntryMode, TreeEntry};
pub use version::{GitVersion, VersionRefusal};
