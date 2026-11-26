//! Skills, commands, slot stubs, rules, permissions, MCP servers and models of the model.

use std::collections::BTreeMap;

use crate::model::mode::{FloorField, Mode};
use crate::source::Source;

/// One file of a skill folder (§7.3), kept as the layer has it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkillFile {
    /// The path below the skill's folder, as `SKILL.md` or `scripts/run.sh`.
    pub path: String,
    /// The bytes.
    pub content: Vec<u8>,
    /// Whether the file is executable.
    pub executable: bool,
}

impl SkillFile {
    /// A skill file.
    #[must_use]
    pub fn new(path: impl Into<String>, content: Vec<u8>, executable: bool) -> Self {
        Self {
            path: path.into(),
            content,
            executable,
        }
    }
}

/// A skill: the winning definition of its folder (§7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Skill {
    /// The skill's name, the name of its folder.
    pub name: String,
    /// Every file of the folder.
    pub files: Vec<SkillFile>,
    /// The definitions that were not overridden, for the header of `SKILL.md`.
    pub sources: Vec<Source>,
}

impl Skill {
    /// A skill.
    #[must_use]
    pub fn new(name: impl Into<String>, files: Vec<SkillFile>) -> Self {
        Self {
            name: name.into(),
            files,
            sources: Vec::new(),
        }
    }
}

/// A command (§7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Command {
    /// The command's name.
    pub name: String,
    /// The description.
    pub description: String,
    /// The `argument-hint`, when set.
    pub argument_hint: Option<String>,
    /// The body.
    pub body: String,
    /// The definition it came from.
    pub sources: Vec<Source>,
}

impl Command {
    /// A command with a name, a description and a body.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            argument_hint: None,
            body: body.into(),
            sources: Vec::new(),
        }
    }
}

/// A slot stub (§7.3): a skill that a layer declares and no layer defines.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SlotStub {
    /// The skill's name.
    pub name: String,
}

impl SlotStub {
    /// A slot stub.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// The text a layer adds for one target only (§7.1, `targets/<target>/<NAME>.extra.md`), such as
/// the Claude-only block of `CLAUDE.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TargetInstruction {
    /// The text, as the layer wrote it.
    pub body: String,
    /// The layer file it came from.
    pub source: Source,
}

impl TargetInstruction {
    /// A target-only instruction from `source`.
    #[must_use]
    pub fn new(body: impl Into<String>, source: Source) -> Self {
        Self {
            body: body.into(),
            source,
        }
    }
}

/// A path-scoped rule (§7.10).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PathRule {
    /// The rule's name, the file's stem.
    pub name: String,
    /// The globs, each once, that decide when the rule is loaded.
    pub paths: Vec<String>,
    /// The body, with the sections that `mode: extend` appends.
    pub body: String,
    /// The files it came from.
    pub sources: Vec<Source>,
}

impl PathRule {
    /// A rule.
    #[must_use]
    pub fn new(name: impl Into<String>, paths: Vec<String>, body: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            paths,
            body: body.into(),
            sources: Vec::new(),
        }
    }
}

/// The merged permissions (§7.6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Permissions {
    /// The `allow` rules.
    pub allow: Vec<String>,
    /// The `ask` rules.
    pub ask: Vec<String>,
    /// The `deny` rules. Each is also a security item.
    pub deny: Vec<String>,
    /// The merged `default_mode`, never looser than the strictest floor.
    pub default_mode: Option<Mode>,
    /// The floor fields that are set, which are also security items.
    pub floor_fields: Vec<FloorField>,
}

/// How a server of `mcp.yaml` is reached (§7.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpTransport {
    /// `stdio`: a command with arguments.
    Stdio {
        /// The command.
        command: String,
        /// The arguments, in order; none when `args` is not written, which differs from a list
        /// written empty (`args: []`): a target's file keeps the difference.
        args: Option<Vec<String>>,
        /// The names of the environment variables to pass through; values are never stored.
        env_from: Vec<String>,
    },
    /// `http`.
    Http {
        /// The URL.
        url: String,
    },
    /// `sse`.
    Sse {
        /// The URL.
        url: String,
    },
}

/// An enabled MCP server: one whose id is in `mcp.enable` (§7.7).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct McpServer {
    /// The server's id.
    pub id: String,
    /// How it is reached.
    pub transport: McpTransport,
}

impl McpServer {
    /// An enabled server.
    #[must_use]
    pub fn new(id: impl Into<String>, transport: McpTransport) -> Self {
        Self {
            id: id.into(),
            transport,
        }
    }
}

/// The models resolved for the target (§7.8). An alias with no model for the target is not here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Models {
    /// Each alias and its concrete model id for this target.
    pub aliases: BTreeMap<String, String>,
    /// Each role and the alias it names.
    pub roles: BTreeMap<String, String>,
    /// The concrete model of the default alias, when there is one.
    pub default: Option<String>,
}
