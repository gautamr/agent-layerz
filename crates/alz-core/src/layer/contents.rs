//! The files of a layer as the walk read them: each path with the bytes that were hashed.
//!
//! The content hash of a layer is taken over these bytes ([`LayerContents::files`]) and the merge
//! reads these bytes, so what the lock check covered is what the merge uses, with no second read
//! in between (spec §5.3, §6).

use std::collections::BTreeMap;
use std::fmt;

use crate::lock::{LayerFiles, is_hashed};
use crate::path::SafeRelPath;

/// The files of a layer that the content hash covers, with their bytes. The paths are in a map,
/// so name order (spec §6) holds by construction. A path the hash does not cover (see
/// [`is_hashed`]) is never held, so [`LayerContents::files`] has a digest for exactly these paths.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct LayerContents(BTreeMap<SafeRelPath, Vec<u8>>);

impl LayerContents {
    /// The number of files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the layer has no hashed file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The bytes of the file at `path`.
    #[must_use]
    pub fn get(&self, path: &SafeRelPath) -> Option<&[u8]> {
        self.0.get(path).map(Vec::as_slice)
    }

    /// The files and their bytes in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&SafeRelPath, &[u8])> + '_ {
        self.0.iter().map(|(path, bytes)| (path, bytes.as_slice()))
    }

    /// The paths of the files, in name order.
    pub fn paths(&self) -> impl Iterator<Item = &SafeRelPath> + '_ {
        self.0.keys()
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
        Self(
            files
                .into_iter()
                .filter(|(path, _)| is_hashed(path))
                .collect(),
        )
    }
}

impl fmt::Debug for LayerContents {
    /// The paths and the sizes, never the bytes: a layer's files may hold anything.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(self.0.iter().map(|(path, bytes)| (path, bytes.len())))
            .finish()
    }
}

#[cfg(test)]
mod tests;
