//! Why the repository writer failed.

use std::io;

use alz_core::Stripped;
use alz_core::path::SafeRelPath;
use thiserror::Error;

/// A failure of the repository root. The paths are those of the plan, printed stripped.
#[derive(Debug, Error)]
pub enum RepoError {
    /// The operating system refused an operation.
    #[error("{operation} {path} failed")]
    Io {
        /// What was being done: `reading`, `listing`, `writing`, `removing`.
        operation: &'static str,
        /// The path, stripped.
        path: String,
        /// The operating system's error.
        #[source]
        source: io::Error,
    },
    /// A folder holds an entry whose name is not a safe relative path (invalid UTF-8, a backslash
    /// or `.git`), so it cannot be compared with anything.
    #[error("{folder} holds an entry whose name alz cannot read")]
    UnnameableEntry {
        /// The folder, stripped.
        folder: String,
    },
    /// A managed folder holds more entries than the writer looks at.
    #[error("{folder} holds more than {limit} entries")]
    TooManyEntries {
        /// The folder, stripped.
        folder: String,
        /// The limit.
        limit: usize,
    },
    /// Something is at a path that was to be created: a file, a folder or a link. Nothing was
    /// written there.
    #[error("{path} already exists")]
    Exists {
        /// The path, stripped.
        path: String,
    },
    /// A path was a folder, a file or a link when the plan was made and is something else now:
    /// something changed the repository while the build ran. Nothing was written through it.
    #[error("{path} changed while alz was writing")]
    Changed {
        /// The path, stripped.
        path: String,
    },
}

impl RepoError {
    pub(super) fn io(operation: &'static str, path: &SafeRelPath, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: Stripped::from_untrusted(path.as_str()).to_string(),
            source,
        }
    }

    pub(super) fn exists(path: &SafeRelPath) -> Self {
        Self::Exists {
            path: Stripped::from_untrusted(path.as_str()).to_string(),
        }
    }

    pub(super) fn changed(path: &SafeRelPath) -> Self {
        Self::Changed {
            path: Stripped::from_untrusted(path.as_str()).to_string(),
        }
    }
}
