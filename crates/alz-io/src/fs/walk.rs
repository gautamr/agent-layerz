//! Answers the component walk of alz-core with real system calls (spec §6 "The layer cache" and
//! "Layers in or around an account folder", §9.1; TASK-52).
//!
//! The walk asks for one `lstat` or `readlink` at a time. Each is made relative to a handle on
//! the folder that holds the name (`fstatat` with `AT_SYMLINK_NOFOLLOW`, `readlinkat`), and the
//! handles are opened lazily, one `openat(O_DIRECTORY | O_NOFOLLOW)` per component, only for a
//! folder that something below it is asked about or that ends the walk. The walk refuses a name
//! inside an account folder before it asks, so no call is made there; a folder that only touches
//! one (the home folder, or a folder above it) is never opened as the end of a walk that is
//! refused, because the refusal comes before the handle is returned.

use std::io;
use std::os::fd::{AsFd, OwnedFd};

use alz_core::account::AccountFolders;
use alz_core::path::{AbsolutePath, Entry, LinkTarget, Name, RealPathWalk, Resolved, Step};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::fs::walk_error::WalkError;

/// Where a walk ended: a handle on the deepest folder that exists, its real path, and the names
/// below it that do not exist yet.
#[derive(Debug)]
pub(super) struct Found {
    pub(super) handle: OwnedFd,
    pub(super) existing: AbsolutePath,
    pub(super) below: Vec<Name>,
}

/// Walks `path` and returns the handle of the deepest folder that exists.
pub(super) fn find(path: &AbsolutePath, folders: &AccountFolders) -> Result<Found, WalkError> {
    let chain = Chain::at_root()?;
    drive(chain, RealPathWalk::start(path, folders))
}

/// The folders opened so far, outermost first: a cache of handles for the prefix that was last
/// asked about, so each component is opened once.
struct Chain {
    root: OwnedFd,
    opened: Vec<(String, OwnedFd)>,
}

impl Chain {
    fn at_root() -> Result<Self, WalkError> {
        fs::open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(|root| Self {
            root,
            opened: Vec::new(),
        })
        .map_err(io_error)
    }

    /// The handle of the innermost opened folder.
    fn current(&self) -> &OwnedFd {
        self.opened.last().map_or(&self.root, |(_, handle)| handle)
    }

    /// Makes the opened folders exactly `names`: keeps the common beginning, drops the rest and
    /// opens what is missing. The walk only names real folders, so a name that is a link now was
    /// swapped after the walk looked, and `O_NOFOLLOW` refuses it.
    fn align(&mut self, names: &[&str]) -> Result<(), WalkError> {
        let keep = self
            .opened
            .iter()
            .zip(names)
            .take_while(|((opened, _), name)| opened == *name)
            .count();
        self.opened.truncate(keep);
        for name in names.iter().skip(keep) {
            let handle = open_folder(self.current(), name)?;
            self.opened.push(((*name).to_owned(), handle));
        }
        Ok(())
    }

    /// The handle of the folder `path`, which the walk found to be a real folder.
    fn into_handle_of(mut self, path: &AbsolutePath) -> Result<OwnedFd, WalkError> {
        let names: Vec<&str> = path.components().collect();
        self.align(&names)?;
        Ok(self.opened.pop().map_or(self.root, |(_, handle)| handle))
    }

    /// What `lstat` finds at `path`, a name inside a real folder, asked relative to its handle.
    fn lstat(&mut self, path: &AbsolutePath) -> Result<Entry, WalkError> {
        let (parent, name) = split(path)?;
        self.align(&parent)?;
        match fs::statat(self.current(), name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(status) => Ok(match FileType::from_raw_mode(status.st_mode) {
                FileType::Directory => Entry::Directory,
                FileType::Symlink => Entry::Link,
                FileType::RegularFile
                | FileType::Fifo
                | FileType::Socket
                | FileType::CharacterDevice
                | FileType::BlockDevice
                | FileType::Unknown => Entry::Other,
            }),
            Err(Errno::NOENT) => Ok(Entry::Missing),
            Err(errno) => Err(io_error(errno)),
        }
    }

    /// Where the link at `path` points.
    fn readlink(&mut self, path: &AbsolutePath) -> Result<LinkTarget, WalkError> {
        let (parent, name) = split(path)?;
        self.align(&parent)?;
        let target = fs::readlinkat(self.current(), name, Vec::new())
            .map_err(io_error)?
            .into_string()
            .map_err(|_not_text| not_text())?;
        LinkTarget::try_from(target.as_str()).map_err(|_empty| not_text())
    }
}

fn not_text() -> WalkError {
    WalkError::Io(io::Error::new(
        io::ErrorKind::InvalidData,
        "a link target is not text",
    ))
}

fn io_error(errno: Errno) -> WalkError {
    WalkError::Io(errno.into())
}

/// The folder names above the last name of `path`, and that name. The walk never asks about `/`.
fn split(path: &AbsolutePath) -> Result<(Vec<&str>, &str), WalkError> {
    let mut names: Vec<&str> = path.components().collect();
    names
        .pop()
        .map(|name| (names, name))
        .ok_or(WalkError::NotADirectory)
}

fn open_folder(parent: &OwnedFd, name: &str) -> Result<OwnedFd, WalkError> {
    fs::openat(
        parent.as_fd(),
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| match errno {
        Errno::NOENT => WalkError::Missing,
        Errno::LOOP | Errno::NOTDIR => WalkError::NotADirectory,
        other => io_error(other),
    })
}

/// Answers each request of the walk until it ends. The loop performs the system calls.
fn drive(mut chain: Chain, mut step: Step<'_>) -> Result<Found, WalkError> {
    loop {
        step = match step {
            Step::Lstat(request) => {
                let entry = chain.lstat(request.path())?;
                request.answer(entry)
            }
            Step::Readlink(request) => {
                let target = chain.readlink(request.path())?;
                request.answer(&target)
            }
            Step::Done(Resolved::Existing(existing)) => {
                return Ok(Found {
                    handle: chain.into_handle_of(&existing)?,
                    existing,
                    below: Vec::new(),
                });
            }
            Step::Done(Resolved::Missing { existing, below }) => {
                return Ok(Found {
                    handle: chain.into_handle_of(&existing)?,
                    existing,
                    below,
                });
            }
            Step::Stopped(reason) => return Err(reason.into()),
        };
    }
}
