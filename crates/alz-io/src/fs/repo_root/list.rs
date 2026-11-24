//! Listing folders below the repository root without following a link.

use std::ffi::{CStr, CString};
use std::os::fd::OwnedFd;
use std::rc::Rc;

use alz_core::Stripped;
use alz_core::path::SafeRelPath;
use rustix::fs::{self, AtFlags, Dir, FileType};

use crate::fs::repo_root::error::RepoError;
use crate::fs::repo_root::probe::{Below, folder_at, step_into};

/// One entry of a folder: its name as the file system holds it, and whether it is a folder. A link
/// is no folder.
struct Entry {
    name: CString,
    is_folder: bool,
}

/// Calls `each` with every entry of `handle`, which is `label`, without collecting them. A folder
/// is told by the type the directory gives, and by `lstat` where it gives none, so a link is never
/// a folder. Any number of entries is accepted.
fn for_each_entry(
    handle: &OwnedFd,
    label: &dyn Fn() -> String,
    mut each: impl FnMut(&CStr, bool),
) -> Result<(), RepoError> {
    let failed = |errno: rustix::io::Errno| RepoError::Io {
        operation: "listing",
        path: label(),
        source: errno.into(),
    };
    for entry in Dir::read_from(handle).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        if matches!(entry.file_name().to_bytes(), b"." | b"..") {
            continue;
        }
        let kind = entry.file_type();
        let is_folder = if kind == FileType::Unknown {
            let status =
                fs::statat(handle, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW).map_err(failed)?;
            FileType::from_raw_mode(status.st_mode) == FileType::Directory
        } else {
            kind == FileType::Directory
        };
        each(entry.file_name(), is_folder);
    }
    Ok(())
}

/// The entries of `handle`, which is `folder` (the repository root when `None`).
fn listing(handle: &OwnedFd, folder: Option<&SafeRelPath>) -> Result<Vec<Entry>, RepoError> {
    let mut found = Vec::new();
    for_each_entry(handle, &|| folder_label(folder), |name, is_folder| {
        found.push(Entry {
            name: name.to_owned(),
            is_folder,
        });
    })?;
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
                .to_str()
                .ok()
                .map(|name| (name.to_owned(), entry.is_folder))
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
    }
    found.sort();
    Ok(found)
}

/// Folders the walk of the whole repository never enters: version control and what tools install.
/// `.git` is compared byte for byte: on a file system that folds case the directory gives the name
/// it holds, and on one that does not, `.GIT` is a folder like any other.
const NOT_WALKED: [&[u8]; 3] = [b".git", b".DS_Store", b"node_modules"];

/// A folder the walk has still to enter: where it lies, and the open parent it is entered from.
/// The parent stays open only while a sibling is pending, so a chain of folders holds one handle.
struct Pending {
    parent: Rc<OwnedFd>,
    name: CString,
    path: Vec<u8>,
}

/// `parent` and `name` joined with `/`; `parent` is empty for the repository root.
fn joined(parent: &[u8], name: &CStr) -> Vec<u8> {
    let separator: &[u8] = if parent.is_empty() { b"" } else { b"/" };
    [parent, separator, name.to_bytes()].concat()
}

/// The path as a person reads it: control characters removed, a name that is no Unicode shown
/// with replacement characters.
fn shown(path: &[u8]) -> Stripped {
    Stripped::from_untrusted(&String::from_utf8_lossy(path))
}

fn walked_label(path: &[u8]) -> String {
    if path.is_empty() {
        String::from("the repository root")
    } else {
        shown(path).to_string()
    }
}

/// Visits every entry of the repository once, each folder listed once, the folders below the root
/// entered one from the handle of its parent (never from the root again), with no limit on the
/// entries of a folder or the depth, and no link followed. `visit` gets the path of the folder
/// (empty for the root), the entry's name as bytes and whether it is a folder. A folder in
/// [`NOT_WALKED`] is visited as an entry and not entered. A folder that is gone, or has become a
/// link, since it was listed is passed over.
fn walk(root: &OwnedFd, mut visit: impl FnMut(&[u8], &CStr, bool)) -> Result<(), RepoError> {
    let mut pending: Vec<Pending> = Vec::new();
    let mut list = |handle: Rc<OwnedFd>, path: Vec<u8>, pending: &mut Vec<Pending>| {
        for_each_entry(&handle, &|| walked_label(&path), |name, is_folder| {
            visit(&path, name, is_folder);
            if is_folder && !NOT_WALKED.contains(&name.to_bytes()) {
                pending.push(Pending {
                    parent: Rc::clone(&handle),
                    name: name.to_owned(),
                    path: joined(&path, name),
                });
            }
        })
    };
    let first = root.try_clone().map_err(|source| RepoError::Io {
        operation: "listing",
        path: walked_label(b""),
        source,
    })?;
    list(Rc::new(first), Vec::new(), &mut pending)?;
    while let Some(Pending { parent, name, path }) = pending.pop() {
        let opened = step_into(&parent, name.as_c_str()).map_err(|source| RepoError::Io {
            operation: "listing",
            path: walked_label(&path),
            source,
        })?;
        drop(parent);
        if let Below::Folder(handle) = opened {
            list(Rc::new(handle), path, &mut pending)?;
        }
    }
    Ok(())
}

/// Every file named `name` anywhere in the repository, in path order, as it is shown (see
/// [`shown`]). Each folder is listed once and no link is followed; an entry that is no folder
/// counts, a link included, since a link named so is read by the tool. A folder whose name is not
/// valid Unicode or not a safe relative path is entered like any other: the walk goes by the
/// names the file system holds, and only what it finds is shown.
pub(super) fn files_named(root: &OwnedFd, name: &str) -> Result<Vec<Stripped>, RepoError> {
    let mut found: Vec<Vec<u8>> = Vec::new();
    walk(root, |folder, entry, is_folder| {
        if !is_folder && entry.to_bytes() == name.as_bytes() {
            found.push(joined(folder, entry));
        }
    })?;
    found.sort();
    Ok(found.iter().map(|path| shown(path)).collect())
}

#[cfg(test)]
mod tests;
