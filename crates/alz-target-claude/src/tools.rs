//! The native names of Claude Code for the neutral vocabulary (spec §8.1, target-facts §1).
//!
//! The adapter reads the names of the model, which are text: a neutral tool is `read`, an `mcp__`
//! tool keeps its name. The two reverse functions at the end serve adoption, which reads Claude's
//! files into the neutral vocabulary of the core; they answer in the words of the contract (the
//! neutral tool name as text, a [`Mode`]), and the core turns those into its own types.

use alz_adapter::{Memory, Mode};

/// The neutral tools and the native name of each (spec §8.1).
const TOOLS: [(&str, &str); 12] = [
    ("read", "Read"),
    ("write", "Write"),
    ("edit", "Edit"),
    ("grep", "Grep"),
    ("glob", "Glob"),
    ("bash", "Bash"),
    ("web_fetch", "WebFetch"),
    ("web_search", "WebSearch"),
    ("agent", "Agent"),
    ("notebook_edit", "NotebookEdit"),
    ("todo_write", "TodoWrite"),
    ("skill", "Skill"),
];

/// The neutral permission modes, in the order of the vocabulary.
const MODES: [Mode; 6] = [
    Mode::BypassPermissions,
    Mode::Auto,
    Mode::AcceptEdits,
    Mode::Default,
    Mode::DontAsk,
    Mode::Plan,
];

/// The native name of a tool: the neutral tools by the table of §8.1, an `mcp__` tool as written.
pub(super) fn tool(name: &str) -> String {
    TOOLS
        .iter()
        .find(|(neutral, _)| *neutral == name)
        .map_or(name, |(_, native)| native)
        .to_owned()
}

/// The Claude rule of a permission or hook entry: `<tool>:<pattern>` is `<Tool>(<pattern>)` and a
/// bare `<tool>` is `<Tool>` (target-facts §7b). The tool is the text before the first `:`.
pub(super) fn rule(entry: &str) -> String {
    entry.split_once(':').map_or_else(
        || tool(entry),
        |(name, scope)| format!("{}({scope})", tool(name)),
    )
}

/// The native `permissionMode` of a neutral mode.
pub(super) const fn permission_mode(mode: Mode) -> &'static str {
    match mode {
        Mode::BypassPermissions => "bypassPermissions",
        Mode::Auto => "auto",
        Mode::AcceptEdits => "acceptEdits",
        Mode::Default => "default",
        Mode::DontAsk => "dontAsk",
        Mode::Plan => "plan",
    }
}

/// The native `memory` of a neutral memory: Claude has no `none`, so the field is left out.
pub(super) const fn memory(memory: Memory) -> Option<&'static str> {
    match memory {
        Memory::None => None,
        Memory::Project => Some("project"),
        Memory::User => Some("user"),
        Memory::Local => Some("local"),
    }
}

/// Whether `text` is a value Claude accepts for `model` (target-facts §8.9): `sonnet`, `opus`,
/// `haiku`, `fable`, `inherit`, or `claude-` and lower-case letters, digits, `.` and `-`.
#[must_use]
pub fn is_model_value(text: &str) -> bool {
    ["sonnet", "opus", "haiku", "fable", "inherit"].contains(&text)
        || text.strip_prefix("claude-").is_some_and(|rest| {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
        })
}

/// The neutral tool that Claude calls `native`, by the adapter's table of tools, as the text the
/// neutral vocabulary writes; `None` for a name that table does not make (an `mcp__` tool has no
/// neutral form here, it keeps its name).
#[must_use]
pub fn neutral_tool(native: &str) -> Option<&'static str> {
    TOOLS
        .iter()
        .find(|(_, name)| *name == native)
        .map(|(neutral, _)| *neutral)
}

/// The neutral mode that Claude calls `native`, by the adapter's table of permission modes, and
/// `manual` as `default`, the name an older Claude used.
#[must_use]
pub fn neutral_permission_mode(native: &str) -> Option<Mode> {
    if native == "manual" {
        return Some(Mode::Default);
    }
    MODES
        .into_iter()
        .find(|mode| permission_mode(*mode) == native)
}
