//! The security items of the model (spec §8.0 **Model.**, §9.3).
//!
//! Model 1.0 has four kinds: each deny rule of the merged permissions, each floor field that is
//! set, the `default_mode` floor, and each locked hook that runs on the target. The coverage
//! report of an adapter (§8.0 **Coverage report.**) accounts for every one of them.

use crate::layer_name::LayerName;
use crate::model::hook::{HookEvent, OnFail};
use crate::model::mode::{FloorField, Mode};

/// The layers that set a security item: at least one, in stack order, each once. A deny rule or a
/// floor field is set by each layer that declares it, and the `default_mode` floor by each layer
/// whose floor is in force. At least one, because an item that no layer set is not an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingLayers {
    first: LayerName,
    later: Vec<LayerName>,
}

impl SettingLayers {
    /// The layers of an item that one layer set.
    #[must_use]
    pub const fn new(first: LayerName) -> Self {
        Self {
            first,
            later: Vec::new(),
        }
    }

    /// The same layers and one more, later in the stack. A layer that is already there is not
    /// added again.
    #[must_use]
    pub fn and(self, layer: LayerName) -> Self {
        if self.iter().any(|known| *known == layer) {
            self
        } else {
            Self {
                first: self.first,
                later: self.later.into_iter().chain([layer]).collect(),
            }
        }
    }

    /// The layers, in stack order.
    pub fn iter(&self) -> impl Iterator<Item = &LayerName> + '_ {
        std::iter::once(&self.first).chain(&self.later)
    }

    /// How many layers set the item. Never 0.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.later.len() + 1
    }

    /// Always false: there is at least one layer. It exists for the `len` convention.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

/// A deny rule of the merged permissions (§7.6).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DenyRule {
    /// The rule: `<tool>` or `<tool>:<pattern>`.
    pub rule: String,
    /// Each layer that declares it.
    pub set_by: SettingLayers,
}

impl DenyRule {
    /// A deny rule.
    #[must_use]
    pub fn new(rule: impl Into<String>, set_by: SettingLayers) -> Self {
        Self {
            rule: rule.into(),
            set_by,
        }
    }
}

/// A floor field that is set (§7.6): a field that no layer sets, or that is `false` from the first
/// layer that writes it, is not an item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FloorFieldItem {
    /// The field.
    pub field: FloorField,
    /// Each layer that sets it to `true`.
    pub set_by: SettingLayers,
}

impl FloorFieldItem {
    /// A floor field that is set.
    #[must_use]
    pub const fn new(field: FloorField, set_by: SettingLayers) -> Self {
        Self { field, set_by }
    }
}

/// The `default_mode` floor (§7.6), present when a layer sets `floor: true` and the floor is not
/// refused. The merged mode is never looser than the strictest floor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DefaultModeFloor {
    /// The strictest floor in force.
    pub mode: Mode,
    /// Each layer whose floor is in force.
    pub set_by: SettingLayers,
}

impl DefaultModeFloor {
    /// A floor.
    #[must_use]
    pub const fn new(mode: Mode, set_by: SettingLayers) -> Self {
        Self { mode, set_by }
    }
}

/// A locked hook (§7.5) that runs on the target: every one with a neutral event, and one with a
/// target's own event on that target only. Its `on_fail` is part of the item: a locked hook with
/// `on_fail: deny` on an event where the target cannot block is not expressed (§8.0 **Model.**).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LockedHook {
    /// The hook's id.
    pub id: String,
    /// The event.
    pub event: HookEvent,
    /// What happens when the hook fails.
    pub on_fail: OnFail,
    /// The one layer that defines it.
    pub defined_by: LayerName,
}

impl LockedHook {
    /// A locked hook.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        event: HookEvent,
        on_fail: OnFail,
        defined_by: LayerName,
    ) -> Self {
        Self {
            id: id.into(),
            event,
            on_fail,
            defined_by,
        }
    }
}

/// The identity of a security item: what a coverage report names (§8.0 **Output checks.**). Two
/// layers that declare the same deny rule give one item, with both layers in its `set_by`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecurityItemId {
    /// A deny rule, by its text.
    DenyRule(String),
    /// A floor field.
    FloorField(FloorField),
    /// The `default_mode` floor, of which there is at most one.
    DefaultModeFloor,
    /// A locked hook, by its id.
    LockedHook(String),
}

/// The security items of a model, of the four kinds of model 1.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SecurityItems {
    /// Each deny rule, once.
    pub deny_rules: Vec<DenyRule>,
    /// Each floor field that is set, once.
    pub floor_fields: Vec<FloorFieldItem>,
    /// The `default_mode` floor, when a layer sets one that is not refused.
    pub default_mode_floor: Option<DefaultModeFloor>,
    /// Each locked hook that runs on the target.
    pub locked_hooks: Vec<LockedHook>,
}

impl SecurityItems {
    /// The identity of every item, in the order deny rules, floor fields, the `default_mode`
    /// floor and locked hooks, each kind in the order it is held. The core compares a coverage
    /// report with this list.
    pub fn ids(&self) -> impl Iterator<Item = SecurityItemId> + '_ {
        let deny_rules = self
            .deny_rules
            .iter()
            .map(|item| SecurityItemId::DenyRule(item.rule.clone()));
        let floor_fields = self
            .floor_fields
            .iter()
            .map(|item| SecurityItemId::FloorField(item.field));
        let default_mode = self
            .default_mode_floor
            .iter()
            .map(|_| SecurityItemId::DefaultModeFloor);
        let locked_hooks = self
            .locked_hooks
            .iter()
            .map(|item| SecurityItemId::LockedHook(item.id.clone()));
        deny_rules
            .chain(floor_fields)
            .chain(default_mode)
            .chain(locked_hooks)
    }
}

#[cfg(test)]
mod tests {
    use crate::layer_name::LayerName;
    use crate::model::hook::{HookEvent, NeutralEvent, OnFail};
    use crate::model::mode::{FloorField, Mode};

    use super::{
        DefaultModeFloor, DenyRule, FloorFieldItem, LockedHook, SecurityItemId, SecurityItems,
        SettingLayers,
    };

    fn layer(name: &str) -> LayerName {
        LayerName::try_from(name).expect("a layer name")
    }

    #[test]
    fn the_layers_of_an_item_are_in_stack_order_and_each_once() {
        let layers = SettingLayers::new(layer("personal"))
            .and(layer("client"))
            .and(layer("personal"))
            .and(layer("repo"));
        let names: Vec<&str> = layers.iter().map(LayerName::as_str).collect();
        assert_eq!(names, ["personal", "client", "repo"]);
        assert_eq!(layers.len(), 3);
        assert!(!layers.is_empty());
    }

    #[test]
    fn every_kind_of_item_has_an_identity() {
        let items = SecurityItems {
            deny_rules: vec![DenyRule::new(
                "read:.env*",
                SettingLayers::new(layer("personal")),
            )],
            floor_fields: vec![FloorFieldItem::new(
                FloorField::BlockWritesOutside,
                SettingLayers::new(layer("client")),
            )],
            default_mode_floor: Some(DefaultModeFloor::new(
                Mode::Default,
                SettingLayers::new(layer("personal")),
            )),
            locked_hooks: vec![LockedHook::new(
                "block-secrets",
                HookEvent::Neutral(NeutralEvent::Stop),
                OnFail::Deny,
                layer("personal"),
            )],
        };
        let ids: Vec<SecurityItemId> = items.ids().collect();
        assert_eq!(
            ids,
            [
                SecurityItemId::DenyRule("read:.env*".to_owned()),
                SecurityItemId::FloorField(FloorField::BlockWritesOutside),
                SecurityItemId::DefaultModeFloor,
                SecurityItemId::LockedHook("block-secrets".to_owned()),
            ]
        );
    }

    #[test]
    fn a_model_without_items_has_no_identity() {
        assert_eq!(SecurityItems::default().ids().count(), 0);
    }
}
