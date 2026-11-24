//! What the writer saw in the repository: the input of the pure plan.

use std::collections::{BTreeMap, BTreeSet};

use crate::path::SafeRelPath;
use crate::render::tree::fold;

/// The largest file the writer reads to compare it: 1 MiB. A larger file is
/// [`Observed::TooLarge`].
pub const MAX_COMPARED_BYTES: usize = 1024 * 1024;

/// What is at one path, found without following a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// Nothing is there, or a folder above it is missing.
    Absent,
    /// A regular file, at most [`MAX_COMPARED_BYTES`] long.
    Regular {
        /// The content.
        bytes: Vec<u8>,
        /// Whether the owner-execute bit is set.
        executable: bool,
    },
    /// The entry is a symbolic link, or a folder above it is one. It is never followed.
    Link,
    /// Not a regular file: a folder, a FIFO, a socket or a device, or a folder above the path is
    /// not a folder.
    NotRegular,
    /// A regular file longer than [`MAX_COMPARED_BYTES`]; its content was not kept.
    TooLarge,
}

static ABSENT: Observed = Observed::Absent;

/// The repository as the writer observed it: every path the plan asks about, the files found in
/// the folders the adapter manages, and the names found beside the generated paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Disk {
    seen: BTreeMap<SafeRelPath, Observed>,
    managed: BTreeSet<SafeRelPath>,
    names: BTreeMap<String, BTreeSet<SafeRelPath>>,
}

impl Disk {
    /// A disk of `seen` paths; `managed` names the paths that were found by listing a managed
    /// folder (each also in `seen`), and `listed` every name found by listing a folder, which is
    /// what a path that differs only in case is looked for in.
    #[must_use]
    pub fn new(
        seen: impl IntoIterator<Item = (SafeRelPath, Observed)>,
        managed: impl IntoIterator<Item = SafeRelPath>,
        listed: impl IntoIterator<Item = SafeRelPath>,
    ) -> Self {
        let managed: BTreeSet<SafeRelPath> = managed.into_iter().collect();
        let names = managed.iter().cloned().chain(listed).fold(
            BTreeMap::<String, BTreeSet<SafeRelPath>>::new(),
            |mut names, path| {
                names.entry(fold(&path)).or_default().insert(path);
                names
            },
        );
        Self {
            seen: seen.into_iter().collect(),
            managed,
            names,
        }
    }

    /// What was observed at `path`; [`Observed::Absent`] when the writer did not look.
    #[must_use]
    pub fn get(&self, path: &SafeRelPath) -> &Observed {
        self.seen.get(path).unwrap_or(&ABSENT)
    }

    /// The paths found in managed folders, in name order.
    pub fn managed(&self) -> impl Iterator<Item = &SafeRelPath> + '_ {
        self.managed.iter()
    }

    /// Whether the listing of the folder found `path` spelled exactly so: on a file system that
    /// ignores case, a name is listed only as it is stored.
    #[must_use]
    pub fn lists(&self, path: &SafeRelPath) -> bool {
        self.names
            .get(&fold(path))
            .is_some_and(|spellings| spellings.contains(path))
    }

    /// A listed name that is the same file as `path` on a file system that folds case, but is
    /// spelled differently.
    #[must_use]
    pub fn variant_of(&self, path: &SafeRelPath) -> Option<&SafeRelPath> {
        self.names
            .get(&fold(path))
            .and_then(|spellings| spellings.iter().find(|spelling| *spelling != path))
    }
}
