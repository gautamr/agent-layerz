//! The names of the folders inside the cache (spec §6): one per kind of entry. Printing a name
//! gives the folder's name; parsing it back gives the same value, so two different entries can
//! never share a folder.

use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lock::{ObjectId, hex};
use crate::path::AbsolutePath;

/// The SHA-256 of what is cloned, as the `git-` entry names it. A `github:` source is cloned from
/// `https://github.com/<owner>/<repo>.git`, a `git:` source from its URL as written or the
/// canonical absolute path of a local repository, and the text is hashed as it is, not rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CloneKey([u8; 32]);

impl CloneKey {
    /// The key of the text that is passed to `git clone`.
    #[must_use]
    pub fn of(clone_argument: &str) -> Self {
        Self(Sha256::digest(clone_argument.as_bytes()).into())
    }
}

/// The SHA-256 of the real path of a pinned `path:` layer's directory, as the `pathref@` index
/// entry names it. A distinct type from [`CloneKey`], so a directory is never keyed as a clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DirKey([u8; 32]);

impl DirKey {
    /// The key of the layer directory's real path.
    #[must_use]
    pub fn of(real_dir: &AbsolutePath) -> Self {
        Self(Sha256::digest(real_dir.as_str().as_bytes()).into())
    }
}

/// A folder name inside the layer cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryName {
    /// `git-<sha256 hex>@<commit>`: a `github:` or `git:` layer, cloned and checked out.
    Git {
        /// What was cloned.
        key: CloneKey,
        /// The commit that was checked out.
        commit: ObjectId,
    },
    /// `tree2@<tree>`: a pinned `path:` layer, written from the blobs of its git tree.
    Tree(ObjectId),
    /// `pathref@<sha256 hex>-<commit>`: the index entry that lets account resolution find the
    /// `tree2@` entry of a pinned `path:` layer without git.
    PathIndex {
        /// The real path of the layer directory.
        dir: DirKey,
        /// The commit the layer's `ref` resolved to.
        commit: ObjectId,
    },
}

/// Text that is not the name of a cache entry. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a cache entry name")]
pub struct EntryNameError;

impl fmt::Display for EntryName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Git { key, commit } => write!(formatter, "git-{}@{commit}", hex::encode(&key.0)),
            Self::Tree(tree) => write!(formatter, "tree2@{tree}"),
            Self::PathIndex { dir, commit } => {
                write!(formatter, "pathref@{}-{commit}", hex::encode(&dir.0))
            }
        }
    }
}

impl TryFrom<&str> for EntryName {
    type Error = EntryNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let object = |text: &str| ObjectId::try_from(text).map_err(|_not_an_id| EntryNameError);
        let digest = |text: &str| hex::decode::<32>(text).ok_or(EntryNameError);
        match (
            text.strip_prefix("git-"),
            text.strip_prefix("tree2@"),
            text.strip_prefix("pathref@"),
        ) {
            (Some(rest), _, _) => {
                let (key, commit) = rest.split_once('@').ok_or(EntryNameError)?;
                Ok(Self::Git {
                    key: CloneKey(digest(key)?),
                    commit: object(commit)?,
                })
            }
            (_, Some(tree), _) => object(tree).map(Self::Tree),
            (_, _, Some(rest)) => {
                let (dir, commit) = rest.split_once('-').ok_or(EntryNameError)?;
                Ok(Self::PathIndex {
                    dir: DirKey(digest(dir)?),
                    commit: object(commit)?,
                })
            }
            (None, None, None) => Err(EntryNameError),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert_eq, prop_oneof, proptest};

    use super::{CloneKey, DirKey, EntryName, EntryNameError};
    use crate::lock::ObjectId;
    use crate::path::AbsolutePath;

    const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";

    fn commit() -> ObjectId {
        ObjectId::try_from(SHA1).expect("an object id")
    }

    fn absolute(text: &str) -> AbsolutePath {
        AbsolutePath::try_from(text).expect("absolute")
    }

    #[test]
    fn a_github_clone_is_named_after_the_hash_of_its_url_and_the_commit() {
        let key = CloneKey::of("https://github.com/owner/repo.git");
        assert_eq!(
            EntryName::Git {
                key,
                commit: commit()
            }
            .to_string(),
            format!("git-bc40893b43beea6303cb93d2e61df787840b4e09dcf293506e68491b9a082686@{SHA1}")
        );
        // The standard SHA-256 vector: the key is the hash of the text and nothing else.
        assert_eq!(
            EntryName::Git {
                key: CloneKey::of("abc"),
                commit: commit()
            }
            .to_string(),
            format!("git-ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad@{SHA1}")
        );
    }

    #[test]
    fn a_pinned_path_layer_has_a_tree_entry_and_an_index_entry() {
        assert_eq!(
            EntryName::Tree(commit()).to_string(),
            format!("tree2@{SHA1}")
        );
        assert_eq!(
            EntryName::PathIndex {
                dir: DirKey::of(&absolute("/srv/layers/hub")),
                commit: commit()
            }
            .to_string(),
            format!(
                "pathref@ec7b7898953e6b21bbc4b505e7277640525bb4b66e3874f87090fc03b3904394-{SHA1}"
            )
        );
    }

    #[test]
    fn a_text_that_is_not_an_entry_name_is_refused() {
        let key = "ab".repeat(32);
        for bad in [
            "",
            "git-",
            "tree@0123456789abcdef0123456789abcdef01234567",
            "tree2@",
            "tree2@xyz",
            "tree2@0123456789ABCDEF0123456789abcdef01234567",
            &format!("git-{key}"),
            &format!("git-{key}@"),
            &format!("git-{key}@nothex"),
            &format!("git-{}@{SHA1}", "ab".repeat(31)),
            &format!("pathref@{key}@{SHA1}"),
            &format!("pathref@{key}-{SHA1}x"),
            &format!(".tmp-{SHA1}"),
        ] {
            assert_eq!(EntryName::try_from(bad), Err(EntryNameError), "{bad:?}");
        }
    }

    fn any_object() -> impl Strategy<Value = ObjectId> {
        prop_oneof![
            any::<[u8; 20]>().prop_map(ObjectId::Sha1),
            any::<[u8; 32]>().prop_map(ObjectId::Sha256),
        ]
    }

    fn any_name() -> impl Strategy<Value = EntryName> {
        prop_oneof![
            ("[ -~]{0,24}", any_object()).prop_map(|(argument, commit)| EntryName::Git {
                key: CloneKey::of(&argument),
                commit
            }),
            any_object().prop_map(EntryName::Tree),
            ("[a-z/]{0,24}", any_object()).prop_map(|(path, commit)| EntryName::PathIndex {
                dir: DirKey::of(&absolute(&format!("/{path}"))),
                commit
            }),
        ]
    }

    proptest! {
        /// Printing and parsing are inverse, so the name is injective over its inputs: two
        /// different (clone argument, commit), tree or (directory, commit) never share a folder.
        #[test]
        fn a_name_prints_back_as_it_was_read(name in any_name()) {
            prop_assert_eq!(EntryName::try_from(name.to_string().as_str()), Ok(name));
        }

        #[test]
        fn two_names_print_the_same_exactly_when_they_are_equal(
            left in any_name(),
            right in any_name(),
        ) {
            prop_assert_eq!(left == right, left.to_string() == right.to_string());
        }

        #[test]
        fn the_same_clone_at_another_commit_is_another_entry(
            argument in "[ -~]{0,24}",
            first in any_object(),
            second in any_object(),
        ) {
            let key = CloneKey::of(&argument);
            prop_assert_eq!(
                first == second,
                EntryName::Git { key, commit: first }.to_string()
                    == EntryName::Git { key, commit: second }.to_string()
            );
        }
    }
}
