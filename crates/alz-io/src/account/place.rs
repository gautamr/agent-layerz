//! Where the working directory is, once (spec §7.13 "Paths", §9.1; decision-19, TASK-52).
//!
//! [`WorkingDirectory::resolve`] takes the captured directory by value and returns a [`Place`]:
//! the directory cannot be resolved twice, because resolving it uses it up. The place is then the
//! only thing the root walk, the user config step and the allow-state match use, so no later step
//! asks the file system where the working directory is again.

use alz_core::account::{AccountFolders, Relation};
use alz_core::path::AbsolutePath;

use crate::env::WorkingDirectory;
use crate::fs::walk::{Located, locate};

/// Where the working directory is, relative to the account folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// At or below `~/.claude*` or `~/.codex*`, as written or through a link. Nothing there was
    /// looked at.
    InAccountFolder,
    /// At or below a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, but not under
    /// `~/.claude*` or `~/.codex*`. Nothing inside it was looked at; this is the real path of the
    /// configured folder itself, which is as far as the walk went.
    InConfiguredFolder(AbsolutePath),
    /// `$HOME` itself, or above it.
    AtOrAboveHome,
    /// An ordinary directory, by its real path. It may hold a configured folder.
    Directory(AbsolutePath),
    /// There is no working directory, or it does not resolve (it is gone, it is not a folder, too
    /// many links, no permission).
    Unresolvable,
}

/// What finds the real place of a directory. The real one walks the file system; a test counts
/// the calls to show that the working directory is resolved once.
pub trait PlaceLookup {
    /// The place of `directory`, which is not an account folder as written.
    fn lookup(&self, directory: &AbsolutePath, folders: &AccountFolders) -> Place;
}

/// The lookup on the real file system: one `lstat` or `readlink` at a time, never inside an
/// account folder (spec §9.1).
#[derive(Debug, Clone, Copy, Default)]
pub struct OnDisk;

impl PlaceLookup for OnDisk {
    fn lookup(&self, directory: &AbsolutePath, folders: &AccountFolders) -> Place {
        locate(directory, folders).map_or(Place::Unresolvable, |located| match located {
            Located::Directory(real) => Place::Directory(real),
            Located::AtOrAboveHome => Place::AtOrAboveHome,
            Located::Inside { relation, at } => match relation {
                Relation::AccountFolder => Place::InAccountFolder,
                Relation::Configured => Place::InConfiguredFolder(at),
                Relation::Home => Place::AtOrAboveHome,
                Relation::Holds => Place::Directory(at),
            },
        })
    }
}

impl WorkingDirectory {
    /// Resolves the directory on the real file system. It is used up: the [`Place`] is all that
    /// later steps get.
    #[must_use]
    pub fn resolve(self, folders: &AccountFolders) -> Place {
        self.resolve_with(folders, &OnDisk)
    }

    /// Resolves the directory with `lookup`, which is called at most once.
    ///
    /// A directory that names an account folder as written, or is `$HOME` or above it as written,
    /// is placed before any file system call.
    #[must_use]
    pub fn resolve_with(self, folders: &AccountFolders, lookup: &impl PlaceLookup) -> Place {
        match self.path() {
            None => Place::Unresolvable,
            Some(directory) => match folders.relation(directory) {
                Some(Relation::AccountFolder) => Place::InAccountFolder,
                Some(Relation::Home) => Place::AtOrAboveHome,
                Some(Relation::Configured | Relation::Holds) | None => {
                    lookup.lookup(directory, folders)
                }
            },
        }
    }
}
