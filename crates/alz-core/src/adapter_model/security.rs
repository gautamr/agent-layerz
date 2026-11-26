//! The four kinds of security item of model 1.0 (spec §8.0 **Model.**, §9.3), each with the layers
//! that set it.

use alz_adapter::{
    DefaultModeFloor, DenyRule, FloorFieldItem, LockedHook, SecurityItems, SettingLayers, TargetId,
};

use crate::item::{FloorFlag, Lock};
use crate::layer::LayerName;
use crate::merge::{Merged, Traced};

use super::hooks::{defined_by, event, on_fail, runs_on};
use super::permissions::{floor_field, mode};

/// The layers that set an item, in stack order: `declared`, or the one layer the merge
/// remembers when it knows no more. None only for an item that no layer set, which is no item.
fn setting_layers(declared: &[LayerName], remembered: Option<&LayerName>) -> Option<SettingLayers> {
    let mut layers = declared
        .iter()
        .chain(remembered.filter(|_| declared.is_empty()));
    layers.next().map(|first| {
        layers.fold(SettingLayers::new(first.to_contract()), |all, layer| {
            all.and(layer.to_contract())
        })
    })
}

/// The first layer that contributed to a value.
fn first_layer<T>(traced: &Traced<T>) -> Option<&LayerName> {
    traced
        .provenance()
        .contributions()
        .first()
        .map(crate::merge::Contribution::layer)
}

/// The security items of the merged stack for `target`.
pub(super) fn of(merged: &Merged, target: &TargetId) -> SecurityItems {
    let permissions = merged.permissions();
    let mut items = SecurityItems::default();
    items.deny_rules = permissions
        .deny()
        .iter()
        .filter_map(|rule| {
            setting_layers(
                permissions.deny_declared_by(rule.value()),
                first_layer(rule),
            )
            .map(|layers| DenyRule::new(rule.value().to_string(), layers))
        })
        .collect();
    items.floor_fields = FloorFlag::ALL
        .into_iter()
        .filter_map(|flag| {
            setting_layers(
                permissions.flag_set_by(flag),
                permissions.flag(flag).and_then(first_layer),
            )
            .map(|layers| FloorFieldItem::new(floor_field(flag), layers))
        })
        .collect();
    items.default_mode_floor = permissions.floor().and_then(|strictest| {
        setting_layers(permissions.floor_set_by(), None)
            .map(|layers| DefaultModeFloor::new(mode(strictest), layers))
    });
    items.locked_hooks = merged
        .hooks()
        .iter()
        .filter(|traced| {
            matches!(traced.value().lock(), Lock::Locked) && runs_on(traced.value(), target)
        })
        .filter_map(|traced| {
            let hook = traced.value();
            defined_by(traced)
                .map(|layer| LockedHook::new(hook.id().as_str(), event(hook), on_fail(hook), layer))
        })
        .collect();
    items
}
