//! The merge of model routing (spec §7.8).
//!
//! Aliases and roles are deep-merged and the most specific value wins: an alias keeps the targets
//! of the more general layers and a later layer replaces the model of a target it names, a role
//! and `default` are replaced by the most specific layer that sets them. Each alias, role and the
//! default carries the layers that set it.
//!
//! Whether a role or `default` names an alias that exists is known after the whole stack is
//! merged, because a more general layer may define the alias: one that names no alias is an
//! `unresolved` error, which `alz lint` leaves out.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::item::{ConcreteModel, ModelAlias, ModelItems, Models, Role};
use crate::merge::action::Action;
use crate::merge::algebra::Replace;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::untrusted_text::QuotedKey;
use alz_adapter::TargetId;

/// The models of each target of an alias.
pub type TargetModels = Replace<TargetId, ConcreteModel>;

/// The model routing after the merge.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergedModels {
    aliases: BTreeMap<ModelAlias, Traced<TargetModels>>,
    /// The aliases in the order a layer first wrote them, the most general layer first.
    order: Vec<ModelAlias>,
    roles: BTreeMap<Role, Traced<ModelAlias>>,
    default: Option<Traced<ModelAlias>>,
}

impl MergedModels {
    /// The aliases, each with the layers that set it.
    #[must_use]
    pub const fn aliases(&self) -> &BTreeMap<ModelAlias, Traced<TargetModels>> {
        &self.aliases
    }

    /// The aliases in the order the spec lists them, which decides which of two aliases of
    /// one model adoption reuses (spec §4.3): names that are array indexes (`7`) first, ascending,
    /// as the spec puts integer-like keys, then the others in the order a layer first wrote
    /// them, the most general layer first.
    pub fn aliases_in_definition_order(&self) -> impl Iterator<Item = &ModelAlias> {
        let (mut indexes, names): (Vec<_>, Vec<_>) = self
            .order
            .iter()
            .partition(|alias| array_index(alias.as_str()).is_some());
        indexes.sort_by_key(|alias| array_index(alias.as_str()));
        indexes.into_iter().chain(names)
    }

    /// The alias each role routes to, each with the layers that set it.
    #[must_use]
    pub const fn roles(&self) -> &BTreeMap<Role, Traced<ModelAlias>> {
        &self.roles
    }

    /// The alias of the session, when a layer set one.
    #[must_use]
    pub const fn default(&self) -> Option<&Traced<ModelAlias>> {
        self.default.as_ref()
    }

    /// The concrete model that `alias` names for `target`. `None` when the alias does not exist
    /// or has no model for the target.
    #[must_use]
    pub fn model(&self, alias: &ModelAlias, target: &TargetId) -> Option<&ConcreteModel> {
        self.aliases
            .get(alias)
            .and_then(|targets| targets.value().get(target))
    }

    /// The alias `role` routes to.
    #[must_use]
    pub fn alias_of(&self, role: &Role) -> Option<&ModelAlias> {
        self.roles.get(role).map(Traced::value)
    }

    /// The concrete model for `role` on `target`: the role's alias, then the alias's model of the
    /// target. `None` when the role is not routed, its alias does not exist or has no model for
    /// the target.
    #[must_use]
    pub fn for_role(&self, role: &Role, target: &TargetId) -> Option<&ConcreteModel> {
        self.alias_of(role)
            .and_then(|alias| self.model(alias, target))
    }

    /// The concrete model of the session's default alias on `target`.
    #[must_use]
    pub fn for_default(&self, target: &TargetId) -> Option<&ConcreteModel> {
        self.default
            .as_ref()
            .and_then(|alias| self.model(alias.value(), target))
    }
}

/// The value of `name` when it is an array index: a canonical decimal below 2^32 - 1.
fn array_index(name: &str) -> Option<u32> {
    name.parse::<u32>()
        .ok()
        .filter(|index| *index != u32::MAX && index.to_string() == name)
}

/// The state of the fold for models.
#[derive(Debug, Default)]
pub(super) struct ModelState(MergedModels);

impl ModelState {
    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let (items, found) = ModelItems::read(context.layer());
        match items.into_file() {
            None => (self, found),
            Some((path, file)) => (
                Self(self.apply(&file, &Origin::new(context.layer(), path))),
                found,
            ),
        }
    }

    /// The models after `file` of `layer`.
    fn apply(self, file: &Models, origin: &Origin) -> MergedModels {
        let MergedModels {
            aliases,
            order,
            roles,
            default,
        } = self.0;
        let order = file.alias_order().iter().fold(order, |mut order, alias| {
            if !order.contains(alias) {
                order.push(alias.clone());
            }
            order
        });
        let aliases = file
            .aliases()
            .iter()
            .fold(aliases, |mut merged, (name, targets)| {
                let next = match merged.remove(name) {
                    Some(before) => {
                        let combined = before.value().clone().merge(targets.clone());
                        before.revised(combined, origin.clone(), Action::Extend)
                    }
                    None => Traced::new(targets.clone(), origin.clone(), Action::Define),
                };
                merged.insert(name.clone(), next);
                merged
            });
        let roles = file
            .roles()
            .iter()
            .fold(roles, |mut merged, (role, alias)| {
                let next = match merged.remove(role) {
                    Some(before) => before.revised(alias.clone(), origin.clone(), Action::Replace),
                    None => Traced::new(alias.clone(), origin.clone(), Action::Define),
                };
                merged.insert(role.clone(), next);
                merged
            });
        let default = match (default, file.default()) {
            (Some(before), Some(alias)) => {
                Some(before.revised(alias.clone(), origin.clone(), Action::Replace))
            }
            (None, Some(alias)) => Some(Traced::new(alias.clone(), origin.clone(), Action::Define)),
            (before, None) => before,
        };
        MergedModels {
            aliases,
            order,
            roles,
            default,
        }
    }

    /// The merged models, and an `unresolved` error for each role and the default that names an
    /// alias no layer defines.
    pub(super) fn finish(self) -> (MergedModels, Diagnostics) {
        let models = self.0;
        let known = |alias: &ModelAlias| models.aliases.contains_key(alias);
        let roles = models
            .roles
            .iter()
            .filter(|(_, alias)| !known(alias.value()))
            .map(|(role, alias)| {
                unresolved(
                    Message::new("role ")
                        .then(QuotedKey::from_untrusted(role.as_str()))
                        .then(" routes to unknown alias ")
                        .then(QuotedKey::from_untrusted(alias.value().as_str())),
                )
            });
        let default = models
            .default
            .iter()
            .filter(|alias| !known(alias.value()))
            .map(|alias| {
                unresolved(
                    Message::new("default ")
                        .then(QuotedKey::from_untrusted(alias.value().as_str()))
                        .then(" is not an alias"),
                )
            });
        let found = roles.chain(default).collect();
        (models, found)
    }
}

fn unresolved(message: Message) -> Diagnostic {
    Diagnostic::error(Code::Unresolved, None, message)
}

#[cfg(test)]
mod tests;
