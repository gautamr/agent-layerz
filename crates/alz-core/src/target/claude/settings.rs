//! `.claude/settings.json` (spec §8.1, target-facts §13): the claude settings of `settings.yaml`
//! with the generated keys merged over them. The file is made from that settings map and the
//! first-class items (permissions, hooks, the default model), and from nothing else.

use std::collections::BTreeMap;

use alz_adapter::{Context, JsonValue, Model};

use crate::diagnostic::Diagnostics;

use super::document::{deep_merge, file, map, object, text};
use super::{Output, hooks, output_path, permissions};

/// The settings path.
const SETTINGS_FILE: &str = ".claude/settings.json";

/// `.claude/settings.json` when it would not be empty, and the hook scripts it runs.
pub(super) fn render(model: &Model, context: &Context) -> Output {
    let hooks = hooks::render(model);
    let hook_groups = (!hooks.events.is_empty()).then(|| {
        object(
            hooks
                .events
                .into_iter()
                .map(|(event, groups)| (event, JsonValue::Array(groups))),
        )
    });
    let default_model = model.models.default.as_deref().map(text);
    let generated = map([
        permissions::render(model, context).map(|value| ("permissions", value)),
        hook_groups.map(|value| ("hooks", value)),
        default_model.map(|value| ("model", value)),
    ]
    .into_iter()
    .flatten());
    let settings = deep_merge(
        object(
            model
                .settings
                .entries()
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect::<BTreeMap<_, _>>(),
        ),
        generated,
    );
    let settings_file = if matches!(&settings, JsonValue::Object(entries) if entries.is_empty()) {
        Output::default()
    } else {
        Output::one(
            output_path(SETTINGS_FILE),
            file(&settings),
            Diagnostics::new(),
        )
    };
    hooks.output.then(settings_file)
}
