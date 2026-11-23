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

use crate::lock::FileDigest;
use crate::path::SafeRelPath;

pub use verdict::{LayerRefusal, ReadLayer, scan_verdict};

/// The most entries account resolution lets the walk list (§7.13).
const MAX_ENTRIES: usize = 2000;

/// The most bytes of files account resolution reads in all, 16 MiB (§7.13).
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// Why a layer is walked. Account resolution has budgets; the build has none (§7.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Account resolution (R6): the walk stops at entry 2001 and reads at most 16 MiB.
    Resolution,
    /// `alz build` and the commands that rebuild: no limit.
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
    /// A name that is not valid Unicode (`OsStr::to_str` fails).
    NotUtf8,
}

/// What the walk of one layer directory saw. It is built by the walk, one fact at a time, and
/// judged by [`scan_verdict`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerScan {
    listed: usize,
    name_problem: Option<NameProblem>,
    files: Vec<(SafeRelPath, FileDigest)>,
    env_files: Vec<SafeRelPath>,
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

    /// The scan after a regular file was read in full: its path, the digest of its bytes and how
    /// many bytes were read.
    #[must_use]
    pub fn with_file(self, path: SafeRelPath, digest: FileDigest, length: u64) -> Self {
        let bytes = self.bytes.saturating_add(length);
        let mut files = self.files;
        files.push((path, digest));
        Self {
            files,
            bytes,
            ..self
        }
    }

    /// The scan after a `.env` file was listed. It was never opened.
    #[must_use]
    pub fn with_env_file(self, path: SafeRelPath) -> Self {
        let mut env_files = self.env_files;
        env_files.push(path);
        Self { env_files, ..self }
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
