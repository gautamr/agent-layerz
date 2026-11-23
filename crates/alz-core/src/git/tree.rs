//! The files of a pinned layer's git tree, as `git ls-tree -r -z` lists them, once every path in
//! it has been judged (spec §5.3 "File names", §6 "A pinned `path:` layer"). A [`TreeListing`] is
//! the proof that nothing can be written from it that the layer cache must not hold: every path
//! is valid UTF-8 and a [`SafeRelPath`], and no two paths would be merged by one file system.

use std::collections::{BTreeMap, BTreeSet};

use unicode_normalization::UnicodeNormalization;

use crate::git::tree_entry::{EntryMode, TreeEntry};
use crate::lock::{ObjectId, is_skipped_name};
use crate::path::SafeRelPath;

/// Why a tree is refused, in the order of precedence: the larger refusal is the one reported when
/// several occur, as for the names of a walked layer (§5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ListingRefusal {
    /// Two paths that are equal after Unicode NFC normalisation and case folding, or a file and
    /// a folder with such names: one file system would merge them.
    Collision,
    /// A path with a backslash, a `.` or `..` segment or a `.git` segment, in any case or with a
    /// trailing dot or space.
    NotSafe,
    /// A path that is not valid UTF-8.
    NotUtf8,
}

/// A file of a tree: the blob it names and whether the tree records it as executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListedFile {
    /// The blob's id.
    pub blob: ObjectId,
    /// Whether the mode is `100755`: the owner-execute bit.
    pub executable: bool,
}

/// The regular files of a tree, by path. Links and submodules are skipped: they are never
/// written, so a hub with `CLAUDE.md -> AGENTS.md` keeps working.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreeListing {
    files: BTreeMap<SafeRelPath, ListedFile>,
}

/// The text in Unicode NFC, the form that names are compared in: HFS+ returns NFD and APFS keeps
/// the form it was written in (§6).
pub(crate) fn nfc(text: &str) -> String {
    text.nfc().collect()
}

/// Every proper folder prefix of `path`, and `path` itself.
fn prefixes(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/')
        .filter_map(|(index, _)| path.get(..index))
        .chain(std::iter::once(path))
}

/// Whether two of the paths, or of their folders, would be one name on a file system that folds
/// case and normalises Unicode, while being written differently.
fn has_collision<'p>(paths: impl Iterator<Item = &'p SafeRelPath>) -> bool {
    paths
        .flat_map(|path| prefixes(path.as_str()))
        .fold(
            BTreeMap::<String, BTreeSet<&str>>::new(),
            |mut groups, prefix| {
                groups
                    .entry(nfc(prefix).to_lowercase())
                    .or_default()
                    .insert(prefix);
                groups
            },
        )
        .values()
        .any(|group| group.len() > 1)
}

/// The path of an entry as a safe path, or what is wrong with it.
fn judged(entry: &TreeEntry) -> Result<SafeRelPath, ListingRefusal> {
    std::str::from_utf8(&entry.path)
        .map_err(|_not_utf8| ListingRefusal::NotUtf8)
        .and_then(|text| SafeRelPath::try_from(text).map_err(|_unsafe| ListingRefusal::NotSafe))
}

impl TreeListing {
    /// The listing of `entries`, the output of `git ls-tree -r -z`. Every entry is judged,
    /// links and submodules too, and nothing is written from a tree that is refused.
    ///
    /// # Errors
    ///
    /// The [`ListingRefusal`] that takes precedence: a path that is not valid UTF-8, then a path
    /// that is not safe, then two paths that would be merged.
    pub fn from_entries(entries: &[TreeEntry]) -> Result<Self, ListingRefusal> {
        let judgements: Vec<_> = entries.iter().map(judged).collect();
        if let Some(worst) = judgements
            .iter()
            .filter_map(|judgement| judgement.as_ref().err())
            .max()
        {
            return Err(*worst);
        }
        let files: BTreeMap<SafeRelPath, ListedFile> = entries
            .iter()
            .zip(judgements)
            .filter_map(|(entry, judgement)| {
                let path = judgement.ok()?;
                match entry.mode {
                    EntryMode::File { executable } => Some((
                        path,
                        ListedFile {
                            blob: entry.id,
                            executable,
                        },
                    )),
                    EntryMode::Directory | EntryMode::Link | EntryMode::Submodule => None,
                }
            })
            .collect();
        if has_collision(files.keys()) {
            Err(ListingRefusal::Collision)
        } else {
            Ok(Self { files })
        }
    }

    /// The files, in path order.
    pub fn files(&self) -> impl Iterator<Item = (&SafeRelPath, &ListedFile)> + '_ {
        self.files.iter()
    }

    /// How many files the tree lists.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the tree lists no file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The blobs the files name, each once, in id order.
    #[must_use]
    pub fn blob_ids(&self) -> Vec<ObjectId> {
        self.files
            .values()
            .map(|file| file.blob)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The files whose path has a segment the walk of a layer never enters (`.DS_Store`,
    /// `node_modules`): the walk cannot see them, so they are checked by their path.
    pub fn skipped(&self) -> impl Iterator<Item = (&SafeRelPath, &ListedFile)> + '_ {
        self.files()
            .filter(|(path, _)| path.segments().any(is_skipped_name))
    }
}

#[cfg(test)]
mod tests;
