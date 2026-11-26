//! The merged permissions (spec §7.6).

use alz_adapter::{FloorField, Mode, Permissions};

use crate::item::{FloorFlag, PermissionMode, PermissionRule};
use crate::merge::{Merged, Traced};

/// A mode of the neutral vocabulary.
pub(crate) const fn mode(mode: PermissionMode) -> Mode {
    match mode {
        PermissionMode::BypassPermissions => Mode::BypassPermissions,
        PermissionMode::Auto => Mode::Auto,
        PermissionMode::AcceptEdits => Mode::AcceptEdits,
        PermissionMode::Default => Mode::Default,
        PermissionMode::DontAsk => Mode::DontAsk,
        PermissionMode::Plan => Mode::Plan,
    }
}

/// The mode of the neutral vocabulary of a mode of the contract: the inverse of [`mode`].
pub(crate) const fn neutral_mode(mode: Mode) -> PermissionMode {
    match mode {
        Mode::BypassPermissions => PermissionMode::BypassPermissions,
        Mode::Auto => PermissionMode::Auto,
        Mode::AcceptEdits => PermissionMode::AcceptEdits,
        Mode::Default => PermissionMode::Default,
        Mode::DontAsk => PermissionMode::DontAsk,
        Mode::Plan => PermissionMode::Plan,
    }
}

/// A floor flag as the floor field of the contract.
pub(super) const fn floor_field(flag: FloorFlag) -> FloorField {
    match flag {
        FloorFlag::DisableBypassMode => FloorField::DisableBypassMode,
        FloorFlag::BlockReadsOutside => FloorField::BlockReadsOutside,
        FloorFlag::BlockWritesOutside => FloorField::BlockWritesOutside,
    }
}

/// The rules as written, `<tool>` or `<tool>:<pattern>`.
fn rules(list: &[Traced<PermissionRule>]) -> Vec<String> {
    list.iter().map(|rule| rule.value().to_string()).collect()
}

/// The permissions: the lists, the merged `default_mode`, and the floor fields that are set.
pub(super) fn of(merged: &Merged) -> Permissions {
    let permissions = merged.permissions();
    let mut made = Permissions::default();
    made.allow = rules(permissions.allow());
    made.ask = rules(permissions.ask());
    made.deny = rules(permissions.deny());
    made.default_mode = permissions
        .default_mode()
        .map(|merged_mode| mode(*merged_mode.value()));
    made.floor_fields = FloorFlag::ALL
        .into_iter()
        .filter(|flag| permissions.is_set(*flag))
        .map(floor_field)
        .collect();
    made
}
