//! Looking at one path below the repository root, one component at a time, never through a link
//! (decision-27).

use std::fs::File;
use std::io;
use std::os::fd::OwnedFd;

use alz_core::path::SafeRelPath;
use alz_core::render::write_plan::{MAX_COMPARED_BYTES, Observed};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;
use rustix::path::Arg;

use crate::fs::read_below::{read_at_most, require_regular};
use crate::fs::read_error::ReadError;
use crate::fs::repo_root::error::RepoError;

/// What a step down one folder found.
pub(super) enum Below {
    /// The folder, opened.
    Folder(OwnedFd),
    /// Not a folder to go into: the path is absent, or a link or something else on the way.
    Stop(Observed),
}

/// Opens the folder `name` in `directory` with `O_NOFOLLOW`. A name that is absent, a link or not
/// a folder stops the walk with what it is.
pub(super) fn step_into<N: Arg + Copy>(directory: &OwnedFd, name: N) -> io::Result<Below> {
    match fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(handle) => Ok(Below::Folder(handle)),
        Err(Errno::NOENT) => Ok(Below::Stop(Observed::Absent)),
        Err(Errno::LOOP | Errno::NOTDIR) => classify(directory, name).map(Below::Stop),
        Err(errno) => Err(errno.into()),
    }
}

/// What the entry `name` is, looked at with `lstat` after the open of it was refused.
fn classify<N: Arg>(directory: &OwnedFd, name: N) -> io::Result<Observed> {
    match fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(status) => Ok(match FileType::from_raw_mode(status.st_mode) {
            FileType::Symlink => Observed::Link,
            FileType::RegularFile
            | FileType::Directory
            | FileType::Fifo
            | FileType::Socket
            | FileType::CharacterDevice
            | FileType::BlockDevice
            | FileType::Unknown => Observed::NotRegular,
        }),
        Err(Errno::NOENT) => Ok(Observed::Absent),
        Err(errno) => Err(errno.into()),
    }
}

/// Opens the folder at `segments` below `root`, or says what stopped the walk.
pub(super) fn folder_at<'a>(
    root: &OwnedFd,
    segments: impl IntoIterator<Item = &'a str>,
) -> io::Result<Below> {
    segments.into_iter().try_fold(
        Below::Folder(root.try_clone()?),
        |below, segment| match below {
            Below::Folder(directory) => step_into(&directory, segment),
            stopped @ Below::Stop(_) => Ok(stopped),
        },
    )
}

/// What is at `path` below `root`: nothing, a regular file with its bytes, or an entry that is
/// not alz's to touch. Every component is opened with `O_NOFOLLOW | O_NONBLOCK`, so a link is
/// never followed and a FIFO or a device cannot block the open; what was opened is checked with
/// `fstat` on the handle that is read.
pub(super) fn probe(root: &OwnedFd, path: &SafeRelPath) -> Result<Observed, RepoError> {
    let segments: Vec<&str> = path.segments().collect();
    let Some((name, parents)) = segments.split_last() else {
        return Ok(Observed::Absent);
    };
    let failed = |source| RepoError::io("reading", path, source);
    match folder_at(root, parents.iter().copied()).map_err(failed)? {
        Below::Stop(observed) => Ok(observed),
        Below::Folder(directory) => file_in(&directory, name).map_err(failed),
    }
}

/// The last component: opened without following a link, then checked.
fn file_in(directory: &OwnedFd, name: &str) -> io::Result<Observed> {
    match fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(handle) => read_regular(handle),
        Err(Errno::NOENT) => Ok(Observed::Absent),
        Err(Errno::LOOP | Errno::NXIO | Errno::NOTDIR) => classify(directory, name),
        Err(errno) => Err(errno.into()),
    }
}

fn read_regular(handle: OwnedFd) -> io::Result<Observed> {
    match require_regular(&handle) {
        Ok(mode) => {
            let executable = mode.as_raw_mode() & 0o100 != 0;
            match read_at_most(File::from(handle), MAX_COMPARED_BYTES) {
                Ok(bytes) => Ok(Observed::Regular { bytes, executable }),
                Err(ReadError::TooLarge) => Ok(Observed::TooLarge),
                Err(other) => Err(unexpected(other)),
            }
        }
        Err(ReadError::NotRegular) => Ok(Observed::NotRegular),
        Err(other) => Err(unexpected(other)),
    }
}

fn unexpected(error: ReadError) -> io::Error {
    match error {
        ReadError::Io(source) => source,
        ReadError::Missing | ReadError::NotRegular | ReadError::TooLarge => {
            io::Error::other("an unexpected state of the entry")
        }
    }
}
