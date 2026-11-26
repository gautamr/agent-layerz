//! `.claude/settings.json` (spec §8.1, target-facts §13): the claude settings of `settings.yaml`
//! with the generated keys merged over them. The file is made from that settings map and the
//! first-class items (permissions, hooks, the default model), and from nothing else.

use std::collections::BTreeMap;

use crate::diagnostic::Diagnostics;
use crate::merge::{Merged, Tree};
use crate::stack::Workspace;

use super::document::{file, map, text};
use super::{Output, hooks, output_path, permissions};

/// The settings path.
const SETTINGS_FILE: &str = ".claude/settings.json";

/// `.claude/settings.json` when it would not be empty, and the hook scripts it runs.
pub(super) fn render(merged: &Merged, workspace: &Workspace) -> Output {
    let hooks = hooks::render(merged);
    let hook_groups = (!hooks.events.is_empty()).then(|| {
        Tree::Map(
            hooks
                .events
                .into_iter()
                .map(|(event, groups)| (event, Tree::List(groups)))
                .collect(),
        )
    });
    let model = merged
        .models()
        .for_default(crate::target::claude())
        .map(|model| text(model.as_str()));
    let generated = map([
        permissions::render(merged, workspace).map(|tree| ("permissions", tree)),
        hook_groups.map(|tree| ("hooks", tree)),
        model.map(|tree| ("model", tree)),
    ]
    .into_iter()
    .flatten());
    let settings = merged
        .settings()
        .get(crate::target::claude())
        .map_or_else(
            || Tree::Map(BTreeMap::new()),
            |traced| traced.value().to_tree(),
        )
        .deep_merge(generated);
    let settings_file = if matches!(&settings, Tree::Map(entries) if entries.is_empty()) {
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
