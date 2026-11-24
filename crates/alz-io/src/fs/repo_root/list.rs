//! Listing folders below the repository root without following a link.

use std::os::fd::OwnedFd;

use alz_core::Stripped;
use alz_core::path::SafeRelPath;
use rustix::fs::{self, AtFlags, Dir, FileType};

use crate::fs::repo_root::error::RepoError;
use crate::fs::repo_root::probe::{Below, folder_at};

/// The most entries the writer looks at in one folder tree.
const MAX_ENTRIES: usize = 10_000;

/// One entry of a folder. `name` is `None` when the file system's name for it is not valid
/// Unicode.
struct Entry {
    name: Option<String>,
    is_folder: bool,
}

/// The entries of `handle`, which is `folder` (the repository root when `None`). Each is looked at
/// with `lstat`, so a link is no folder. A name that is not valid Unicode is kept as `None`.
fn listing(handle: &OwnedFd, folder: Option<&SafeRelPath>) -> Result<Vec<Entry>, RepoError> {
    let label = folder_label(folder);
    let failed = |errno: rustix::io::Errno| RepoError::Io {
        operation: "listing",
        path: label.clone(),
        source: errno.into(),
    };
    let mut found = Vec::new();
    for entry in Dir::read_from(handle).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        if matches!(entry.file_name().to_bytes(), b"." | b"..") {
            continue;
        }
        if found.len() >= MAX_ENTRIES {
            return Err(RepoError::TooManyEntries {
                folder: label,
                limit: MAX_ENTRIES,
            });
        }
        let status =
            fs::statat(handle, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW).map_err(failed)?;
        found.push(Entry {
            name: entry.file_name().to_str().ok().map(str::to_owned),
            is_folder: FileType::from_raw_mode(status.st_mode) == FileType::Directory,
        });
    }
    Ok(found)
}

fn folder_label(folder: Option<&SafeRelPath>) -> String {
    folder.map_or_else(
        || String::from("the repository root"),
        |folder| Stripped::from_untrusted(folder.as_str()).to_string(),
    )
}

/// The entries of `handle` when every name is valid Unicode, else the error for the folder.
fn entries(
    handle: &OwnedFd,
    folder: Option<&SafeRelPath>,
) -> Result<Vec<(String, bool)>, RepoError> {
    listing(handle, folder)?
        .into_iter()
        .map(|entry| {
            entry
                .name
                .map(|name| (name, entry.is_folder))
                .ok_or_else(|| RepoError::UnnameableEntry {
                    folder: folder_label(folder),
                })
        })
        .collect()
}

fn join(folder: Option<&SafeRelPath>, name: &str) -> Option<SafeRelPath> {
    let text = folder.map_or_else(
        || name.to_owned(),
        |folder| format!("{}/{name}", folder.as_str()),
    );
    SafeRelPath::try_from(text.as_str()).ok()
}

/// The paths of the entries directly in `folder` (the root when `None`). A folder that is absent,
/// a link or not a folder lists nothing, and an entry whose name is no safe relative path is left
/// out: it can be the same file as no path of a build.
pub(super) fn names_in(
    root: &OwnedFd,
    folder: Option<&SafeRelPath>,
) -> Result<Vec<SafeRelPath>, RepoError> {
    let listed = match folder {
        None => entries(root, None),
        Some(folder) => match folder_at(root, folder.segments())
            .map_err(|source| RepoError::io("listing", folder, source))?
        {
            Below::Folder(handle) => entries(&handle, Some(folder)),
            Below::Stop(_) => return Ok(Vec::new()),
        },
    };
    // A name that is not valid Unicode is not an error here: it is no spelling of a safe path.
    Ok(match listed {
        Ok(found) => found
            .into_iter()
            .filter_map(|(name, _)| join(folder, &name))
            .collect(),
        Err(RepoError::UnnameableEntry { .. }) => Vec::new(),
        Err(other) => return Err(other),
    })
}

/// Every file below `folder`, folders entered from an explicit stack and no link followed: a link
/// is listed as an entry like a file and never entered. A name that is not a safe relative path
/// is an error, since it can be compared with nothing.
pub(super) fn files_below(
    root: &OwnedFd,
    folder: &SafeRelPath,
) -> Result<Vec<SafeRelPath>, RepoError> {
    let mut pending = vec![folder.clone()];
    let mut found = Vec::new();
    while let Some(next) = pending.pop() {
        let Below::Folder(handle) = folder_at(root, next.segments())
            .map_err(|source| RepoError::io("listing", &next, source))?
        else {
            continue;
        };
        for (name, is_folder) in entries(&handle, Some(&next))? {
            let path = join(Some(&next), &name).ok_or_else(|| RepoError::UnnameableEntry {
                folder: Stripped::from_untrusted(next.as_str()).to_string(),
            })?;
            if is_folder {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
        if found.len().saturating_add(pending.len()) > MAX_ENTRIES {
            return Err(RepoError::TooManyEntries {
                folder: Stripped::from_untrusted(folder.as_str()).to_string(),
                limit: MAX_ENTRIES,
            });
        }
    }
    found.sort();
    Ok(found)
}

/// Folders the walk of the whole repository never enters: version control and what tools install.
const NOT_WALKED: [&str; 3] = [".git", ".DS_Store", "node_modules"];

/// Every file named `name` anywhere in the repository, in path order. Folders are entered from an
/// explicit stack and no link is followed; a link is no file. An entry whose name is no valid
/// Unicode or no safe relative path cannot be named, so it is skipped when it is no folder (it is
/// not the file looked for), and refused as [`RepoError::UnnameableEntry`] when it is a folder:
/// what lies below it cannot be searched, and passing it over would hide a file.
pub(super) fn files_named(root: &OwnedFd, name: &str) -> Result<Vec<SafeRelPath>, RepoError> {
    let mut pending: Vec<Option<SafeRelPath>> = vec![None];
    let mut found = Vec::new();
    while let Some(next) = pending.pop() {
        let listed = match &next {
            None => listing(root, None)?,
            Some(folder) => match folder_at(root, folder.segments())
                .map_err(|source| RepoError::io("listing", folder, source))?
            {
                Below::Folder(handle) => listing(&handle, Some(folder))?,
                Below::Stop(_) => continue,
            },
        };
        for entry in listed {
            // Folders the walk never enters are passed over before their name is parsed: `.git`
            // is no safe relative path.
            if entry.is_folder
                && entry
                    .name
                    .as_deref()
                    .is_some_and(|n| NOT_WALKED.contains(&n))
            {
                continue;
            }
            let path = entry
                .name
                .as_deref()
                .and_then(|entry_name| join(next.as_ref(), entry_name));
            match (entry.name.as_deref(), path) {
                (Some(_), Some(path)) if entry.is_folder => pending.push(Some(path)),
                (Some(entry_name), Some(path)) if entry_name == name => found.push(path),
                (_, None) if entry.is_folder => {
                    return Err(RepoError::UnnameableEntry {
                        folder: folder_label(next.as_ref()),
                    });
                }
                _ => {}
            }
        }
    }
    found.sort();
    Ok(found)
}
