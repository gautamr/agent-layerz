//! The target's own native settings (spec §7.9).

use alz_adapter::{JsonObject, JsonValue, TargetId};

use crate::merge::Merged;

use super::json::of_tree;

/// The merged `settings.yaml` section of `target`: empty when no layer wrote one.
pub(super) fn of(merged: &Merged, target: &TargetId) -> JsonObject {
    merged
        .settings()
        .get(target)
        .map(|traced| of_tree(&traced.value().to_tree()))
        .and_then(|value| match value {
            JsonValue::Object(object) => Some(object),
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Int(_)
            | JsonValue::Float(_)
            | JsonValue::Text(_)
            | JsonValue::Array(_) => None,
        })
        .unwrap_or_default()
}
