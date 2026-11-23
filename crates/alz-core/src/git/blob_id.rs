//! The id git gives a blob, computed in Rust (spec §6 "Git subprocesses"; decision-36): the hash
//! of `blob <length>` and a NUL, followed by the bytes. The hash follows the repository: SHA-1 for
//! a repository whose ids have 40 characters and SHA-256 for one whose ids have 64. It is never
//! used for a check that alz defines itself; content hashes and cache names are SHA-256
//! (decision-34).

use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::lock::ObjectId;

/// The hash a repository names its objects with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectFormat {
    /// SHA-1, 40 hexadecimal characters.
    Sha1,
    /// SHA-256, 64 hexadecimal characters.
    Sha256,
}

impl ObjectFormat {
    /// The format that `id` is written in: the repository's own, since its commit and tree ids
    /// have this form.
    #[must_use]
    pub const fn of(id: &ObjectId) -> Self {
        match id {
            ObjectId::Sha1(_) => Self::Sha1,
            ObjectId::Sha256(_) => Self::Sha256,
        }
    }
}

/// The id of a blob: what git names the file `bytes` with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobId(ObjectId);

impl BlobId {
    /// The id of the blob whose content is `bytes`, in `format`.
    #[must_use]
    pub fn of(format: ObjectFormat, bytes: &[u8]) -> Self {
        let header = format!("blob {}\0", bytes.len());
        Self(match format {
            ObjectFormat::Sha1 => ObjectId::Sha1(
                Sha1::new_with_prefix(header.as_bytes())
                    .chain_update(bytes)
                    .finalize()
                    .into(),
            ),
            ObjectFormat::Sha256 => ObjectId::Sha256(
                Sha256::new_with_prefix(header.as_bytes())
                    .chain_update(bytes)
                    .finalize()
                    .into(),
            ),
        })
    }

    /// The id as an object id, to compare with what `git ls-tree` lists.
    #[must_use]
    pub const fn id(self) -> ObjectId {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{BlobId, ObjectFormat};
    use crate::lock::ObjectId;

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).expect("an id")
    }

    #[test]
    fn the_empty_blob_has_the_id_git_gives_it_in_both_formats() {
        assert_eq!(
            BlobId::of(ObjectFormat::Sha1, b"").id(),
            id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
        );
        assert_eq!(
            BlobId::of(ObjectFormat::Sha256, b"").id(),
            id("473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813")
        );
    }

    #[test]
    fn a_blob_with_content_has_the_id_of_git_hash_object() {
        // `printf 'hello\n' | git hash-object --stdin`.
        assert_eq!(
            BlobId::of(ObjectFormat::Sha1, b"hello\n").id(),
            id("ce013625030ba8dba906f756967f9e9ca394464a")
        );
    }

    #[test]
    fn the_format_is_the_one_the_id_is_written_in() {
        assert_eq!(
            ObjectFormat::of(&id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
            ObjectFormat::Sha1
        );
        assert_eq!(
            ObjectFormat::of(&id(&"ab".repeat(32))),
            ObjectFormat::Sha256
        );
    }
}
