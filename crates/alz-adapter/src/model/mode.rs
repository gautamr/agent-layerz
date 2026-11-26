//! The permission modes and the floor fields of spec §7.6.

use std::fmt;

/// A permission mode. The order is the order of §7.6, from the loosest to the strictest, so a
/// mode is "looser than" another when it is less, and a floor is a `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    /// `bypass_permissions`, the loosest.
    BypassPermissions,
    /// `auto`.
    Auto,
    /// `accept_edits`.
    AcceptEdits,
    /// `default`.
    Default,
    /// `dont_ask`.
    DontAsk,
    /// `plan`, the strictest.
    Plan,
}

impl Mode {
    /// The mode as the spec and `permissions.yaml` write it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BypassPermissions => "bypass_permissions",
            Self::Auto => "auto",
            Self::AcceptEdits => "accept_edits",
            Self::Default => "default",
            Self::DontAsk => "dont_ask",
            Self::Plan => "plan",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One of the three boolean floor fields of §7.6. Once a layer sets one to `true` it stays true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloorField {
    /// `disable_bypass_mode`: switches off the target's bypass-permissions mode.
    DisableBypassMode,
    /// `block_reads_outside`: blocks reads outside the working directories.
    BlockReadsOutside,
    /// `block_writes_outside`: blocks writes outside the working directories.
    BlockWritesOutside,
}

impl FloorField {
    /// The field as `permissions.yaml` writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DisableBypassMode => "disable_bypass_mode",
            Self::BlockReadsOutside => "block_reads_outside",
            Self::BlockWritesOutside => "block_writes_outside",
        }
    }
}

impl fmt::Display for FloorField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{FloorField, Mode};

    #[test]
    fn modes_run_from_the_loosest_to_the_strictest() {
        let modes = [
            Mode::BypassPermissions,
            Mode::Auto,
            Mode::AcceptEdits,
            Mode::Default,
            Mode::DontAsk,
            Mode::Plan,
        ];
        assert!(modes.windows(2).all(|pair| pair[0] < pair[1]));
        let words: Vec<&str> = modes.into_iter().map(Mode::as_str).collect();
        assert_eq!(
            words,
            [
                "bypass_permissions",
                "auto",
                "accept_edits",
                "default",
                "dont_ask",
                "plan"
            ]
        );
    }

    #[test]
    fn floor_fields_print_as_permissions_yaml_writes_them() {
        assert_eq!(
            FloorField::DisableBypassMode.to_string(),
            "disable_bypass_mode"
        );
        assert_eq!(
            FloorField::BlockReadsOutside.to_string(),
            "block_reads_outside"
        );
        assert_eq!(
            FloorField::BlockWritesOutside.to_string(),
            "block_writes_outside"
        );
    }
}
