//! What the walk of a layer directory saw, as plain facts (spec §5.3 "File names", "`.env` files"
//! and "Cache integrity"; §7.13 "No fetch on resolve"), and the verdict the order of checks draws
//! from them. The walk itself, which lists names and reads files, is `alz-io`'s; nothing here
//! touches a file.
//!
//! A [`LayerScan`] holds counts and flags, never a name that is not valid Unicode, so a verdict
//! cannot print one. Facts are joined by commutative operations (a sum, a flag, the larger
//! [`NameProblem`], a set), so the verdict never depends on the order in which the file system
//! lists names (§5.3).

mod verdict;

use crate::lock::ObjectId;
use crate::path::SafeRelPath;

pub use verdict::{LayerRefusal, ReadLayer, scan_verdict};

/// The most entries account resolution lets the walk list (§7.13).
const MAX_ENTRIES: usize = 2000;

/// The most bytes of files account resolution reads in all, 16 MiB (§7.13).
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The most bytes of one file a walk reads, 16 MiB, in the build too (§5.3 "Cache integrity"): a
/// file of more bytes is unreadable.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Why a layer is walked. Account resolution has budgets in all; the build caps each file at
/// [`MAX_FILE_BYTES`] and has no other limit (§7.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Account resolution (R6): the walk stops at entry 2001 and reads at most 16 MiB.
    Resolution,
    /// `alz build` and the commands that rebuild: no budget in all, but no file over 16 MiB.
    Build,
}

/// The budgets of a walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    entries: usize,
    bytes: u64,
}

impl Limits {
    /// The most entries that may be listed: the walk stops at the entry after it.
    #[must_use]
    pub const fn entries(self) -> usize {
        self.entries
    }

    /// The most bytes that may be read from files in all.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.bytes
    }
}

impl Purpose {
    /// The budgets of the walk, or `None` when there are none.
    #[must_use]
    pub const fn limits(self) -> Option<Limits> {
        match self {
            Self::Resolution => Some(Limits {
                entries: MAX_ENTRIES,
                bytes: MAX_BYTES,
            }),
            Self::Build => None,
        }
    }
}

/// What is wrong with the names of a layer. Both make the layer invalid; the first is the one the
/// spec words (§5.3 "File names"), so it is the larger one and wins when both occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameProblem {
    /// A path that is valid Unicode but is no [`SafeRelPath`] (a backslash, or `.git` in another
    /// case), so no hash can be taken over it.
    NotSafe,
    /// A name that is valid Unicode and holds a control or format character, which would start a
    /// new line in a generated header or be written to a terminal.
    ControlOrFormat,
    /// A name that is not valid Unicode (`OsStr::to_str` fails).
    NotUtf8,
}

/// What the walk saw of one regular file when it was asked to compare the layer with a git tree
/// (spec §6 "A pinned `path:` layer"): the blob id git computes over its bytes and its
/// owner-execute bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileFacts {
    /// The file's path.
    pub path: SafeRelPath,
    /// Whether the owner-execute bit is set.
    pub executable: bool,
    /// The blob id of the bytes that were read.
    pub blob: ObjectId,
}

/// What the walk of one layer directory saw. It is built by the walk, one fact at a time, and
/// judged by [`scan_verdict`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerScan {
    listed: usize,
    name_problem: Option<NameProblem>,
    files: Vec<(SafeRelPath, Vec<u8>)>,
    executable: Vec<SafeRelPath>,
    env_files: Vec<SafeRelPath>,
    git_files: Vec<GitFileFacts>,
    other_entries: usize,
    bytes: u64,
    unreadable: bool,
}

impl LayerScan {
    /// A scan that has seen nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The scan after one more entry was listed: a file, a folder, a link or anything else. The
    /// skipped names and what lies below them are never listed, so never counted.
    #[must_use]
    pub fn listed_entry(self) -> Self {
        Self {
            listed: self.listed.saturating_add(1),
            ..self
        }
    }

    /// The scan after a name was found to be a problem.
    #[must_use]
    pub fn with_name_problem(self, problem: NameProblem) -> Self {
        Self {
            name_problem: Some(self.name_problem.map_or(problem, |seen| seen.max(problem))),
            ..self
        }
    }

    /// The scan after a regular file was read in full: its path and the bytes that were read. The
    /// scan keeps them, so that the hash and the merge cover the same content.
    #[must_use]
    pub fn with_file(self, path: SafeRelPath, content: Vec<u8>) -> Self {
        let bytes = self
            .bytes
            .saturating_add(u64::try_from(content.len()).unwrap_or(u64::MAX));
        let mut files = self.files;
        files.push((path, content));
        Self {
            files,
            bytes,
            ..self
        }
    }

    /// The scan after a regular file was read in full and its owner-execute bit was found set.
    #[must_use]
    pub fn with_executable(self, path: SafeRelPath) -> Self {
        let mut executable = self.executable;
        executable.push(path);
        Self { executable, ..self }
    }

    /// The scan after a `.env` file was listed. It was never opened.
    #[must_use]
    pub fn with_env_file(self, path: SafeRelPath) -> Self {
        let mut env_files = self.env_files;
        env_files.push(path);
        Self { env_files, ..self }
    }

    /// The scan after the git facts of a file that was read in full were recorded.
    #[must_use]
    pub fn with_git_facts(self, facts: GitFileFacts) -> Self {
        let mut git_files = self.git_files;
        git_files.push(facts);
        Self { git_files, ..self }
    }

    /// The scan after an entry that is no file and no folder was listed: a link, a pipe, a socket
    /// or a device. It was never opened.
    #[must_use]
    pub fn with_other_entry(self) -> Self {
        Self {
            other_entries: self.other_entries.saturating_add(1),
            ..self
        }
    }

    /// The scan after `length` more bytes were read, or counted as read when a file was refused
    /// for passing the budget.
    #[must_use]
    pub fn with_bytes(self, length: u64) -> Self {
        Self {
            bytes: self.bytes.saturating_add(length),
            ..self
        }
    }

    /// The scan after a folder or file could not be opened, listed or read.
    #[must_use]
    pub fn with_unreadable(self) -> Self {
        Self {
            unreadable: true,
            ..self
        }
    }

    /// The git facts of the files that were read, when the walk was asked for them.
    #[must_use]
    pub fn git_files(&self) -> &[GitFileFacts] {
        &self.git_files
    }

    /// The paths of the `.env` files that were listed.
    #[must_use]
    pub fn env_files(&self) -> &[SafeRelPath] {
        &self.env_files
    }

    /// How many links and other entries that are neither file nor folder were listed.
    #[must_use]
    pub const fn other_entries(&self) -> usize {
        self.other_entries
    }

    /// How many entries were listed.
    #[must_use]
    pub const fn listed(&self) -> usize {
        self.listed
    }

    /// How many bytes were read, or counted as read.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Whether the walk has listed more entries than `purpose` allows, which is where it stops.
    #[must_use]
    pub fn is_past_entry_limit(&self, purpose: Purpose) -> bool {
        purpose
            .limits()
            .is_some_and(|limits| self.listed > limits.entries())
    }

    /// Whether the files read so far already pass the byte budget of `purpose`, after which no
    /// further file is read.
    #[must_use]
    pub fn is_past_byte_limit(&self, purpose: Purpose) -> bool {
        purpose
            .limits()
            .is_some_and(|limits| self.bytes > limits.bytes())
    }
}

#[cfg(test)]
mod tests;
