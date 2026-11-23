//! `layer.lock` (spec §5.3): the object ids, content hashes and file hashes it holds as validated
//! types, the pure part of the content hash, and the lock itself, read from and written as stable
//! JSON. Reading the file and walking a layer are R3's and R5's.

mod content_hash;
pub(crate) mod hex;
mod integrity;
mod lock_file;
mod object_id;

pub use content_hash::{
    ContentHash, DigestError, FileDigest, FileHash, LayerFiles, is_env_file, is_hashed,
    is_skipped_name,
};
pub use integrity::{Integrity, locked_entry_for, locked_sha_for, verify};
pub use lock_file::{Lock, LockError, LockField, LockedLayer};
pub use object_id::{ObjectId, ObjectIdError};
