//! The files of a layer as the walk read them: each path with the bytes that were hashed.
//!
//! The content hash of a layer is taken over these bytes ([`LayerContents::files`]) and the merge
//! reads these bytes, so what the lock check covered is what the merge uses, with no second read
//! in between (spec §5.3, §6).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::lock::{LayerFiles, is_hashed};
use crate::path::SafeRelPath;

/// The files of a layer that the content hash covers, with their bytes. The paths are in a map,
/// so name order (spec §6) holds by construction. A path the hash does not cover (see
/// [`is_hashed`]) is never held, so [`LayerContents::files`] has a digest for exactly these paths.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct LayerContents {
    files: BTreeMap<SafeRelPath, Vec<u8>>,
    executable: BTreeSet<SafeRelPath>,
}

impl LayerContents {
    /// The number of files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the layer has no hashed file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The bytes of the file at `path`.
    #[must_use]
    pub fn get(&self, path: &SafeRelPath) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// The files and their bytes in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&SafeRelPath, &[u8])> + '_ {
        self.files
            .iter()
            .map(|(path, bytes)| (path, bytes.as_slice()))
    }

    /// The paths of the files, in name order.
    pub fn paths(&self) -> impl Iterator<Item = &SafeRelPath> + '_ {
        self.files.keys()
    }

    /// The contents with the owner-execute bit set on each of `paths` that is a file of the layer:
    /// the bit a skill's script or a hook script is written with (§8.1). The content hash does
    /// not cover it, so it does not change [`LayerContents::files`].
    #[must_use]
    pub fn with_executable(self, paths: impl IntoIterator<Item = SafeRelPath>) -> Self {
        let executable = paths
            .into_iter()
            .filter(|path| self.files.contains_key(path))
            .chain(self.executable)
            .collect();
        Self { executable, ..self }
    }

    /// Whether the file at `path` has the owner-execute bit.
    #[must_use]
    pub fn is_executable(&self, path: &SafeRelPath) -> bool {
        self.executable.contains(path)
    }

    /// The digest of each file's bytes: what the content hash covers.
    #[must_use]
    pub fn files(&self) -> LayerFiles {
        LayerFiles::from_contents(self.iter().map(|(path, bytes)| (path.clone(), bytes)))
    }
}

impl FromIterator<(SafeRelPath, Vec<u8>)> for LayerContents {
    /// A path the hash does not cover (see [`is_hashed`]) is dropped, as [`LayerFiles`] drops it.
    /// A path that comes twice keeps the last bytes.
    fn from_iter<I: IntoIterator<Item = (SafeRelPath, Vec<u8>)>>(files: I) -> Self {
        Self {
            files: files
                .into_iter()
                .filter(|(path, _)| is_hashed(path))
                .collect(),
            executable: BTreeSet::new(),
        }
    }
}

impl fmt::Debug for LayerContents {
    /// The paths and the sizes, never the bytes: a layer's files may hold anything.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(self.files.iter().map(|(path, bytes)| (path, bytes.len())))
            .finish()
    }
}

#[cfg(test)]
mod tests;
