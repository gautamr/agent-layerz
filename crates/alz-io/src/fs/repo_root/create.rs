//! Creating a file that does not exist yet (spec §4.3): written as a temporary file in its folder
//! and linked to its name, which fails when anything, a link included, is already there.

use std::io;
use std::os::fd::OwnedFd;

use alz_core::path::SafeRelPath;
use rustix::fs::{self, AtFlags};
use rustix::io::Errno;

use crate::fs::repo_root::error::RepoError;
use crate::fs::repo_root::write::{folder_for, temporary_name, write_temporary};

/// Creates `path` below `root` with `bytes`, mode `0644`, and never replaces or follows anything
/// at it. A missing folder above it is made, each step opened with `O_NOFOLLOW`. The content is
/// complete under the real name or the name is not there at all.
pub(super) fn create_new(
    root: &OwnedFd,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Result<(), RepoError> {
    create_file(root, path, bytes, false)
}

/// [`create_new`] with the executable bit of the new file chosen: `0755` or `0644` whatever the
/// umask.
pub(super) fn create_file(
    root: &OwnedFd,
    path: &SafeRelPath,
    bytes: &[u8],
    executable: bool,
) -> Result<(), RepoError> {
    let segments: Vec<&str> = path.segments().collect();
    let (name, parents) = segments
        .split_last()
        .ok_or_else(|| RepoError::changed(path))?;
    let directory = folder_for(root, parents, path)?;
    let temporary = temporary_name();
    let linked = write_temporary(&directory, &temporary, bytes, executable)
        .map_err(Failure::Other)
        .and_then(|()| {
            fs::linkat(
                &directory,
                temporary.as_str(),
                &directory,
                *name,
                AtFlags::empty(),
            )
            .map_err(|errno| match errno {
                Errno::EXIST => Failure::Exists,
                other => Failure::Other(io::Error::from(other)),
            })
        });
    // The temporary file is removed on a best-effort basis: the error that is reported is the
    // first one.
    match fs::unlinkat(&directory, temporary.as_str(), AtFlags::empty()) {
        Ok(()) | Err(_) => {}
    }
    linked.map_err(|failure| match failure {
        Failure::Exists => RepoError::exists(path),
        Failure::Other(source) => RepoError::io("writing", path, source),
    })
}

/// Why the file was not created.
enum Failure {
    Exists,
    Other(io::Error),
}
