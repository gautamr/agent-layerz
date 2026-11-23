//! Writing a new regular file below a directory handle, never through a link (decision-27). Only
//! the temporary cache entry reaches it.

use std::io::{self, Write};
use std::os::fd::OwnedFd;

use alz_core::path::SafeRelPath;
use rustix::fs::{self, Mode, OFlags};
use rustix::io::Errno;

/// Creates the file `path` below `root` with `bytes` and the mode `0644`, or `0755` when
/// `executable`.
///
/// Each missing folder is made with `mkdirat` and mode `0700` and opened with `O_NOFOLLOW`, so a
/// link is never followed. The file is created with `O_EXCL | O_NOFOLLOW`, written, set to its
/// mode with `fchmod` (so the umask changes nothing) and synced.
pub(super) fn write_file_below(
    root: &OwnedFd,
    path: &SafeRelPath,
    bytes: &[u8],
    executable: bool,
) -> io::Result<()> {
    let segments: Vec<&str> = path.segments().collect();
    let (name, parents) = segments
        .split_last()
        .ok_or_else(|| io::Error::other("an empty path"))?;
    let directory = parents
        .iter()
        .try_fold(root.try_clone()?, |directory, segment| {
            match fs::mkdirat(&directory, *segment, Mode::RWXU) {
                Ok(()) | Err(Errno::EXIST) => {}
                Err(errno) => return Err(io::Error::from(errno)),
            }
            fs::openat(
                &directory,
                *segment,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io::Error::from)
        })?;
    let file = fs::openat(
        &directory,
        *name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    fs::fchmod(
        &file,
        if executable {
            Mode::from_raw_mode(0o755)
        } else {
            Mode::from_raw_mode(0o644)
        },
    )?;
    let mut file = std::fs::File::from(file);
    file.write_all(bytes)?;
    file.sync_all()
}
