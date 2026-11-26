//! The text that layers add for one target only (spec §7.1): the Claude-only block of `CLAUDE.md`.

use alz_adapter::{TargetId, TargetInstruction};

use crate::merge::Merged;

use super::sources::of_origin;

/// The target's own instruction text in stack order: `targets/claude/CLAUDE.extra.md` of each
/// layer for Claude, and nothing for any other target.
pub(super) fn of(merged: &Merged, target: &TargetId) -> Vec<TargetInstruction> {
    if target == crate::target::claude() {
        merged
            .claude_extras()
            .iter()
            .map(|extra| TargetInstruction::new(extra.body(), of_origin(extra.origin())))
            .collect()
    } else {
        Vec::new()
    }
}
