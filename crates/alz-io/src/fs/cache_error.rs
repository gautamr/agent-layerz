//! Why the layer cache could not be used, and the diagnostics that say so (spec §6).

use std::io;

use alz_core::cache::{CacheLocation, EntryName};
use alz_core::{Code, Diagnostic, Message, Stripped};
use thiserror::Error;

use crate::fs::walk_error::WalkError;

/// Why the layer cache or an entry in it could not be used. The error holds no path.
#[derive(Debug, Error)]
pub enum CacheError {
    /// The cache root touches an account folder or the home folder (§9.1).
    #[error("the layer cache touches an account folder or the home folder")]
    TouchesAccountFolder,
    /// An absolute `XDG_CACHE_HOME` puts the cache root inside the repository.
    #[error("the layer cache lies inside the repository")]
    InsideRepository,
    /// The cache root could not be opened or created.
    #[error("the layer cache cannot be opened")]
    Unusable(#[source] WalkError),
    /// An entry is a link or something else but a folder. It is never followed.
    #[error("a cache entry is not a folder")]
    EntryNotDirectory(EntryName),
    /// A temporary entry is not the folder this process just made: the name was swapped.
    #[error("a temporary cache entry was replaced")]
    TempEntryReplaced,
    /// The entry that a publish found in place vanished before it could be opened.
    #[error("a cache entry vanished")]
    EntryVanished(EntryName),
    /// Any other failure of the operating system.
    #[error("the layer cache cannot be used")]
    Io(#[source] io::Error),
}

impl From<io::Error> for CacheError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl CacheError {
    /// The error as a diagnostic about the cache at `location`. The directory is stripped, as in
    /// every diagnostic (decision-26, class 1).
    #[must_use]
    pub fn into_diagnostic(self, location: &CacheLocation) -> Diagnostic {
        let dir = dir_of(location);
        let message = match self {
            Self::TouchesAccountFolder => Message::new("the layer cache at ")
                .then(dir)
                .then(" touches an account folder or the home folder; set XDG_CACHE_HOME to another folder"),
            Self::InsideRepository => Message::new("the layer cache at ")
                .then(dir)
                .then(" lies inside the repository; set XDG_CACHE_HOME to a folder outside it"),
            Self::EntryNotDirectory(name) => entry_message(&name, dir)
                .then(" must be a folder, not a link or a file; delete it and run alz again"),
            Self::EntryVanished(name) => {
                entry_message(&name, dir).then(" was removed while alz used it; run alz again")
            }
            Self::Unusable(_) | Self::TempEntryReplaced | Self::Io(_) => {
                Message::new("cannot use the layer cache at ").then(dir)
            }
        };
        Diagnostic::error(Code::Cache, None, message)
    }
}

/// Why the index entry of a pinned `path:` layer could not be written (§6).
#[derive(Debug, Error)]
pub enum IndexError {
    /// A folder is at the index name: the cache error, "delete it and rebuild".
    #[error("a folder is at the name of an index entry")]
    Blocked(EntryName),
    /// Any other failure, such as a read-only cache: the warning `cache-index`.
    #[error("an index entry cannot be written")]
    Unwritable(EntryName, #[source] io::Error),
}

impl IndexError {
    /// The error as a diagnostic about the cache at `location`.
    #[must_use]
    pub fn into_diagnostic(self, location: &CacheLocation) -> Diagnostic {
        let dir = dir_of(location);
        match self {
            Self::Blocked(name) => Diagnostic::error(
                Code::Cache,
                None,
                entry_message(&name, dir).then(" is a folder; delete it and rebuild"),
            ),
            Self::Unwritable(name, _) => Diagnostic::warning(
                Code::CacheIndex,
                None,
                entry_message(&name, dir).then(" cannot be written"),
            ),
        }
    }
}

fn dir_of(location: &CacheLocation) -> Stripped {
    Stripped::from_untrusted(&location.path().to_string())
}

/// `cache entry <name> in the layer cache at <dir>`; the name is printable by construction.
fn entry_message(name: &EntryName, dir: Stripped) -> Message {
    Message::new("cache entry ")
        .then(Stripped::from_untrusted(&name.to_string()))
        .then(" in the layer cache at ")
        .then(dir)
}

#[cfg(test)]
mod tests {
    use alz_core::cache::CacheLocation;

    use super::CacheError;

    #[test]
    fn the_refusals_are_the_error_cache_with_the_spec_text() {
        let location = CacheLocation::from_env(Some("/h"), Some("/c\u{1b}[0m")).expect("root");
        assert_eq!(
            CacheError::TouchesAccountFolder
                .into_diagnostic(&location)
                .to_string(),
            "error [cache] the layer cache at /c[0m/agent-layerz touches an account folder or the home folder; set XDG_CACHE_HOME to another folder"
        );
        assert_eq!(
            CacheError::InsideRepository
                .into_diagnostic(&location)
                .to_string(),
            "error [cache] the layer cache at /c[0m/agent-layerz lies inside the repository; set XDG_CACHE_HOME to a folder outside it"
        );
    }
}
