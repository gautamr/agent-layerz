//! The output of a build before it is written (spec §4.3, §6): every generated file by its path,
//! in code point order, with the bytes and the mode to write it with.
//!
//! A path is an [`OutputPath`]. Two paths that a case-insensitive or normalising file system
//! would make one file (`.claude/settings.json` and `.claude/Settings.json`, or `é` as one code
//! point and as `e` with a combining accent) cannot both be in a [`FileTree`]: the second is
//! refused with an error that names both, and the first stays.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::path::{SafeRelPath, SafeRelPathError};
use crate::untrusted_text::Stripped;

/// A path a build may generate: a [`SafeRelPath`] with no control or format character, so that it
/// can be printed, and put in a header, without a line break or a hidden character.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutputPath(SafeRelPath);

/// Text that cannot be the path of a generated file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OutputPathError {
    /// Not a safe relative path.
    #[error("{}", SafeRelPathError.rule())]
    Unsafe,
    /// The path holds a control or format character.
    #[error("must hold no control or format character")]
    Unprintable,
}

impl From<SafeRelPathError> for OutputPathError {
    fn from(_: SafeRelPathError) -> Self {
        Self::Unsafe
    }
}

impl OutputPath {
    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The path as the safe relative path it is.
    #[must_use]
    pub const fn as_safe(&self) -> &SafeRelPath {
        &self.0
    }

    /// The key two paths share when a file system would make them one file: the path in
    /// normalisation form D, with ASCII upper case folded to lower case. Decomposing first means
    /// that U+00CD and `i` with a combining accent fold together, which they would not if the
    /// path were composed first. Only ASCII is folded. The protected paths (§4.5), the passthrough
    /// checks and the reserved names compare in the same way: a path that a file system folds
    /// further is a limit of §4.3, not something alz guesses.
    #[must_use]
    pub fn folded(&self) -> String {
        fold(&self.0)
    }
}

/// The key two paths share when a file system would make them one file, for any safe relative
/// path: see [`OutputPath::folded`].
#[must_use]
pub fn fold(path: &SafeRelPath) -> String {
    fold_text(path.as_str())
}

/// [`fold`] for any text, such as a path of an adapter's territory that is not a safe relative
/// path.
#[must_use]
pub fn fold_text(text: &str) -> String {
    text.nfd().collect::<String>().to_ascii_lowercase()
}

impl TryFrom<SafeRelPath> for OutputPath {
    type Error = OutputPathError;

    fn try_from(path: SafeRelPath) -> Result<Self, Self::Error> {
        if Stripped::from_untrusted(path.as_str()).as_str() == path.as_str() {
            Ok(Self(path))
        } else {
            Err(OutputPathError::Unprintable)
        }
    }
}

impl TryFrom<&str> for OutputPath {
    type Error = OutputPathError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::try_from(SafeRelPath::try_from(text)?)
    }
}

impl fmt::Display for OutputPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// How a generated file is written: `0644` or `0755`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileMode {
    /// `0644`.
    Regular,
    /// `0755`: a script a hook runs.
    Executable,
}

/// One generated file: its bytes and its mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// The content, written as it is.
    pub bytes: Vec<u8>,
    /// The mode.
    pub mode: FileMode,
}

impl GeneratedFile {
    /// A regular file with the bytes of `text`.
    #[must_use]
    pub fn text(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            mode: FileMode::Regular,
        }
    }
}

/// A path that cannot join a tree because another path of the tree is the same file on a file
/// system that folds case or normalises. It names both; the first stays in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct PathCollision {
    kept: OutputPath,
    rejected: OutputPath,
}

impl PathCollision {
    /// The path that was in the tree.
    #[must_use]
    pub const fn kept(&self) -> &OutputPath {
        &self.kept
    }

    /// The path that was refused.
    #[must_use]
    pub const fn rejected(&self) -> &OutputPath {
        &self.rejected
    }
}

impl fmt::Display for PathCollision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "two outputs claim one file: {} and {}",
            self.kept, self.rejected
        )
    }
}

/// The generated files of a build, by path in code point order (§6). It never holds two paths
/// with one [`OutputPath::folded`] key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileTree {
    files: BTreeMap<OutputPath, GeneratedFile>,
    claimed: BTreeMap<String, OutputPath>,
}

impl FileTree {
    /// An empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file.
    ///
    /// # Errors
    ///
    /// [`PathCollision`] when `path` has the folded key of a path already in the tree, itself
    /// included. The tree is unchanged.
    pub fn insert(&mut self, path: OutputPath, file: GeneratedFile) -> Result<(), PathCollision> {
        match self.claimed.entry(path.folded()) {
            Entry::Occupied(held) => Err(PathCollision {
                kept: held.get().clone(),
                rejected: path,
            }),
            Entry::Vacant(slot) => {
                slot.insert(path.clone());
                self.files.insert(path, file);
                Ok(())
            }
        }
    }

    /// The tree of `files`, or every collision found: a file that collides is left out, so each
    /// error names a path that is in the tree and one that is not.
    ///
    /// # Errors
    ///
    /// Every [`PathCollision`], in the order the files came.
    pub fn from_files(
        files: impl IntoIterator<Item = (OutputPath, GeneratedFile)>,
    ) -> Result<Self, Vec<PathCollision>> {
        let (tree, collisions) = files.into_iter().fold(
            (Self::new(), Vec::new()),
            |(mut tree, mut collisions), (path, file)| {
                if let Err(collision) = tree.insert(path, file) {
                    collisions.push(collision);
                }
                (tree, collisions)
            },
        );
        if collisions.is_empty() {
            Ok(tree)
        } else {
            Err(collisions)
        }
    }

    /// The file at exactly `path`.
    #[must_use]
    pub fn get(&self, path: &OutputPath) -> Option<&GeneratedFile> {
        self.files.get(path)
    }

    /// The path of the tree that `path` is the same file as on a folding file system, which is
    /// `path` itself when it is in the tree.
    #[must_use]
    pub fn claim_of(&self, path: &OutputPath) -> Option<&OutputPath> {
        self.claimed.get(&path.folded())
    }

    /// The files in path order.
    pub fn iter(&self) -> impl Iterator<Item = (&OutputPath, &GeneratedFile)> + '_ {
        self.files.iter()
    }

    /// The paths in path order.
    pub fn paths(&self) -> impl Iterator<Item = &OutputPath> + '_ {
        self.files.keys()
    }

    /// How many files the tree holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the tree holds no file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The tree without the files whose path `drop` accepts, and those paths in path order.
    #[must_use]
    pub fn without(self, drop: impl Fn(&OutputPath) -> bool) -> (Self, Vec<OutputPath>) {
        let (dropped, kept): (BTreeMap<_, _>, BTreeMap<_, _>) =
            self.files.into_iter().partition(|(path, _)| drop(path));
        let claimed = kept
            .keys()
            .map(|path| (path.folded(), path.clone()))
            .collect();
        (
            Self {
                files: kept,
                claimed,
            },
            dropped.into_keys().collect(),
        )
    }
}

#[cfg(test)]
mod tests;
