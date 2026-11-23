//! Reading a regular file below a directory handle, never through a link (decision-27).

use std::fs::File;
use std::io::Read;
use std::os::fd::OwnedFd;

use alz_core::path::SafeRelPath;
use rustix::fs::{self, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::fs::read_error::ReadError;

/// Reads the regular file at `path` below `root`, at most `cap` bytes.
///
/// Every component is opened relative to the handle before it, with `O_NOFOLLOW`, so no link is
/// followed. Every component, the file included, is opened with `O_NONBLOCK`, so a pipe or a
/// device cannot block the open wherever it lies in the path, and the file with `O_NOCTTY`, so a
/// terminal device never becomes the process's controlling terminal. What was opened is checked
/// with `fstat` on that handle: only a regular file is read (decision-27, amended). `st_size` is
/// not trusted: at most `cap + 1` bytes are read, and the file is refused when the last of them
/// arrives.
///
/// # Errors
///
/// [`ReadError::Missing`] when a component does not exist, [`ReadError::NotRegular`] when the
/// last component is a link, a folder or anything else but a regular file (or an earlier one is a
/// link or not a folder), [`ReadError::TooLarge`] when the file is longer than `cap` bytes, and
/// [`ReadError::Io`] for any other failure.
pub(super) fn read_file_below(
    root: &OwnedFd,
    path: &SafeRelPath,
    cap: usize,
) -> Result<Vec<u8>, ReadError> {
    let segments: Vec<&str> = path.segments().collect();
    let (name, parents) = segments.split_last().ok_or(ReadError::Missing)?;
    let directory = parents.iter().try_fold(
        root.try_clone().map_err(ReadError::Io)?,
        |directory, segment| {
            open_below(
                &directory,
                segment,
                OFlags::DIRECTORY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            )
        },
    )?;
    let file = open_below(
        &directory,
        name,
        OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
    )?;
    require_regular(&file)?;
    read_at_most(File::from(file), cap)
}

/// Opens `name` read-only relative to `directory`. The refusal of a link surfaces as `ELOOP`
/// (or `ENOTDIR` with `O_DIRECTORY`), which is [`ReadError::NotRegular`].
fn open_below(directory: &OwnedFd, name: &str, flags: OFlags) -> Result<OwnedFd, ReadError> {
    fs::openat(directory, name, OFlags::RDONLY | flags, Mode::empty()).map_err(
        |errno| match errno {
            Errno::NOENT => ReadError::Missing,
            Errno::LOOP | Errno::NOTDIR => ReadError::NotRegular,
            other => ReadError::Io(other.into()),
        },
    )
}

/// The trust check of decision-27: `fstat` on the handle that is read, never on a name.
fn require_regular(file: &OwnedFd) -> Result<(), ReadError> {
    let status = fs::fstat(file).map_err(|errno| ReadError::Io(errno.into()))?;
    match FileType::from_raw_mode(status.st_mode) {
        FileType::RegularFile => Ok(()),
        FileType::Directory
        | FileType::Symlink
        | FileType::Fifo
        | FileType::Socket
        | FileType::CharacterDevice
        | FileType::BlockDevice
        | FileType::Unknown => Err(ReadError::NotRegular),
    }
}

/// Reads `source` to its end, taking at most `cap + 1` bytes, and refuses it when the last of
/// those arrives. The source's own size is never consulted.
pub(super) fn read_at_most(source: impl Read, cap: usize) -> Result<Vec<u8>, ReadError> {
    let limit = u64::try_from(cap).map_or(u64::MAX, |cap| cap.saturating_add(1));
    let mut bytes = Vec::new();
    source
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(ReadError::Io)?;
    if bytes.len() > cap {
        Err(ReadError::TooLarge)
    } else {
        Ok(bytes)
    }
}
