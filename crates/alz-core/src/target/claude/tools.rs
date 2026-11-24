//! The native names of Claude Code for the neutral vocabulary (spec §8.1, target-facts §1).

use crate::item::{Memory, NeutralTool, PermissionMode, ToolGrant, ToolName};

/// The native name of a tool: the neutral tools by the table of §8.1, an `mcp__` tool as written.
pub(super) fn tool(name: &ToolName) -> String {
    match name {
        ToolName::Neutral(neutral) => match neutral {
            NeutralTool::Read => "Read",
            NeutralTool::Write => "Write",
            NeutralTool::Edit => "Edit",
            NeutralTool::Grep => "Grep",
            NeutralTool::Glob => "Glob",
            NeutralTool::Bash => "Bash",
            NeutralTool::WebFetch => "WebFetch",
            NeutralTool::WebSearch => "WebSearch",
            NeutralTool::Agent => "Agent",
            NeutralTool::NotebookEdit => "NotebookEdit",
            NeutralTool::TodoWrite => "TodoWrite",
            NeutralTool::Skill => "Skill",
        }
        .to_owned(),
        ToolName::Mcp(_) => name.as_str().to_owned(),
    }
}

/// The Claude rule of a permission or hook entry: `<tool>:<pattern>` is `<Tool>(<pattern>)` and a
/// bare `<tool>` is `<Tool>` (target-facts §7b).
pub(super) fn rule(grant: &ToolGrant) -> String {
    let native = tool(grant.tool());
    grant.scope().map_or(native.clone(), |scope| {
        format!("{native}({})", scope.as_str())
    })
}

/// The native `permissionMode` of a neutral mode.
pub(super) const fn permission_mode(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::BypassPermissions => "bypassPermissions",
        PermissionMode::Auto => "auto",
        PermissionMode::AcceptEdits => "acceptEdits",
        PermissionMode::Default => "default",
        PermissionMode::DontAsk => "dontAsk",
        PermissionMode::Plan => "plan",
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
    .find(|neutral| tool(&ToolName::Neutral(*neutral)) == native)
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
        .find(|mode| permission_mode(*mode) == native)
}
