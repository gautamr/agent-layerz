//! The native names of Claude Code for the neutral vocabulary (spec §8.1, target-facts §1).
//!
//! The adapter reads the names of the model, which are text: a neutral tool is `read`, an `mcp__`
//! tool keeps its name. The two reverse functions at the end serve adoption, which reads Claude's
//! files into the neutral vocabulary of the core.

use alz_adapter::{Memory, Mode};

use crate::item::{NeutralTool, PermissionMode};

/// The native name of a tool: the neutral tools by the table of §8.1, an `mcp__` tool as written.
pub(super) fn tool(name: &str) -> String {
    match name {
        "read" => "Read",
        "write" => "Write",
        "edit" => "Edit",
        "grep" => "Grep",
        "glob" => "Glob",
        "bash" => "Bash",
        "web_fetch" => "WebFetch",
        "web_search" => "WebSearch",
        "agent" => "Agent",
        "notebook_edit" => "NotebookEdit",
        "todo_write" => "TodoWrite",
        "skill" => "Skill",
        other => other,
    }
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

/// The neutral tool that Claude calls `native`, by the adapter's table of tools; `None` for a name that
/// table does not make (an `mcp__` tool has no neutral form here, it keeps its name).
#[must_use]
pub fn neutral_tool(native: &str) -> Option<NeutralTool> {
    use NeutralTool::{
        Agent, Bash, Edit, Glob, Grep, NotebookEdit, Read, Skill, TodoWrite, WebFetch, WebSearch,
        Write,
    };
    [
        Read,
        Write,
        Edit,
        Grep,
        Glob,
        Bash,
        WebFetch,
        WebSearch,
        Agent,
        NotebookEdit,
        TodoWrite,
        Skill,
    ]
    .into_iter()
    .find(|neutral| tool(neutral.as_str()) == native)
}

/// The neutral mode that Claude calls `native`, by the adapter's table of permission modes, and `manual`
/// as `default`, the name an older Claude used.
#[must_use]
pub fn neutral_permission_mode(native: &str) -> Option<PermissionMode> {
    use PermissionMode::{AcceptEdits, Auto, BypassPermissions, Default, DontAsk, Plan};
    if native == "manual" {
        return Some(Default);
    }
    [BypassPermissions, Auto, AcceptEdits, Default, DontAsk, Plan]
        .into_iter()
        .find(|mode| permission_mode(crate::adapter_model::contract_mode(*mode)) == native)
}
