//! A directory handle that only the component walk produces (decision-27, doc-6 §3.4; TASK-52).

use std::io;
use std::os::fd::OwnedFd;

use alz_core::account::AccountFolders;
use alz_core::path::{AbsolutePath, SafeRelPath};

use crate::fs::read_below::read_file_below;
use crate::fs::read_error::ReadError;
use crate::fs::walk::find;
use crate::fs::walk_error::WalkError;

/// A handle on a directory found by the component walk, or inside such a directory. It has no
/// method that writes, creates or removes anything. A name inside it is opened relative to the
/// handle and never through a link, and what is checked is checked with `fstat` on the handle
/// that is read, so a name swapped for a link between two calls changes nothing.
///
/// [`ReadOnlyDir::open`](crate::ReadOnlyDir::open) stays for a path a person typed; the directory
/// of a manifest's `path:` layer always comes through here.
#[derive(Debug)]
pub struct Dir {
    handle: OwnedFd,
}

impl Dir {
    /// Opens the directory at `path`, which is resolved one component at a time. Every link in
    /// it is followed by the walk itself, and each resolved prefix is checked against the account
    /// folders before the next name is looked at (spec §6 "Layers in or around an account
    /// folder"): a directory inside an account folder, the home folder or above it, or one that
    /// holds a `CLAUDE_CONFIG_DIR` or `CODEX_HOME` folder is refused before any file in it is
    /// opened, whether or not a link leads there.
    ///
    /// # Errors
    ///
    /// [`WalkError::Refused`] for such a path, [`WalkError::Missing`] or
    /// [`WalkError::NotADirectory`] when the directory does not exist or is not one,
    /// [`WalkError::TooManyLinks`], and [`WalkError::Io`] for any other failure.
    pub fn open(path: &AbsolutePath, folders: &AccountFolders) -> Result<Self, WalkError> {
        let found = find(path, folders)?;
        if found.below.is_empty() {
            Ok(Self::from_handle(found.handle))
        } else {
            Err(WalkError::Missing)
        }
    }

    pub(super) const fn from_handle(handle: OwnedFd) -> Self {
        Self { handle }
    }

    /// A second handle on the same directory.
    pub(super) fn try_clone(&self) -> io::Result<Self> {
        self.handle.try_clone().map(Self::from_handle)
    }

    /// Reads the regular file at `path` below the directory, at most `cap` bytes: no link is
    /// followed, nothing that is not a regular file is read, and at most `cap + 1` bytes are
    /// taken whatever `st_size` says.
    ///
    /// # Errors
    ///
    /// [`ReadError::Missing`] when a component does not exist, [`ReadError::NotRegular`] when
    /// the last component is a link, a folder or anything else but a regular file (or an earlier
    /// one is a link or not a folder), [`ReadError::TooLarge`] when the file is longer than
    /// `cap` bytes, and [`ReadError::Io`] for any other failure.
    pub fn read_file(&self, path: &SafeRelPath, cap: usize) -> Result<Vec<u8>, ReadError> {
        read_file_below(&self.handle, path, cap)
    }
}
