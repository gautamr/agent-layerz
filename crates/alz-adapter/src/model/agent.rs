//! Agents (spec §7.2) for one target.

use crate::json::JsonValue;
use crate::model::mode::Mode;
use crate::source::Source;

/// A tool of an agent: a neutral tool or an `mcp__` tool, with the scope that §7.2 allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTool {
    /// The tool: `read`, `write`, `edit`, `grep`, `glob`, `bash`, `web_fetch`, `web_search`,
    /// `agent`, `notebook_edit`, `todo_write`, `skill`, or a name that starts with `mcp__`.
    pub name: String,
    /// The scope, when the tool has one, as `write:docs/**` has `docs/**`. A target that cannot
    /// keep a scope drops it with a warning (Claude: `tool-scope`).
    pub scope: Option<String>,
}

/// The `isolation` of an agent (§7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isolation {
    /// `none`.
    None,
    /// `worktree`.
    Worktree,
}

/// The `memory` of an agent (§7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Memory {
    /// `none`.
    None,
    /// `project`.
    Project,
    /// `user`.
    User,
    /// `local`.
    Local,
}

/// An agent for one target, after its overlays are applied (§7.2).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Agent {
    /// The name, equal to the file's stem.
    pub name: String,
    /// The description.
    pub description: String,
    /// The tools; none when the agent lists none, which is not the same as an empty list.
    pub tools: Option<Vec<AgentTool>>,
    /// The model, resolved through the routing for this target (§7.8); none when the alias has
    /// no model for the target.
    pub model: Option<String>,
    /// The `permission_mode`, when set.
    pub permission_mode: Option<Mode>,
    /// The skills the agent preloads.
    pub skills: Vec<String>,
    /// The isolation.
    pub isolation: Isolation,
    /// The memory.
    pub memory: Memory,
    /// This target's native fields, in the order of §7.2 (**Key order of native fields**).
    pub native: Vec<(String, JsonValue)>,
    /// The body, with the sections that overlays extend appended.
    pub body: String,
    /// The definitions the agent came from, base first, then each overlay, for the header.
    pub sources: Vec<Source>,
}

impl Agent {
    /// An agent with a name, a description and a body, and nothing else.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tools: None,
            model: None,
            permission_mode: None,
            skills: Vec::new(),
            isolation: Isolation::None,
            memory: Memory::None,
            native: Vec::new(),
            body: body.into(),
            sources: Vec::new(),
        }
    }
}
