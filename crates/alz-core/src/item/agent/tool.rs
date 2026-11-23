//! The tool entries of an agent's `tools` (spec §7.2): `read`, `write:docs/**`,
//! `bash:mise run *`. An entry is read once into a [`ToolGrant`], the tool it names and the scope
//! it is limited to, so the narrowing rule can compare entries without looking at text again.

use std::fmt;

use super::vocabulary::AgentRefusal;

/// A tool of the neutral vocabulary (spec §7.2). The set is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NeutralTool {
    /// `read`.
    Read,
    /// `write`.
    Write,
    /// `edit`.
    Edit,
    /// `grep`.
    Grep,
    /// `glob`.
    Glob,
    /// `bash`.
    Bash,
    /// `web_fetch`.
    WebFetch,
    /// `web_search`.
    WebSearch,
    /// `agent`.
    Agent,
    /// `notebook_edit`.
    NotebookEdit,
    /// `todo_write`.
    TodoWrite,
    /// `skill`.
    Skill,
}

impl NeutralTool {
    /// The word the neutral vocabulary writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Edit => "edit",
            Self::Grep => "grep",
            Self::Glob => "glob",
            Self::Bash => "bash",
            Self::WebFetch => "web_fetch",
            Self::WebSearch => "web_search",
            Self::Agent => "agent",
            Self::NotebookEdit => "notebook_edit",
            Self::TodoWrite => "todo_write",
            Self::Skill => "skill",
        }
    }

    fn named(name: &str) -> Option<Self> {
        match name {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "edit" => Some(Self::Edit),
            "grep" => Some(Self::Grep),
            "glob" => Some(Self::Glob),
            "bash" => Some(Self::Bash),
            "web_fetch" => Some(Self::WebFetch),
            "web_search" => Some(Self::WebSearch),
            "agent" => Some(Self::Agent),
            "notebook_edit" => Some(Self::NotebookEdit),
            "todo_write" => Some(Self::TodoWrite),
            "skill" => Some(Self::Skill),
            _ => None,
        }
    }

    /// What a scope of this tool is.
    const fn scope_kind(self) -> ScopeKind {
        match self {
            Self::Read
            | Self::Write
            | Self::Edit
            | Self::Grep
            | Self::Glob
            | Self::NotebookEdit => ScopeKind::Path,
            Self::Bash => ScopeKind::Command,
            Self::WebFetch | Self::WebSearch | Self::Agent | Self::TodoWrite | Self::Skill => {
                ScopeKind::Other
            }
        }
    }
}

/// The name of a tool of an MCP server, `mcp__<server>__<tool>`: the neutral vocabulary keeps it
/// as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct McpToolName(String);

impl McpToolName {
    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The tool an entry names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolName {
    /// A tool of the neutral vocabulary.
    Neutral(NeutralTool),
    /// A tool of an MCP server.
    Mcp(McpToolName),
}

impl ToolName {
    fn named(name: &str) -> Option<Self> {
        NeutralTool::named(name).map(Self::Neutral).or_else(|| {
            name.starts_with("mcp__")
                .then(|| Self::Mcp(McpToolName(name.to_owned())))
        })
    }

    /// The name as the entry writes it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Neutral(tool) => tool.as_str(),
            Self::Mcp(tool) => tool.as_str(),
        }
    }

    const fn scope_kind(&self) -> ScopeKind {
        match self {
            Self::Neutral(tool) => tool.scope_kind(),
            Self::Mcp(_) => ScopeKind::Other,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScopeKind {
    Path,
    Command,
    Other,
}

/// What a tool is limited to, as written after the first `:` of an entry. The kind follows the
/// tool, because the narrowing rule compares each kind its own way.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolScope {
    /// A path or a glob, for `read`, `write`, `edit`, `grep`, `glob` and `notebook_edit`.
    Path(String),
    /// A command line pattern, for `bash`.
    Command(String),
    /// The scope of any other tool.
    Other(String),
}

impl ToolScope {
    fn of(kind: ScopeKind, text: &str) -> Self {
        let text = text.to_owned();
        match kind {
            ScopeKind::Path => Self::Path(text),
            ScopeKind::Command => Self::Command(text),
            ScopeKind::Other => Self::Other(text),
        }
    }

    /// The scope as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Path(text) | Self::Command(text) | Self::Other(text) => text,
        }
    }
}

/// One entry of an agent's `tools`: a tool, and the scope it is limited to when it has one. An
/// entry without a scope grants the whole tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolGrant {
    tool: ToolName,
    scope: Option<ToolScope>,
}

impl ToolGrant {
    /// The tool.
    #[must_use]
    pub const fn tool(&self) -> &ToolName {
        &self.tool
    }

    /// The scope, `None` when the entry grants the whole tool.
    #[must_use]
    pub const fn scope(&self) -> Option<&ToolScope> {
        self.scope.as_ref()
    }
}

impl TryFrom<&str> for ToolGrant {
    type Error = AgentRefusal;

    /// The entry `tool` or `tool:scope`. The tool is the text before the first `:`.
    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            return Err(AgentRefusal::Tool);
        }
        let (name, scope) = text
            .split_once(':')
            .map_or((text, None), |(name, scope)| (name, Some(scope)));
        let tool = ToolName::named(name).ok_or(AgentRefusal::UnknownTool)?;
        let scope = scope.map(|scope| ToolScope::of(tool.scope_kind(), scope));
        Ok(Self { tool, scope })
    }
}

impl fmt::Display for ToolGrant {
    /// The entry as written: `tool` or `tool:scope`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.tool.as_str())?;
        self.scope
            .as_ref()
            .map_or(Ok(()), |scope| write!(formatter, ":{}", scope.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::{NeutralTool, ToolGrant, ToolName, ToolScope};
    use crate::item::AgentRefusal;

    fn grant(text: &str) -> ToolGrant {
        ToolGrant::try_from(text).expect("a tool entry")
    }

    #[test]
    fn a_scope_has_the_kind_its_tool_gives() {
        assert_eq!(
            grant("write:docs/**").scope(),
            Some(&ToolScope::Path("docs/**".to_owned()))
        );
        assert_eq!(
            grant("bash:mise run *").scope(),
            Some(&ToolScope::Command("mise run *".to_owned()))
        );
        assert_eq!(
            grant("web_fetch:example.invalid").scope(),
            Some(&ToolScope::Other("example.invalid".to_owned()))
        );
        assert_eq!(grant("read").scope(), None);
    }

    #[test]
    fn the_scope_starts_at_the_first_colon() {
        let entry = grant("bash:echo a:b");
        assert_eq!(entry.tool(), &ToolName::Neutral(NeutralTool::Bash));
        assert_eq!(entry.scope().map(ToolScope::as_str), Some("echo a:b"));
    }

    #[test]
    fn an_mcp_tool_is_kept_as_written() {
        assert_eq!(grant("mcp__docs__search").to_string(), "mcp__docs__search");
    }

    #[test]
    fn an_entry_prints_as_it_was_written() {
        for text in ["read", "write:docs/**", "bash:mise run *", "mcp__a__b:x"] {
            assert_eq!(grant(text).to_string(), text);
        }
    }

    #[test]
    fn an_unknown_tool_and_a_blank_entry_are_refused() {
        assert_eq!(
            ToolGrant::try_from("Write:docs"),
            Err(AgentRefusal::UnknownTool)
        );
        assert_eq!(ToolGrant::try_from(":docs"), Err(AgentRefusal::UnknownTool));
        assert_eq!(ToolGrant::try_from("mcp_x"), Err(AgentRefusal::UnknownTool));
        assert_eq!(ToolGrant::try_from("  "), Err(AgentRefusal::Tool));
    }
}
