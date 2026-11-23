//! The merge of passthrough files (spec §7.11).
//!
//! Files are copied into place and the most specific layer wins per path: the merged files are an
//! ordered map from target to an ordered map from the path inside the target's folder to the file
//! ([`Replace`] per path, whole files, never combined). The layers that had the path before stay
//! in the file's provenance, marked as replaced. Each layer's use of a path warns when the layer
//! is read, whether or not a more specific layer replaces the file later.
//!
//! The merge of two layers' files is [`PassthroughState::merge`]. It is associative and has the
//! empty state as its identity, so the stack can be folded in any grouping.

use std::collections::BTreeMap;

use crate::diagnostic::Diagnostics;
use crate::item::{PassthroughFile, PassthroughItems};
use crate::layer::LayerName;
use crate::merge::algebra::Replace;
use crate::merge::provenance::{Action, Provenance, Traced};
use crate::merge::stack::LayerContext;
use crate::path::SafeRelPath;
use crate::target::TargetName;

/// The merged files of each target, by their path inside the target's folder, each with the
/// layers that had the path.
pub type MergedPassthrough = BTreeMap<TargetName, Replace<SafeRelPath, Traced<PassthroughFile>>>;

/// What the layers so far say about one path: the file of the most specific layer, and the layers
/// that had the path, most general first. The first defined the path and each later one replaced
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathChain {
    first: LayerName,
    replacers: Vec<LayerName>,
    latest: PassthroughFile,
}

impl PathChain {
    fn of(file: PassthroughFile, layer: LayerName) -> Self {
        Self {
            first: layer,
            replacers: Vec::new(),
            latest: file,
        }
    }

    /// The chain with what a more specific layer says: its file wins, and its layers come after.
    fn merge(self, later: Self) -> Self {
        Self {
            first: self.first,
            replacers: self
                .replacers
                .into_iter()
                .chain([later.first])
                .chain(later.replacers)
                .collect(),
            latest: later.latest,
        }
    }

    fn into_traced(self) -> Traced<PassthroughFile> {
        Traced::with_provenance(
            self.latest,
            self.replacers.into_iter().fold(
                Provenance::first(self.first, Action::Define),
                |provenance, layer| provenance.then(layer, Action::Replace),
            ),
        )
    }
}

/// The state of the fold for passthrough files. `PassthroughState::default()` is the identity of
/// [`PassthroughState::merge`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PassthroughState(BTreeMap<TargetName, BTreeMap<SafeRelPath, PathChain>>);

impl PassthroughState {
    /// The files of one layer: each path has the layer as its only contributor.
    fn of_layer(items: PassthroughItems, layer: &LayerName) -> Self {
        Self(
            items
                .into_files()
                .into_iter()
                .map(|file| {
                    (
                        file.target(),
                        BTreeMap::from([(file.path().clone(), PathChain::of(file, layer.clone()))]),
                    )
                })
                .fold(BTreeMap::new(), |all, (target, files)| {
                    union_with(all, BTreeMap::from([(target, files)]), |before, later| {
                        union_with(before, later, PathChain::merge)
                    })
                }),
        )
    }

    /// The state with what a more specific layer says: per target and path, its file wins.
    pub(super) fn merge(self, later: Self) -> Self {
        Self(union_with(self.0, later.0, |before, later| {
            union_with(before, later, PathChain::merge)
        }))
    }

    /// The state after one more, more specific, layer, with the warning of each file it uses and
    /// the problems of the files it holds.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = PassthroughItems::read(context.layer());
        (self.merge(Self::of_layer(items, layer)), found)
    }

    /// The merged files.
    pub(super) fn finish(self) -> MergedPassthrough {
        self.0
            .into_iter()
            .map(|(target, files)| {
                (
                    target,
                    files
                        .into_iter()
                        .map(|(path, chain)| (path, chain.into_traced()))
                        .collect(),
                )
            })
            .collect()
    }
}

/// The union of two ordered maps, where a key in both gets `combine(earlier, later)`.
fn union_with<K: Ord, V>(
    earlier: BTreeMap<K, V>,
    later: BTreeMap<K, V>,
    combine: impl Fn(V, V) -> V,
) -> BTreeMap<K, V> {
    later.into_iter().fold(earlier, |mut map, (key, value)| {
        let combined = match map.remove(&key) {
            Some(before) => combine(before, value),
            None => value,
        };
        map.insert(key, combined);
        map
    })
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
