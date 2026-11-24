//! `permissions` of `.claude/settings.json` (spec §7.6, §8.1, target-facts §13.4): the rules as
//! Claude writes them, the default mode, the additional directories and the floor fields.
//!
//! This is the only place that writes the three floor keys, and it writes each exactly when a
//! layer set the field to `true`.

use crate::item::{FloorFlag, PermissionRule};
use crate::merge::{Merged, Traced, Tree};
use crate::stack::Workspace;

use super::document::{map, text, texts};
use super::tools;

/// The Claude key of a floor field, and the value that switches it on. Claude takes
/// `disableBypassPermissionsMode` as the string `"disable"`, and the two `block` keys as booleans
/// (target-facts §13.4).
const fn floor_entry(flag: FloorFlag) -> (&'static str, FloorValue) {
    match flag {
        FloorFlag::DisableBypassMode => ("disableBypassPermissionsMode", FloorValue::Disable),
        FloorFlag::BlockReadsOutside => ("blockReadsOutsideWorkingDirectories", FloorValue::Set),
        FloorFlag::BlockWritesOutside => ("blockWritesOutsideWorkingDirectories", FloorValue::Set),
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
    fn tree(self) -> Tree {
        match self {
            Self::Disable => text("disable"),
            Self::Set => Tree::Bool(true),
        }
    }
}

/// The `permissions` mapping, `None` when nothing is set.
pub(super) fn render(merged: &Merged, workspace: &Workspace) -> Option<Tree> {
    let permissions = merged.permissions();
    let rules = |list: &[Traced<PermissionRule>]| -> Option<Tree> {
        (!list.is_empty()).then(|| {
            Tree::List(
                list.iter()
                    .map(|rule| Tree::Text(tools::rule(rule.value().grant())))
                    .collect(),
            )
        })
    };
    let directories = (!workspace.add_dirs().is_empty()).then(|| {
        texts(
            workspace
                .add_dirs()
                .iter()
                .map(crate::stack::WorkspaceDir::as_str),
        )
    });
    let floors = FloorFlag::ALL
        .into_iter()
        .filter(|flag| permissions.is_set(*flag))
        .map(|flag| {
            let (key, value) = floor_entry(flag);
            (key, value.tree())
        });
    let entries: Vec<(&str, Tree)> = [
        rules(permissions.allow()).map(|tree| ("allow", tree)),
        rules(permissions.ask()).map(|tree| ("ask", tree)),
        rules(permissions.deny()).map(|tree| ("deny", tree)),
        permissions
            .default_mode()
            .map(|mode| ("defaultMode", text(tools::permission_mode(*mode.value())))),
        directories.map(|tree| ("additionalDirectories", tree)),
    ]
    .into_iter()
    .flatten()
    .chain(floors)
    .collect();
    (!entries.is_empty()).then(|| map(entries))
}
