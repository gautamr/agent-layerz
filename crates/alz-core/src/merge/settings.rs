//! The merge of settings (spec §7.9).
//!
//! The settings of each target are merged by [`SettingsTree::deep_merge`], a left fold in stack
//! order: mappings merge key by key, two lists are concatenated and de-duplicated by equality as
//! trees, and anything else, a change of kind included, is replaced by the more specific value.
//! The merged settings are an ordered map from [`TargetId`] to the settings of that target; the
//! only keys at the top level are the known targets, because a file with any other key is
//! refused when it is read and none of its settings reach the fold.
//!
//! The keys that have a floor in the key rules of the target's descriptor
//! ([`crate::item::KeyRules`]) are not folded
//! freely: a layer whose settings loosen one of them against what the more general layers merged
//! (a looser value, a list that gains an entry where none may, a change of kind) is a `settings`
//! error naming the layer, the file and the key path, and none of the file's settings are merged.
//! A target with no key rules, which has neither an adapter nor a descriptor-only entry, has no
//! floor.

use std::collections::BTreeMap;

use crate::diagnostic::{Diagnostics, Where};
use crate::item::{Settings, SettingsItems, SettingsTree, place};
use crate::merge::action::Action;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::target::Registry;
use alz_adapter::TargetId;

/// The state of the fold for settings: the settings of each target that some layer set, with the
/// layers that set them.
#[derive(Debug, Clone, Default)]
pub(super) struct SettingsState(BTreeMap<TargetId, Traced<SettingsTree>>);

impl SettingsState {
    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = SettingsItems::read(context.layer(), context.registry());
        match items.into_file() {
            None => (self, found),
            Some((path, file)) => {
                let origin = Origin::new(context.layer(), path.clone());
                let (state, problems) =
                    self.apply(&file, &origin, &place(layer, &path), context.registry());
                (state, found.merge(problems))
            }
        }
    }

    /// The state after the settings `file` of `layer`, and the problems it makes: each target it
    /// names is deep-merged into what the target had, unless a floor key of the file is looser
    /// than what the more general layers merged, in which case the state is unchanged.
    pub(super) fn apply(
        self,
        file: &Settings,
        origin: &Origin,
        at: &Where,
        registry: &Registry,
    ) -> (Self, Diagnostics) {
        let found = file
            .iter()
            .filter_map(|(target, tree)| {
                let rules = registry.key_rules(target)?;
                let before = self.0.get(target)?;
                Some(rules.loosenings(&before.value().to_tree(), &tree.to_tree()))
            })
            .flatten()
            .map(|loosening| loosening.diagnostic(at))
            .collect::<Diagnostics>();
        if found.has_errors() {
            return (self, found);
        }
        let merged = Self(file.iter().fold(self.0, |mut merged, (target, tree)| {
            let next = match merged.remove(target) {
                Some(before) => {
                    let combined = before.value().clone().deep_merge(tree.clone());
                    before.revised(combined, origin.clone(), Action::Extend)
                }
                None => Traced::new(tree.clone(), origin.clone(), Action::Define),
            };
            merged.insert(target.clone(), next);
            merged
        }));
        (merged, found)
    }

    /// The merged settings.
    pub(super) fn finish(self) -> BTreeMap<TargetId, Traced<SettingsTree>> {
        self.0
    }
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
