//! Looking at `.git` and `.git/config` of the repository root without following a link (spec
//! §9.4 "Writing"). git follows a link at the file it writes (`docs/target-facts.md` §11), so
//! these probes run right before and again right after each write.

use std::os::fd::OwnedFd;

use alz_core::identity::{Blocker, GitConfigPlace};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;

/// What was true of `.git` and `.git/config` when they were checked before a write: `.git` is a
/// real directory, of this device and inode, and `.git/config` is absent or a regular file. It
/// exists only for a [`GitConfigPlace::Writable`] place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitConfigSeal {
    git: FileId,
}

/// What a check after a write found changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changed {
    /// `.git` is no longer the directory that was checked: it is gone, a link, or another folder.
    Git,
    /// `.git/config` is a link, a folder or anything else but a regular file, or could not be
    /// looked at.
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileId {
    device: u64,
    inode: u64,
}

/// `.git` opened, and what is in it.
struct Inspected {
    git: FileId,
}

/// Looks at `.git` of `root`, then at `config` inside the handle that was opened.
fn inspect(root: &OwnedFd) -> Result<Inspected, Blocker> {
    let git = match fs::openat(
        root,
        ".git",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(handle) => handle,
        Err(Errno::NOENT) => return Err(Blocker::NotTopLevel),
        Err(Errno::LOOP | Errno::NOTDIR) => return Err(Blocker::GitNotADirectory),
        Err(_) => return Err(Blocker::Uninspectable),
    };
    let status = fs::fstat(&git).or(Err(Blocker::Uninspectable))?;
    config_is_plain(&git)?;
    Ok(Inspected {
        git: FileId {
            device: u64::try_from(status.st_dev).unwrap_or_default(),
            inode: status.st_ino,
        },
    })
}

/// `config` in the `.git` folder `git` is absent or a regular file.
fn config_is_plain(git: &OwnedFd) -> Result<(), Blocker> {
    match fs::statat(git, "config", AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => Ok(()),
        Err(_) => Err(Blocker::Uninspectable),
        Ok(status) => match FileType::from_raw_mode(status.st_mode) {
            FileType::RegularFile => Ok(()),
            FileType::Symlink
            | FileType::Directory
            | FileType::Fifo
            | FileType::Socket
            | FileType::CharacterDevice
            | FileType::BlockDevice
            | FileType::Unknown => Err(Blocker::ConfigNotRegular),
        },
    }
}

/// The place for `root`'s `.git/config`.
pub(super) fn place(root: &OwnedFd) -> GitConfigPlace {
    match inspect(root) {
        Ok(_) => GitConfigPlace::Writable,
        Err(blocker) => GitConfigPlace::Blocked(blocker),
    }
}

/// Checks the place before a write and remembers `.git`.
pub(super) fn seal(root: &OwnedFd) -> Result<GitConfigSeal, Blocker> {
    inspect(root).map(|inspected| GitConfigSeal { git: inspected.git })
}

/// Checks the place again after a write.
pub(super) fn verify(root: &OwnedFd, seal: GitConfigSeal) -> Result<(), Changed> {
    match inspect(root) {
        Ok(now) if now.git == seal.git => Ok(()),
        Ok(_) | Err(Blocker::NotTopLevel | Blocker::GitNotADirectory) => Err(Changed::Git),
        Err(Blocker::ConfigNotRegular | Blocker::ConfigOutside | Blocker::Uninspectable) => {
            Err(Changed::Config)
        }
    }
}
