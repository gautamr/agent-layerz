//! The neutral names of the native names Claude Code uses (spec §8.1, target-facts §1 and §8.5),
//! the other way round from the adapter's tables, which this module reuses.

use alz_target_claude::{neutral_permission_mode, neutral_tool};

use crate::adapter_model::neutral_mode;
use crate::item::{NeutralEvent, PermissionMode};

/// What a hook command writes for the project folder (spec §7.5); Claude's own variable is
/// replaced by it.
pub(super) const PROJECT_DIR_VAR: &str = "${PROJECT_DIR}";

/// The folder of hand-written hook scripts.
pub(super) const HOOKS_DIR: &str = ".claude/hooks";

/// Claude's variable for the project folder, in the two forms a command may write it.
const CLAUDE_PROJECT_DIR: [&str; 2] = ["${CLAUDE_PROJECT_DIR}", "$CLAUDE_PROJECT_DIR"];

/// A command with Claude's project folder variable written as the neutral one.
pub(super) fn neutral_command(command: &str) -> String {
    CLAUDE_PROJECT_DIR
        .iter()
        .fold(command.to_owned(), |text, form| {
            text.replace(form, PROJECT_DIR_VAR)
        })
}

/// The neutral tool of a native tool name; an `mcp__` tool keeps its name.
pub(super) fn tool(native: &str) -> Option<String> {
    if native.starts_with("mcp__") {
        Some(native.to_owned())
    } else {
        neutral_tool(native).map(str::to_owned)
    }
}

/// The neutral rule of a native `Tool(pattern)` or `Tool`: `tool:pattern` or `tool`. `None` when
/// the tool is unknown or the rule is not that shape.
pub(super) fn rule(native: &str) -> Option<String> {
    let rule = native.trim();
    let name_end = rule
        .char_indices()
        .find(|(at, c)| {
            !(if *at == 0 {
                c.is_ascii_alphabetic() || *c == '_'
            } else {
                c.is_ascii_alphanumeric() || *c == '_' || *c == '-'
            })
        })
        .map_or(rule.len(), |(at, _)| at);
    let (name, rest) = rule.split_at_checked(name_end)?;
    let tool = tool(name)?;
    if rest.is_empty() {
        Some(tool)
    } else {
        rest.strip_prefix('(')
            .and_then(|inner| inner.strip_suffix(')'))
            .map(|pattern| format!("{tool}:{pattern}"))
    }
}

/// The neutral event of a native event name.
pub(super) fn event(native: &str) -> Option<NeutralEvent> {
    match native {
        "SessionStart" => Some(NeutralEvent::SessionStart),
        "PreToolUse" => Some(NeutralEvent::PreTool),
        "PostToolUse" => Some(NeutralEvent::PostTool),
        "Stop" => Some(NeutralEvent::Stop),
        "SubagentStop" => Some(NeutralEvent::SubagentStop),
        "UserPromptSubmit" => Some(NeutralEvent::PromptSubmit),
        _ => None,
    }
}

/// Whether the event carries a tool, so a `match` of tools applies to it.
pub(super) const fn is_tool_event(event: NeutralEvent) -> bool {
    matches!(event, NeutralEvent::PreTool | NeutralEvent::PostTool)
}

/// The neutral permission mode of a native one.
pub(super) fn mode(native: &str) -> Option<PermissionMode> {
    neutral_permission_mode(native).map(neutral_mode)
}
