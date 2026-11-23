//! The walk of a layer directory through directory handles (spec §5.3, §7.13; TASK-69).
//!
//! Every folder is opened relative to the handle of its parent, and its entries are listed from
//! that handle, so no path is opened again by name. The walk records facts in a
//! [`LayerScan`] and decides nothing: `alz-core`'s `scan_verdict` applies the order of checks.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;

use alz_core::git::{BlobId, ObjectFormat};
use alz_core::layer::{GitFileFacts, LayerScan, MAX_FILE_BYTES, NameProblem, Purpose};
use alz_core::lock::{FileDigest, is_env_file, is_skipped_name};
use alz_core::path::{SafeRelPath, SafeRelPathError};
use rustix::fs::{self, AtFlags, DirEntry, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::fs::read_below::{read_at_most, require_regular};
use crate::fs::read_error::ReadError;

/// What a name is, for the walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Directory,
    RegularFile,
    /// A link, a pipe, a socket, a device or anything else: counted, never opened.
    Other,
}

impl Kind {
    fn of(file_type: FileType) -> Option<Self> {
        match file_type {
            FileType::Directory => Some(Self::Directory),
            FileType::RegularFile => Some(Self::RegularFile),
            FileType::Symlink
            | FileType::Fifo
            | FileType::Socket
            | FileType::CharacterDevice
            | FileType::BlockDevice => Some(Self::Other),
            FileType::Unknown => None,
        }
    }
}

/// Where in the layer a folder lies, as far as its path can be written down.
#[derive(Debug, Clone)]
enum Place {
    /// The layer directory itself.
    Root,
    /// A folder whose path is a safe relative path.
    Named(SafeRelPath),
    /// A folder whose own name or an ancestor's is a problem, so nothing below it has a path.
    Unnamed,
}

impl Place {
    /// The path of `name` inside this place: `None` when no path can be written, and `Err` when
    /// the path would not be a safe relative path.
    fn path_of(&self, name: &str) -> Option<Result<SafeRelPath, SafeRelPathError>> {
        match self {
            Self::Root => Some(SafeRelPath::try_from(name)),
            Self::Named(parent) => Some(SafeRelPath::try_from(format!("{parent}/{name}").as_str())),
            Self::Unnamed => None,
        }
    }
}

/// A folder being listed: the handle its entries are opened relative to, and the listing, which
/// is read one entry at a time so that the walk never reads past the entry limit.
struct Frame {
    handle: OwnedFd,
    entries: fs::Dir,
    place: Place,
}

impl Frame {
    fn new(handle: OwnedFd, place: Place) -> io::Result<Self> {
        fs::Dir::read_from(&handle)
            .map_err(io::Error::from)
            .map(|entries| Self {
                handle,
                entries,
                place,
            })
    }
}

/// Walks the layer below `root`. Failures are facts of the scan, never errors.
///
/// With a `format`, the blob id of each file's bytes and its executable bit are recorded too.
pub(super) fn scan_layer(
    root: &OwnedFd,
    purpose: Purpose,
    format: Option<ObjectFormat>,
) -> LayerScan {
    let first = root
        .try_clone()
        .and_then(|handle| Frame::new(handle, Place::Root));
    match first {
        Ok(frame) => walk(vec![frame], purpose, format, LayerScan::new()),
        Err(_) => LayerScan::new().with_unreadable(),
    }
}

/// The walk, as a loop over an explicit stack of folders. It performs the system calls, so it
/// is the shell around the pure facts; the stack keeps its depth off the call stack.
fn walk(
    mut stack: Vec<Frame>,
    purpose: Purpose,
    format: Option<ObjectFormat>,
    mut scan: LayerScan,
) -> LayerScan {
    loop {
        let Some(frame) = stack.last_mut() else {
            return scan;
        };
        match frame.entries.next() {
            None => {
                stack.pop();
            }
            Some(Err(_)) => {
                scan = scan.with_unreadable();
                stack.pop();
            }
            Some(Ok(entry)) => {
                let (seen, below) = visit(frame, &entry, purpose, format, scan);
                scan = seen;
                if scan.is_past_entry_limit(purpose) {
                    return scan;
                }
                stack.extend(below);
            }
        }
    }
}

/// Handles one entry of `frame`: counts it, checks its name, and reads it, records it or opens
/// it as a folder to enter.
fn visit(
    frame: &Frame,
    entry: &DirEntry,
    purpose: Purpose,
    format: Option<ObjectFormat>,
    scan: LayerScan,
) -> (LayerScan, Option<Frame>) {
    let raw = entry.file_name();
    if matches!(raw.to_bytes(), b"." | b"..") {
        return (scan, None);
    }
    match OsStr::from_bytes(raw.to_bytes()).to_str() {
        Some(name) if is_skipped_name(name) => (scan, None),
        Some(name) => {
            let path = frame.place.path_of(name);
            let scan = match path {
                Some(Err(_)) => scan.with_name_problem(NameProblem::NotSafe),
                Some(Ok(_)) | None => scan,
            };
            visit_named(
                frame,
                entry,
                path.and_then(Result::ok),
                purpose,
                format,
                scan.listed_entry(),
            )
        }
        None => visit_named(
            frame,
            entry,
            None,
            purpose,
            format,
            scan.with_name_problem(NameProblem::NotUtf8).listed_entry(),
        ),
    }
}

/// What the entry is, and what to do with it. `path` is `None` when the entry has no safe path,
/// which is when it lies below a name that is a problem or is one itself.
fn visit_named(
    frame: &Frame,
    entry: &DirEntry,
    path: Option<SafeRelPath>,
    purpose: Purpose,
    format: Option<ObjectFormat>,
    scan: LayerScan,
) -> (LayerScan, Option<Frame>) {
    match kind_of(frame, entry) {
        Err(_) => (scan.with_unreadable(), None),
        Ok(Kind::Other) => (scan.with_other_entry(), None),
        Ok(Kind::Directory) => enter(frame, entry, path, scan),
        Ok(Kind::RegularFile) => (
            read_or_record(frame, entry, path, purpose, format, scan),
            None,
        ),
    }
}

fn kind_of(frame: &Frame, entry: &DirEntry) -> io::Result<Kind> {
    match Kind::of(entry.file_type()) {
        Some(kind) => Ok(kind),
        // The file system does not say in the listing: ask, without following a link.
        None => fs::statat(&frame.handle, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
            .map(|status| Kind::of(FileType::from_raw_mode(status.st_mode)).unwrap_or(Kind::Other))
            .map_err(io::Error::from),
    }
}

/// Opens the folder `entry` relative to the handle of its parent. The folder below a name that is
/// a problem is entered too (§5.3 "File names": the walk lists every entry), without a path.
fn enter(
    frame: &Frame,
    entry: &DirEntry,
    path: Option<SafeRelPath>,
    scan: LayerScan,
) -> (LayerScan, Option<Frame>) {
    let place = path.map_or(Place::Unnamed, Place::Named);
    let opened = fs::openat(
        &frame.handle,
        entry.file_name(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)
    .and_then(|handle| Frame::new(handle, place));
    match opened {
        Ok(below) => (scan, Some(below)),
        Err(_) => (scan.with_unreadable(), None),
    }
}

/// A regular file: a `.env` file is recorded and never opened; any other is read, unless it has
/// no path to be hashed under or the byte budget is already used up.
fn read_or_record(
    frame: &Frame,
    entry: &DirEntry,
    path: Option<SafeRelPath>,
    purpose: Purpose,
    format: Option<ObjectFormat>,
    scan: LayerScan,
) -> LayerScan {
    match path {
        None => scan,
        Some(path) if is_env_file(&path) => scan.with_env_file(path),
        Some(_) if scan.is_past_byte_limit(purpose) => scan,
        Some(path) => {
            let remaining = remaining_bytes(purpose, &scan);
            match read_file(frame, entry, remaining) {
                Ok((bytes, executable)) => {
                    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                    let scan = match format {
                        Some(format) => scan.with_git_facts(GitFileFacts {
                            path: path.clone(),
                            executable,
                            blob: BlobId::of(format, &bytes).id(),
                        }),
                        None => scan,
                    };
                    scan.with_file(path, FileDigest::of(&bytes), length)
                }
                // In resolution one byte past what is left marks the budget as used up; in the build
                // there is no budget, so a file past the cap of one file is itself unreadable.
                Err(ReadError::TooLarge) => match purpose {
                    Purpose::Resolution => scan.with_bytes(
                        u64::try_from(remaining)
                            .unwrap_or(u64::MAX)
                            .saturating_add(1),
                    ),
                    Purpose::Build => scan.with_unreadable(),
                },
                Err(ReadError::Missing | ReadError::NotRegular | ReadError::Io(_)) => {
                    scan.with_unreadable()
                }
            }
        }
    }
}

/// How many bytes the next file may have: the cap of one file, or what is left of the budget in
/// all when that is less.
fn remaining_bytes(purpose: Purpose, scan: &LayerScan) -> usize {
    let left = purpose.limits().map_or(MAX_FILE_BYTES, |limits| {
        limits.bytes().saturating_sub(scan.bytes())
    });
    usize::try_from(left.min(MAX_FILE_BYTES)).unwrap_or(usize::MAX)
}

/// Opens the file relative to the handle of its folder, with `O_NONBLOCK` so that a pipe swapped
/// in after the listing cannot block the open, checks with `fstat` on that handle that it is a
/// regular file, and reads at most `cap + 1` bytes. It gives the bytes and the owner-execute bit.
fn read_file(frame: &Frame, entry: &DirEntry, cap: usize) -> Result<(Vec<u8>, bool), ReadError> {
    let file = fs::openat(
        &frame.handle,
        entry.file_name(),
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| match errno {
        Errno::NOENT => ReadError::Missing,
        Errno::LOOP | Errno::NOTDIR => ReadError::NotRegular,
        other => ReadError::Io(other.into()),
    })?;
    let mode = require_regular(&file)?;
    let executable = mode.contains(Mode::XUSR);
    read_at_most(File::from(file), cap).map(|bytes| (bytes, executable))
}
