//! `permissions` of `.claude/settings.json` (spec §7.6, §8.1, target-facts §13.4): the rules as
//! Claude writes them, the default mode, the additional directories and the floor fields.
//!
//! This is the only place that writes the three floor keys, and it writes each exactly when a
//! layer set the field to `true`.

use alz_adapter::{Context, FloorField, JsonValue, Model};

use super::document::{map, text, texts};
use super::tools;

/// The Claude key of a floor field, and the value that switches it on. Claude takes
/// `disableBypassPermissionsMode` as the string `"disable"`, and the two `block` keys as booleans
/// (target-facts §13.4).
const fn floor_entry(field: FloorField) -> (&'static str, FloorValue) {
    match field {
        FloorField::DisableBypassMode => ("disableBypassPermissionsMode", FloorValue::Disable),
        FloorField::BlockReadsOutside => ("blockReadsOutsideWorkingDirectories", FloorValue::Set),
        FloorField::BlockWritesOutside => ("blockWritesOutsideWorkingDirectories", FloorValue::Set),
    }
}

/// How Claude takes the value of a floor key.
#[derive(Debug, Clone, Copy)]
enum FloorValue {
    /// The string `"disable"`.
    Disable,
    /// The boolean `true`.
    Set,
}

impl FloorValue {
    fn value(self) -> JsonValue {
        match self {
            Self::Disable => text("disable"),
            Self::Set => JsonValue::Bool(true),
        }
    }
}

/// The `permissions` mapping, `None` when nothing is set.
pub(super) fn render(model: &Model, context: &Context) -> Option<JsonValue> {
    let permissions = &model.permissions;
    let rules = |list: &[String]| -> Option<JsonValue> {
        (!list.is_empty()).then(|| {
            JsonValue::Array(
                list.iter()
                    .map(|rule| JsonValue::Text(tools::rule(rule)))
                    .collect(),
            )
        })
    };
    let directories = (!context.additional_directories.is_empty())
        .then(|| texts(context.additional_directories.iter().map(String::as_str)));
    let floors = permissions.floor_fields.iter().map(|field| {
        let (key, value) = floor_entry(*field);
        (key, value.value())
    });
    let entries: Vec<(&str, JsonValue)> = [
        rules(&permissions.allow).map(|value| ("allow", value)),
        rules(&permissions.ask).map(|value| ("ask", value)),
        rules(&permissions.deny).map(|value| ("deny", value)),
        permissions
            .default_mode
            .map(|mode| ("defaultMode", text(tools::permission_mode(mode)))),
        directories.map(|value| ("additionalDirectories", value)),
    ]
    .into_iter()
    .flatten()
    .chain(floors)
    .collect();
    (!entries.is_empty()).then(|| map(entries))
}
