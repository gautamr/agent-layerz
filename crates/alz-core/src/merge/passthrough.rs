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

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::item::{PassthroughFile, PassthroughItems};
use crate::merge::action::Action;
use crate::merge::algebra::Replace;
use crate::merge::provenance::{Origin, Provenance, Traced};
use crate::merge::stack::LayerContext;
use crate::path::SafeRelPath;
use crate::stack::ResolvedLayer;
use alz_adapter::TargetId;

/// The merged files of each target, by their path inside the target's folder, each with the
/// layers that had the path.
pub type MergedPassthrough = BTreeMap<TargetId, Replace<SafeRelPath, Traced<PassthroughFile>>>;

/// What the layers so far say about one path: the file of the most specific layer, and the layers
/// that had the path, most general first. The first defined the path and each later one replaced
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathChain {
    first: Origin,
    replacers: Vec<Origin>,
    latest: PassthroughFile,
}

impl PathChain {
    fn of(file: PassthroughFile, origin: Origin) -> Self {
        Self {
            first: origin,
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
pub(super) struct PassthroughState {
    files: BTreeMap<TargetId, BTreeMap<SafeRelPath, PathChain>>,
    /// The warnings of the files read, each with its file's target, in layer and path order;
    /// `finish` keeps those of the targets the build is for (§5.2 **Target ids**).
    warnings: Vec<(TargetId, Diagnostic)>,
}

impl PassthroughState {
    /// The files of one layer: each path has the layer as its only contributor.
    fn of_layer(items: PassthroughItems, layer: &ResolvedLayer) -> Self {
        Self {
            warnings: Vec::new(),
            files: items
                .into_files()
                .into_iter()
                .map(|file| {
                    let origin = Origin::new(layer, file.source().clone());
                    (
                        file.target().clone(),
                        BTreeMap::from([(file.path().clone(), PathChain::of(file, origin))]),
                    )
                })
                .fold(BTreeMap::new(), |all, (target, files)| {
                    union_with(all, BTreeMap::from([(target, files)]), |before, later| {
                        union_with(before, later, PathChain::merge)
                    })
                }),
        }
    }

    /// The state with what a more specific layer says: per target and path, its file wins.
    pub(super) fn merge(self, later: Self) -> Self {
        Self {
            files: union_with(self.files, later.files, |before, later| {
                union_with(before, later, PathChain::merge)
            }),
            warnings: self.warnings.into_iter().chain(later.warnings).collect(),
        }
    }

    /// The state after one more, more specific, layer, with the warning of each file it uses and
    /// the problems of the files it holds. For a layer linted alone, which has no targets.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let (items, found) = PassthroughItems::read(context.layer());
        (self.merge(Self::of_layer(items, context.layer())), found)
    }

    /// The state after one more layer, with the problems of the files it holds. The warnings of
    /// its files stay in the state until [`PassthroughState::finish`], which knows the targets.
    pub(super) fn absorb_deferring(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let (items, warnings, found) = PassthroughItems::read_apart(context.layer());
        let layer = Self {
            warnings,
            ..Self::of_layer(items, context.layer())
        };
        (self.merge(layer), found)
    }

    /// The merged files, and the warnings of the files whose target `built_for` accepts: a file
    /// of a target id that is not built for is checked and never written, so it has no warning
    /// (§5.2 **Target ids**).
    pub(super) fn finish(
        self,
        built_for: impl Fn(&TargetId) -> bool,
    ) -> (MergedPassthrough, Diagnostics) {
        let warnings = self
            .warnings
            .into_iter()
            .filter(|(target, _)| built_for(target))
            .fold(Diagnostics::new(), |found, (_, warning)| {
                found.with(warning)
            });
        let files = self
            .files
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
            .collect();
        (files, warnings)
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
