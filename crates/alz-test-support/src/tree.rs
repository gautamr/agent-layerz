//! Whole folders as values: copying one, and reading every file of one with its executable bit.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// A regular file of a folder: its bytes and whether the owner-execute bit is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeFile {
    /// The content.
    pub bytes: Vec<u8>,
    /// The owner-execute bit.
    pub executable: bool,
}

/// Copies the folder `from` to `to`, which is made: every file with its mode, every folder, no
/// link followed or made.
///
/// # Errors
///
/// The first error of reading or writing.
pub fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    fs::read_dir(from)?.try_for_each(|entry| {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).map(|_bytes| ())
        } else {
            Ok(())
        }
    })
}

/// Every regular file below `root`, by its `/`-separated path from `root`, `.git` folders left
/// out. Links and other entries are not listed.
///
/// # Errors
///
/// The first error of listing or reading.
pub fn read_tree(root: &Path) -> io::Result<BTreeMap<String, TreeFile>> {
    let mut found = BTreeMap::new();
    collect(root, "", &mut found)?;
    Ok(found)
}

fn collect(folder: &Path, prefix: &str, found: &mut BTreeMap<String, TreeFile>) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&entry.path(), &path, found)?;
        } else if kind.is_file() {
            let mode = entry.metadata()?.permissions().mode();
            found.insert(
                path,
                TreeFile {
                    bytes: fs::read(entry.path())?,
                    executable: mode & 0o100 != 0,
                },
            );
        }
    }
    Ok(())
}

/// The names of the entries directly in `folder`, sorted; none when it cannot be listed.
#[must_use]
pub fn folder_names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
