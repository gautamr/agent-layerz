//! Every cache miss of account resolution, with the words the report uses (spec §7.13, §10.3).

use crate::layer::LayerRefusal;
use crate::lock::LockError;

/// Why a layer, the lock or the cache could not be verified. A miss skips the layer step and
/// fails closed. The `code` is stable; the `phrase` is the text of `<miss>` in §10.3 and holds no
/// repository text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MissReason {
    /// A layer, the repository or the cache lies inside an account folder, or holds one.
    AccountFolder,
    /// There is no `layer.lock`.
    LockMissing,
    /// `layer.lock` is a link, too large, not valid JSON or not a valid lock.
    LockUnreadable,
    /// A stack source that cannot be located: a `..` segment in a `git:` URL, or an invalid
    /// `file://` URL.
    SourceInvalid,
    /// The lock has no entry for a source.
    LockEntryMissing,
    /// The lock has two entries for a source.
    LockEntryDuplicate,
    /// The lock's `sha` is not a commit id, a layer that needs one has none, or an unpinned
    /// `path:` layer has one.
    LockShaInvalid,
    /// The lock records another ref than the manifest names.
    LockRefMismatch,
    /// The cache directory lies inside the repository.
    CacheInRepo,
    /// A `path:` layer, or a local `git:` repository, does not exist.
    LayerDirectoryMissing,
    /// The `pathref@` index entry is missing.
    IndexMissing,
    /// The `pathref@` index entry is not a regular file holding an id as long as the commit's.
    IndexInvalid,
    /// The cache has no entry for the layer.
    CacheEntryMissing,
    /// The cache entry is a link or not a folder.
    CacheEntryNotDirectory,
    /// The layer's content hash is not the lock's.
    ContentHash,
    /// The layer is not a valid layer: a name, a `.env` file or its `layer.yaml`.
    LayerInvalid,
    /// The layers outside the repository do not make a valid stack.
    StackInvalid,
    /// More than 2000 entries or 16 MiB, a file that cannot be read, or a lock or cache entry that
    /// cannot be opened.
    Unreadable,
    /// `XDG_CACHE_HOME` points away from `~/.cache` while mise or direnv is active.
    CacheLocation,
}

impl MissReason {
    /// The stable code of the miss.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AccountFolder => "account-folder",
            Self::LockMissing => "lock-missing",
            Self::LockUnreadable => "lock-unreadable",
            Self::SourceInvalid => "source-invalid",
            Self::LockEntryMissing => "lock-entry-missing",
            Self::LockEntryDuplicate => "lock-entry-duplicate",
            Self::LockShaInvalid => "lock-sha-invalid",
            Self::LockRefMismatch => "lock-ref-mismatch",
            Self::CacheInRepo => "cache-in-repo",
            Self::LayerDirectoryMissing => "layer-directory-missing",
            Self::IndexMissing => "index-missing",
            Self::IndexInvalid => "index-invalid",
            Self::CacheEntryMissing => "cache-entry-missing",
            Self::CacheEntryNotDirectory => "cache-entry-not-directory",
            Self::ContentHash => "content-hash",
            Self::LayerInvalid => "layer-invalid",
            Self::StackInvalid => "stack-invalid",
            Self::Unreadable => "unreadable",
            Self::CacheLocation => "cache-location",
        }
    }

    /// The words of `<miss>` in the report (§10.3).
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::AccountFolder => "a layer or the repository lies inside an account folder",
            Self::LockMissing => "layer.lock is missing",
            Self::LockUnreadable => "layer.lock cannot be read",
            Self::SourceInvalid => "a stack source is not valid",
            Self::LockEntryMissing => "layer.lock has no entry for a layer",
            Self::LockEntryDuplicate => "layer.lock has two entries for a layer",
            Self::LockShaInvalid => "layer.lock holds a commit id that is not valid",
            Self::LockRefMismatch => "layer.lock records another ref than layer.yaml names",
            Self::CacheInRepo => "the layer cache is inside the repository",
            Self::LayerDirectoryMissing => "a layer directory does not exist",
            Self::IndexMissing => "the cache index entry is missing",
            Self::IndexInvalid => "the cache index entry is not valid",
            Self::CacheEntryMissing => "a cached layer is missing",
            Self::CacheEntryNotDirectory => "a cached layer is not a directory",
            Self::ContentHash => "a layer no longer matches layer.lock",
            Self::LayerInvalid => "a layer is not valid",
            Self::StackInvalid => "the stack is not valid",
            Self::Unreadable => "a layer or the lock cannot be read",
            Self::CacheLocation => {
                "XDG_CACHE_HOME points away from ~/.cache and mise or direnv is active here"
            }
        }
    }
}

impl From<LayerRefusal> for MissReason {
    /// The miss a refusal of the layer's names or files is (§5.3 "File names").
    fn from(refusal: LayerRefusal) -> Self {
        match refusal {
            LayerRefusal::TooManyEntries | LayerRefusal::Unreadable => Self::Unreadable,
            LayerRefusal::NameNotUtf8
            | LayerRefusal::NameControlOrFormat
            | LayerRefusal::NameNotSafe
            | LayerRefusal::EnvFile => Self::LayerInvalid,
        }
    }
}

/// The miss a lock that does not parse is: always [`MissReason::LockUnreadable`]. A `sha` that is
/// short, upper case or empty makes the whole lock invalid (§5.3), so it is not told apart from
/// any other bad field; [`MissReason::LockShaInvalid`] is for an entry that parses but has no
/// usable `sha` for its layer.
#[must_use]
pub const fn lock_miss(error: &LockError) -> MissReason {
    match error {
        LockError::Json(_) | LockError::Invalid(_) | LockError::FilePath { .. } => {
            MissReason::LockUnreadable
        }
    }
}

/// A miss, and the stack position that caused it when one did (the repository, the lock and the
/// cache have none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Miss {
    reason: MissReason,
    position: Option<usize>,
}

impl Miss {
    /// A miss caused by the stack entry at `position`, counted from 0.
    #[must_use]
    pub const fn at(reason: MissReason, position: usize) -> Self {
        Self {
            reason,
            position: Some(position),
        }
    }

    /// A miss that no single stack entry caused.
    #[must_use]
    pub const fn of(reason: MissReason) -> Self {
        Self {
            reason,
            position: None,
        }
    }

    /// Why.
    #[must_use]
    pub const fn reason(self) -> MissReason {
        self.reason
    }

    /// The stack entry that caused it, counted from 0.
    #[must_use]
    pub const fn position(self) -> Option<usize> {
        self.position
    }
}
