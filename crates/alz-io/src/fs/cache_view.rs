//! The layer cache as account resolution reads it (spec §7.13 "No fetch on resolve"; TASK-55.7.8):
//! a root that is opened and never created, and entries that are only read. The type has no method
//! that writes, creates or removes anything, so a resolution that holds only this cannot change the
//! cache.

use std::os::fd::OwnedFd;

use alz_core::account::{AccountFolders, IndexEntry, MissReason, lies_within};
use alz_core::cache::{CacheBase, CacheLocation, DirKey, EntryName};
use alz_core::lock::ObjectId;
use alz_core::path::AbsolutePath;

use rustix::fs::{self, Mode};

use crate::fs::cache_dir::{entry_below, index_below, is_private, joined};
use crate::fs::cache_error::CacheError;
use crate::fs::dir::Dir;
use crate::fs::walk::find;
use crate::fs::walk_error::WalkError;

/// A read-only handle on the cache root, or the knowledge that there is none yet. A root that does
/// not exist holds no entry, so every read of an entry is a miss.
#[derive(Debug)]
pub struct CacheView {
    handle: Option<OwnedFd>,
}

/// Where the cache is, for the checks that need the file system: the root, and the default root
/// `XDG_CACHE_HOME` may have moved it from.
#[derive(Debug, Clone)]
pub struct CacheTrust {
    location: CacheLocation,
    default_location: Option<CacheLocation>,
    version_manager_active: bool,
}

impl CacheTrust {
    /// The cache at `location`. `default_location` is `$HOME/.cache/agent-layerz`, `None` when
    /// there is no usable `HOME`. `version_manager_active` is whether `__MISE_DIFF` or
    /// `DIRENV_DIFF` exists.
    #[must_use]
    pub const fn new(
        location: CacheLocation,
        default_location: Option<CacheLocation>,
        version_manager_active: bool,
    ) -> Self {
        Self {
            location,
            default_location,
            version_manager_active,
        }
    }
}

impl CacheView {
    /// Opens the cache root of `trust` for reading, creating nothing.
    ///
    /// # Errors
    ///
    /// [`MissReason::AccountFolder`] when the root touches an account folder or the home folder,
    /// [`MissReason::CacheInRepo`] when it lies inside `repository` (a real path),
    /// [`MissReason::CacheLocation`] when `XDG_CACHE_HOME` puts it away from the default while
    /// mise or direnv is active (compared as real paths), and [`MissReason::Unreadable`] when
    /// it cannot be walked, or when it exists but this user does not own it or group or others can
    /// write it (the rule of §6, read with `fstat` on the handle that is kept).
    pub fn open(
        trust: &CacheTrust,
        folders: &AccountFolders,
        repository: &AbsolutePath,
    ) -> Result<Self, MissReason> {
        let found = find(trust.location.path(), folders).map_err(|error| walk_miss(&error))?;
        let real =
            joined(&found.existing, &found.below).map_err(|_unusable| MissReason::Unreadable)?;
        if folders.touches(&real) {
            return Err(MissReason::AccountFolder);
        }
        if lies_within(&real, repository) {
            return Err(MissReason::CacheInRepo);
        }
        if moved_from_default(trust, &real, folders) {
            return Err(MissReason::CacheLocation);
        }
        // The root that is kept is held to the rule of §6: what a stranger can write there would be
        // read as a layer. A root that does not exist holds nothing, so there is nothing to check.
        let handle = found.below.is_empty().then_some(found.handle);
        match handle.as_ref().map(fs::fstat) {
            Some(Ok(status)) if !is_private(status.st_uid, Mode::from_raw_mode(status.st_mode)) => {
                Err(MissReason::Unreadable)
            }
            Some(Err(_unreadable)) => Err(MissReason::Unreadable),
            Some(Ok(_)) | None => Ok(Self { handle }),
        }
    }

    /// The entry called `name`.
    ///
    /// # Errors
    ///
    /// [`MissReason::CacheEntryMissing`] when there is none,
    /// [`MissReason::CacheEntryNotDirectory`] for a link or anything but a folder, which is never
    /// followed, and [`MissReason::Unreadable`] for an entry another user owns or any other
    /// failure.
    pub fn entry(&self, name: &EntryName) -> Result<Dir, MissReason> {
        self.handle
            .as_ref()
            .map_or(Ok(None), |root| entry_below(root, name))
            .map_err(|error| match error {
                CacheError::EntryNotDirectory(_) => MissReason::CacheEntryNotDirectory,
                CacheError::TouchesAccountFolder
                | CacheError::InsideRepository
                | CacheError::NotPrivate
                | CacheError::Unusable(_)
                | CacheError::EntryNotOwned(_)
                | CacheError::TempEntryReplaced
                | CacheError::EntryVanished(_)
                | CacheError::Io(_) => MissReason::Unreadable,
            })
            .and_then(|found| found.ok_or(MissReason::CacheEntryMissing))
    }

    /// What the index entry for the pinned `path:` layer at `dir` and `commit` holds.
    #[must_use]
    pub fn index(&self, dir: DirKey, commit: ObjectId) -> IndexEntry {
        self.handle
            .as_ref()
            .map_or(IndexEntry::Missing, |root| index_below(root, dir, commit))
    }
}

fn walk_miss(error: &WalkError) -> MissReason {
    match error {
        WalkError::Refused(_) => MissReason::AccountFolder,
        WalkError::Missing
        | WalkError::NotADirectory
        | WalkError::TooManyLinks
        | WalkError::Io(_) => MissReason::Unreadable,
    }
}

/// Whether `XDG_CACHE_HOME` put the cache elsewhere than the default while mise or direnv is
/// active: they can set it, so what a stranger wrote there would be read as a layer.
fn moved_from_default(trust: &CacheTrust, real: &AbsolutePath, folders: &AccountFolders) -> bool {
    match trust.location.base() {
        CacheBase::Home => false,
        CacheBase::XdgCacheHome => {
            trust.version_manager_active
                && trust.default_location.as_ref().is_none_or(|default| {
                    find(default.path(), folders)
                        .ok()
                        .and_then(|found| joined(&found.existing, &found.below).ok())
                        .is_none_or(|default_real| &default_real != real)
                })
        }
    }
}
