//! The writes of the repository root: a file as a temporary file in its folder renamed into place,
//! a delete, and the removal of folders left empty. Nothing follows a link.

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicU64, Ordering};

use alz_core::path::SafeRelPath;
use alz_core::render::tree::FileMode;
use alz_core::render::write_plan::{Change, ClearPlan, Observed, gitignore_path, lock_path};
use rustix::fs::{self, AtFlags, Mode, OFlags};
use rustix::io::Errno;

use crate::fs::repo_root::create::create_file;
use crate::fs::repo_root::error::RepoError;
use crate::fs::repo_root::probe::{Below, folder_at, probe, step_into};

/// Counts the temporary files this process has made, so two never share a name.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// A name for a temporary file that no other file of this process has.
pub(super) fn temporary_name() -> String {
    format!(
        ".alz-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    )
}

/// What an apply did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    /// Files created.
    pub created: usize,
    /// Files updated.
    pub updated: usize,
    /// Files deleted.
    pub deleted: usize,
    /// Whether `.gitignore` was written.
    pub gitignore_written: bool,
    /// Whether `layer.lock` was written.
    pub lock_written: bool,
}

/// Applies `plan` below `root`: each create and update, then each delete, the folders the deletes
/// left empty, `.gitignore`, and `layer.lock` last. It stops at the first failure, so a lock is
/// written only after every file before it was.
pub(super) fn apply(root: &OwnedFd, plan: &ClearPlan) -> Result<Applied, RepoError> {
    let applied =
        plan.changes()
            .iter()
            .try_fold(Applied::default(), |applied, change| match change {
                Change::Create { path, file } => {
                    create_file(
                        root,
                        path.as_safe(),
                        &file.bytes,
                        file.mode == FileMode::Executable,
                    )
                    .map_err(|error| {
                        // The plan saw nothing here: something appeared since.
                        if matches!(error, RepoError::Exists { .. }) {
                            RepoError::changed(path.as_safe())
                        } else {
                            error
                        }
                    })?;
                    Ok(Applied {
                        created: applied.created.saturating_add(1),
                        ..applied
                    })
                }
                Change::Update {
                    path,
                    file,
                    observed,
                } => {
                    require_unchanged(root, path.as_safe(), observed, Expect::Present)?;
                    write_file(
                        root,
                        path.as_safe(),
                        &file.bytes,
                        file.mode == FileMode::Executable,
                    )?;
                    Ok(Applied {
                        updated: applied.updated.saturating_add(1),
                        ..applied
                    })
                }
                Change::Delete { path, observed } => {
                    require_unchanged(root, path, observed, Expect::PresentOrGone)?;
                    delete_file(root, path)?;
                    Ok(Applied {
                        deleted: applied.deleted.saturating_add(1),
                        ..applied
                    })
                }
            })?;
    prune(
        root,
        plan.changes().iter().filter_map(|change| match change {
            Change::Delete { path, .. } => Some(path),
            Change::Create { .. } | Change::Update { .. } => None,
        }),
    )?;
    let gitignore_written = plan
        .gitignore()
        .map(|bytes| write_file(root, gitignore_path(), bytes, false))
        .transpose()?
        .is_some();
    let lock_written = plan
        .lock()
        .map(|text| write_file(root, lock_path(), text.as_bytes(), false))
        .transpose()?
        .is_some();
    Ok(Applied {
        gitignore_written,
        lock_written,
        ..applied
    })
}

/// What a re-probe of a path may find besides the bytes the plan observed.
#[derive(Clone, Copy)]
enum Expect {
    /// The file must still be there with those bytes.
    Present,
    /// The file must hold those bytes or be gone already.
    PresentOrGone,
}

/// Looks at `path` again before it is changed and requires the bytes the plan observed: a file
/// that was edited, replaced by a link or another entry, or (for an update) removed since the
/// plan is [`RepoError::Changed`], and nothing more is written.
fn require_unchanged(
    root: &OwnedFd,
    path: &SafeRelPath,
    observed: &[u8],
    expect: Expect,
) -> Result<(), RepoError> {
    match (probe(root, path)?, expect) {
        (Observed::Regular { bytes, .. }, _) if bytes == observed => Ok(()),
        (Observed::Absent, Expect::PresentOrGone) => Ok(()),
        (
            Observed::Absent
            | Observed::Regular { .. }
            | Observed::Link
            | Observed::NotRegular
            | Observed::TooLarge,
            _,
        ) => Err(RepoError::changed(path)),
    }
}

/// The folder of `path` below `root`, made when it is missing, each step opened with
/// `O_NOFOLLOW`. A step that is a link or not a folder means the repository changed since the
/// plan.
pub(super) fn folder_for(
    root: &OwnedFd,
    parents: &[&str],
    path: &SafeRelPath,
) -> Result<OwnedFd, RepoError> {
    parents.iter().try_fold(
        root.try_clone()
            .map_err(|e| RepoError::io("writing", path, e))?,
        |directory, segment| {
            match fs::mkdirat(&directory, *segment, Mode::from_raw_mode(0o777)) {
                Ok(()) | Err(Errno::EXIST) => {}
                Err(errno) => return Err(RepoError::io("writing", path, errno.into())),
            }
            match step_into(&directory, *segment).map_err(|e| RepoError::io("writing", path, e))? {
                Below::Folder(handle) => Ok(handle),
                Below::Stop(_) => Err(RepoError::changed(path)),
            }
        },
    )
}

/// Writes `bytes` to `path` as a temporary file in its folder, set to `0644` or `0755` whatever
/// the umask, synced, and renamed over the path. The temporary file is removed when anything
/// fails, so no partial file is left under the real name.
pub(super) fn write_file(
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
    let failed = |source: io::Error| RepoError::io("writing", path, source);
    let written = write_temporary(&directory, &temporary, bytes, executable).and_then(|()| {
        fs::renameat(&directory, temporary.as_str(), &directory, *name).map_err(io::Error::from)
    });
    match written {
        Ok(()) => Ok(()),
        Err(source) => {
            // The temporary file is removed on a best-effort basis: the error that is reported is
            // the first one.
            match fs::unlinkat(&directory, temporary.as_str(), AtFlags::empty()) {
                Ok(()) | Err(_) => {}
            }
            Err(failed(source))
        }
    }
}

pub(super) fn write_temporary(
    directory: &OwnedFd,
    temporary: &str,
    bytes: &[u8],
    executable: bool,
) -> io::Result<()> {
    let handle = fs::openat(
        directory,
        temporary,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    fs::fchmod(
        &handle,
        Mode::from_raw_mode(if executable { 0o755 } else { 0o644 }),
    )?;
    let mut file = std::fs::File::from(handle);
    file.write_all(bytes)?;
    file.sync_all()
}

/// Deletes the file `path`. A missing folder or file is nothing to delete; a link or other
/// entry above it means the repository changed since the plan, and nothing is removed through it.
pub(super) fn delete_file(root: &OwnedFd, path: &SafeRelPath) -> Result<(), RepoError> {
    let segments: Vec<&str> = path.segments().collect();
    let Some((name, parents)) = segments.split_last() else {
        return Ok(());
    };
    let failed = |source: io::Error| RepoError::io("removing", path, source);
    match folder_at(root, parents.iter().copied()).map_err(failed)? {
        Below::Stop(_) => Ok(()),
        Below::Folder(directory) => match fs::unlinkat(&directory, *name, AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => Ok(()),
            Err(errno) => Err(failed(errno.into())),
        },
    }
}

/// Removes the folders above the deleted paths that are empty, deepest first, up to the root,
/// which stays. `rmdir` removes an empty folder only, and a link is not one.
pub(super) fn prune<'a>(
    root: &OwnedFd,
    deleted: impl Iterator<Item = &'a SafeRelPath>,
) -> Result<(), RepoError> {
    let folders: BTreeSet<(std::cmp::Reverse<usize>, String)> = deleted
        .flat_map(|path| {
            let segments: Vec<&str> = path.segments().collect();
            (1..segments.len())
                .filter_map(|depth| segments.get(..depth).map(|prefix| prefix.join("/")))
                .collect::<Vec<_>>()
        })
        .map(|folder| (std::cmp::Reverse(folder.split('/').count()), folder))
        .collect();
    folders.into_iter().try_for_each(|(_, folder)| {
        let Ok(path) = SafeRelPath::try_from(folder.as_str()) else {
            return Ok(());
        };
        let segments: Vec<&str> = path.segments().collect();
        let Some((name, parents)) = segments.split_last() else {
            return Ok(());
        };
        let failed = |source: io::Error| RepoError::io("removing", &path, source);
        match folder_at(root, parents.iter().copied()).map_err(failed)? {
            Below::Stop(_) => Ok(()),
            Below::Folder(directory) => match fs::unlinkat(&directory, *name, AtFlags::REMOVEDIR) {
                Ok(()) | Err(Errno::NOENT | Errno::NOTEMPTY | Errno::EXIST | Errno::NOTDIR) => {
                    Ok(())
                }
                Err(errno) => Err(failed(errno.into())),
            },
        }
    })
}
