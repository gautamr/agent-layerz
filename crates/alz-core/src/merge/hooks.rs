//! The merge of hooks (spec §7.5).
//!
//! Every hook of every layer is kept, in stack order and then file order. A layer may define an id
//! that no layer has used. A layer may disable a hook of a more general layer by id, unless the
//! hook is locked: a locked hook stays, and the disable is a `hook-locked` error that names both
//! layers. Disabling an id that no more general layer defines is `unresolved`. A disabled id stays
//! taken: a more specific layer may not define it again.
//!
//! The lock is a floor in the sense of [`Floor`](super::algebra::Floor): no later layer can take it
//! away, because the only way to remove a hook is a disable, and a disable is refused for a locked
//! hook.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{Hook, HookEntry, HookId, HookItems, Lock, place};
use crate::layer::LayerName;
use crate::merge::provenance::{Action, Traced};
use crate::merge::stack::LayerContext;
use crate::untrusted_text::QuotedKey;

/// A hook that stands, and the layer that defined it.
#[derive(Debug)]
struct Held {
    origin: LayerName,
    hook: Traced<Hook>,
}

/// The state of the fold for hooks: the hooks that stand, in order, and the ids that layers
/// disabled.
#[derive(Debug, Default)]
pub(super) struct HookState {
    hooks: Vec<Held>,
    disabled: BTreeMap<HookId, LayerName>,
}

impl HookState {
    /// The state after one more, more specific, layer: its entries in file order.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = HookItems::read(context.layer());
        match items.into_file() {
            None => (self, found),
            Some((path, entries)) => {
                let at = place(layer, &path);
                entries
                    .into_iter()
                    .fold((self, found), |(state, found), entry| match entry {
                        HookEntry::Define(hook) => state.define(hook, layer, &at, found),
                        HookEntry::Disable(id) => state.disable(&id, layer, &at, found),
                    })
            }
        }
    }

    /// The hooks that stand, in stack order and then file order.
    pub(super) fn finish(self) -> Vec<Traced<Hook>> {
        self.hooks.into_iter().map(|held| held.hook).collect()
    }

    fn define(
        self,
        hook: Hook,
        layer: &LayerName,
        at: &Where,
        found: Diagnostics,
    ) -> (Self, Diagnostics) {
        let taken = self
            .hooks
            .iter()
            .find(|held| held.hook.value().id() == hook.id())
            .map(|held| ("is already defined by layer ", held.origin.clone()))
            .or_else(|| {
                self.disabled
                    .get(hook.id())
                    .map(|by| ("was disabled by layer ", by.clone()))
            });
        match taken {
            Some((how, other)) => {
                let problem = Diagnostic::error(
                    Code::Hooks,
                    Some(at.clone()),
                    named(hook.id())
                        .then(" ")
                        .then(how)
                        .then(MessagePart::printable(&other))
                        .then(" and cannot be defined again"),
                );
                (self, found.with(problem))
            }
            None => (
                Self {
                    hooks: self
                        .hooks
                        .into_iter()
                        .chain([Held {
                            origin: layer.clone(),
                            hook: Traced::new(hook, layer.clone(), Action::Define),
                        }])
                        .collect(),
                    ..self
                },
                found,
            ),
        }
    }

    /// Disables the hook `id` of a more general layer, unless it is locked. A hook of the same
    /// layer is no hook of a more general layer, so it cannot be disabled either.
    fn disable(
        self,
        id: &HookId,
        layer: &LayerName,
        at: &Where,
        found: Diagnostics,
    ) -> (Self, Diagnostics) {
        let target = self
            .hooks
            .iter()
            .enumerate()
            .find(|(_, held)| held.hook.value().id() == id && held.origin != *layer)
            .map(|(index, held)| (index, held.hook.value().lock(), held.origin.clone()));
        match target {
            None => {
                let problem = Diagnostic::error(
                    Code::Unresolved,
                    Some(at.clone()),
                    Message::new("cannot disable hook ")
                        .then(QuotedKey::from_untrusted(id.as_str()))
                        .then(": no more general layer defines it"),
                );
                (self, found.with(problem))
            }
            Some((_, Lock::Locked, owner)) => {
                let problem = Diagnostic::error(
                    Code::HookLocked,
                    Some(at.clone()),
                    named(id)
                        .then(" is locked by layer ")
                        .then(MessagePart::printable(&owner))
                        .then(" and cannot be disabled by layer ")
                        .then(MessagePart::printable(layer)),
                );
                (self, found.with(problem))
            }
            Some((index, Lock::Open, _)) => (
                Self {
                    hooks: self
                        .hooks
                        .into_iter()
                        .enumerate()
                        .filter_map(|(position, held)| (position != index).then_some(held))
                        .collect(),
                    disabled: self
                        .disabled
                        .into_iter()
                        .chain([(id.clone(), layer.clone())])
                        .collect(),
                },
                found,
            ),
        }
    }
}

/// `hook '<id>'`.
fn named(id: &HookId) -> Message {
    Message::new("hook ").then(QuotedKey::from_untrusted(id.as_str()))
}

#[cfg(test)]
mod tests;
