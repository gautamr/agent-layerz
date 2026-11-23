//! How alz runs git (spec §6 "Git subprocesses"), as pure decisions: the runner that carries
//! them out belongs to `alz-io`.

mod batch;
mod blob_id;
mod environment;
mod failure;
mod object_kind;
mod tree;
mod tree_check;
mod tree_entry;
mod version;

pub use batch::{ObjectCheck, ObjectContent};
pub use blob_id::{BlobId, ObjectFormat};
pub use environment::{CloneTransport, GitOperation, child_environment};
pub use failure::FetchFailureLine;
pub use object_kind::{ObjectKind, UnreadableOutput};
pub use tree::{ListedFile, ListingRefusal, TreeListing};
pub use tree_check::{EntryDifference, verify_entry};
pub use tree_entry::{EntryMode, TreeEntry};
pub use version::{GitVersion, VersionRefusal};
