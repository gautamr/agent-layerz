//! A directory handle that only the component walk produces (decision-27, doc-6 §3.4; TASK-52).

use std::io;
use std::os::fd::OwnedFd;

use alz_core::account::AccountFolders;
use alz_core::layer::{LayerScan, Purpose};
use alz_core::path::{AbsolutePath, SafeRelPath};
use rustix::fs::{AtFlags, statat};

use crate::fs::read_below::read_file_below;
use crate::fs::read_error::ReadError;
use crate::fs::scan::scan_layer;
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

    /// Opens the directory at `path` as [`open`](Self::open) does, and gives its real path too:
    /// every link resolved, as the walk found it.
    ///
    /// # Errors
    ///
    /// The same as [`open`](Self::open).
    pub fn open_with_real_path(
        path: &AbsolutePath,
        folders: &AccountFolders,
    ) -> Result<(Self, AbsolutePath), WalkError> {
        let found = find(path, folders)?;
        if found.below.is_empty() {
            Ok((Self::from_handle(found.handle), found.existing))
        } else {
            Err(WalkError::Missing)
        }
    }

    /// Whether `name`, a single name and not a path, exists in this directory, following a link
    /// the way a test for existence does. Only the answer is used, never what it names.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        !name.contains('/') && statat(&self.handle, name, AtFlags::empty()).is_ok()
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

    /// Walks the layer below this directory and reports what it saw (spec §5.3 "File names",
    /// §7.13 "No fetch on resolve"). Every folder is listed and opened relative to the handle
    /// before it with `O_NOFOLLOW`, so no path is opened again by name and no link is followed.
    /// Folders are entered from an explicit stack, so the depth of a tree costs open handles and
    /// never call-stack frames.
    ///
    /// Every entry counts, including the entries of a folder whose name is not valid Unicode,
    /// but `.git`, `.DS_Store` and `node_modules` are neither counted nor entered. Links and
    /// entries that are not regular files, such as a pipe, a socket or a device, are counted and
    /// never opened; a `.env` file is recorded and never opened. A regular file is opened with
    /// `O_NONBLOCK`, checked with `fstat` on that handle, and read to its end, counting the bytes
    /// as they arrive. A name that is not valid Unicode (`OsStr::to_str`) is recorded as a fact
    /// and never kept. Nothing that fails stops the walk with an error: a failure is recorded as
    /// a fact of the scan, and [`scan_verdict`](alz_core::layer::scan_verdict) decides.
    ///
    /// With [`Purpose::Resolution`] the walk stops at entry 2001 and a file is read only while
    /// 16 MiB are not yet used up; with [`Purpose::Build`] there is no budget.
    #[must_use]
    pub fn scan(&self, purpose: Purpose) -> LayerScan {
        scan_layer(&self.handle, purpose)
    }
}
