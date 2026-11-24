//! Which lock entry covers a source and where its layer is read from (spec §7.13 "No fetch on
//! resolve"): decided from the manifest entry and the lock alone.

use crate::cache::DirKey;
use crate::cache::EntryName;
use crate::lock::{Lock, LockedLayer, ObjectId};
use crate::stack::{Source, StackEntry};

use super::MissReason;

/// Where a layer outside the repository is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadPlan {
    /// A `github:` or `git:` layer: the `git-` entry of the commit the lock pins.
    Git(ObjectId),
    /// A pinned `path:` layer: the `pathref@` index entry of the commit, which names the `tree2@`
    /// entry.
    Indexed(ObjectId),
    /// An unpinned `path:` layer: the layer directory itself.
    InPlace,
}

impl ReadPlan {
    /// The cache entry that is read directly, for a layer that is not read in place or through
    /// the index. `clone` is the key of what is cloned.
    #[must_use]
    pub const fn git_entry(self, key: crate::cache::CloneKey) -> Option<EntryName> {
        match self {
            Self::Git(commit) => Some(EntryName::Git { key, commit }),
            Self::Indexed(_) | Self::InPlace => None,
        }
    }

    /// The index entry to read for the layer at `real_dir`.
    #[must_use]
    pub fn index_entry(self, real_dir: &crate::path::AbsolutePath) -> Option<EntryName> {
        match self {
            Self::Indexed(commit) => Some(EntryName::PathIndex {
                dir: DirKey::of(real_dir),
                commit,
            }),
            Self::Git(_) | Self::InPlace => None,
        }
    }
}

/// The lock entry for `source`: exactly one must exist.
///
/// # Errors
///
/// [`MissReason::LockEntryMissing`] and [`MissReason::LockEntryDuplicate`].
pub fn lock_entry<'l>(lock: &'l Lock, source: &Source) -> Result<&'l LockedLayer, MissReason> {
    let mut matches = lock.layers.iter().filter(|layer| &layer.source == source);
    match (matches.next(), matches.next()) {
        (None, _) => Err(MissReason::LockEntryMissing),
        (Some(_), Some(_)) => Err(MissReason::LockEntryDuplicate),
        (Some(entry), None) => Ok(entry),
    }
}

/// Where the layer of `entry` is read from, given its lock entry. The commit is only ever the
/// lock's `sha`, which is an [`ObjectId`] by construction (40 or 64 lowercase hexadecimal
/// characters).
///
/// # Errors
///
/// [`MissReason::LockShaInvalid`] for a `git:`, `github:` or pinned `path:` layer whose entry has
/// no `sha`, and for an unpinned `path:` layer whose entry has one;
/// [`MissReason::LockRefMismatch`] when a pinned `path:` layer's `ref` is not the entry's `rev`
/// (the lock is for another ref until `alz build` runs).
pub fn read_plan(entry: &StackEntry, locked: &LockedLayer) -> Result<ReadPlan, MissReason> {
    match (entry.source(), entry.pin()) {
        (Source::Git { .. } | Source::GitHub { .. }, _) => locked
            .sha
            .map(ReadPlan::Git)
            .ok_or(MissReason::LockShaInvalid),
        (Source::Path { .. }, Some(pin)) if locked.rev.as_ref() != Some(pin) => {
            Err(MissReason::LockRefMismatch)
        }
        (Source::Path { .. }, Some(_)) => locked
            .sha
            .map(ReadPlan::Indexed)
            .ok_or(MissReason::LockShaInvalid),
        (Source::Path { .. }, None) => match locked.sha {
            Some(_) => Err(MissReason::LockShaInvalid),
            None => Ok(ReadPlan::InPlace),
        },
    }
}

/// Whether a `git:` or `github:` source has a `..` segment in its URL, which the operating system
/// could resolve through a link (a miss, [`MissReason::SourceInvalid`]).
#[must_use]
pub fn has_parent_segment(source: &Source) -> bool {
    match source {
        Source::Git { url, .. } => url.as_str().split('/').any(|segment| segment == ".."),
        Source::GitHub { .. } | Source::Path { .. } => false,
    }
}

/// What the `pathref@` index entry held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexEntry {
    /// There is no entry.
    Missing,
    /// The entry is a link, a folder, too long or not an object id.
    Invalid,
    /// A regular file whose whole content is this id.
    Tree(ObjectId),
    /// A regular file that another user owns: what that user wrote is not read as a layer.
    NotOwned,
}

impl IndexEntry {
    /// The tree the entry names for `commit`: the id must be as long as the commit's.
    ///
    /// # Errors
    ///
    /// [`MissReason::IndexMissing`], [`MissReason::IndexInvalid`] and, for an entry that another
    /// user owns, [`MissReason::Unreadable`].
    pub const fn tree_for(self, commit: ObjectId) -> Result<ObjectId, MissReason> {
        match (self, commit) {
            (Self::Missing, _) => Err(MissReason::IndexMissing),
            (Self::NotOwned, _) => Err(MissReason::Unreadable),
            (Self::Tree(tree @ ObjectId::Sha1(_)), ObjectId::Sha1(_))
            | (Self::Tree(tree @ ObjectId::Sha256(_)), ObjectId::Sha256(_)) => Ok(tree),
            (Self::Invalid | Self::Tree(_), _) => Err(MissReason::IndexInvalid),
        }
    }
}
