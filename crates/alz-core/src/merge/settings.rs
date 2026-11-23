//! The merge of settings (spec §7.9).
//!
//! The settings of each target are merged by [`SettingsTree::deep_merge`], a left fold in stack
//! order: mappings merge key by key, two lists are concatenated and de-duplicated by equality as
//! trees, and anything else, a change of kind included, is replaced by the more specific value.
//! The merged settings are an ordered map from [`TargetName`] to the settings of that target; the
//! only keys at the top level are the known targets, because a file with any other key is
//! refused when it is read and none of its settings reach the fold.

use std::collections::BTreeMap;

use crate::diagnostic::Diagnostics;
use crate::item::{Settings, SettingsItems, SettingsTree};
use crate::layer::LayerName;
use crate::merge::provenance::{Action, Traced};
use crate::merge::stack::LayerContext;
use crate::target::TargetName;

/// The state of the fold for settings: the settings of each target that some layer set, with the
/// layers that set them.
#[derive(Debug, Clone, Default)]
pub(super) struct SettingsState(BTreeMap<TargetName, Traced<SettingsTree>>);

impl SettingsState {
    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = SettingsItems::read(context.layer());
        match items.into_file() {
            None => (self, found),
            Some((_, file)) => (self.apply(&file, layer), found),
        }
    }

    /// The state after the settings `file` of `layer`: each target it names is deep-merged into
    /// what the target had.
    pub(super) fn apply(self, file: &Settings, layer: &LayerName) -> Self {
        Self(file.iter().fold(self.0, |mut merged, (target, tree)| {
            let next = match merged.remove(&target) {
                Some(before) => {
                    let combined = before.value().clone().deep_merge(tree.clone());
                    before.revised(combined, layer.clone(), Action::Extend)
                }
                None => Traced::new(tree.clone(), layer.clone(), Action::Define),
            };
            merged.insert(target, next);
            merged
        }))
    }

    /// The merged settings.
    pub(super) fn finish(self) -> BTreeMap<TargetName, Traced<SettingsTree>> {
        self.0
    }
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
