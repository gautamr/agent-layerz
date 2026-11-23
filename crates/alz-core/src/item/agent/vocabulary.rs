//! The values of an agent's neutral fields (spec §7.2), each parsed once from the text of a file:
//! names that are kebab-case, text that is not blank, and the closed sets of words.

use std::fmt;

use super::super::kebab_case::is_kebab_case;
use crate::diagnostic::{Printable, sealed::Sealed};

/// Text that is not an agent's value for a field: the rule that was broken. The error holds no
/// input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRefusal {
    /// An agent name that is not kebab-case.
    Name,
    /// A skill name that is not kebab-case.
    Skill,
    /// A blank `description`.
    Description,
    /// A blank `role`.
    Role,
    /// A blank `model`.
    Model,
    /// A blank tool entry.
    Tool,
    /// A tool entry that names no tool of the neutral vocabulary or of an MCP server.
    UnknownTool,
    /// An `isolation` that is not `none` or `worktree`.
    Isolation,
    /// A `memory` that is not `none`, `project`, `user` or `local`.
    Memory,
    /// A `permission_mode` outside the vocabulary.
    PermissionMode,
    /// An overlay `mode` that is not `extend` or `replace`.
    OverlayMode,
    /// A list edit that is not a list or a string of `+[..]`, `-[..]` and `[..]`.
    ListEdit,
}

impl AgentRefusal {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Name | Self::Skill => {
                "must be kebab-case: lower-case letters and digits in groups joined by '-'"
            }
            Self::Description | Self::Role | Self::Model | Self::Tool => "must not be empty",
            Self::UnknownTool => {
                "must name a tool: read, write, edit, grep, glob, bash, web_fetch, web_search, agent, notebook_edit, todo_write, skill or an mcp__ tool, then an optional :scope"
            }
            Self::Isolation => "must be none or worktree",
            Self::Memory => "must be none, project, user or local",
            Self::PermissionMode => {
                "must be bypass_permissions, auto, accept_edits, default, dont_ask or plan"
            }
            Self::OverlayMode => "must be extend or replace",
            Self::ListEdit => "must be a list, or edits written +[..], -[..] and [..]",
        }
    }
}

macro_rules! kebab_name {
    ($(#[$meta:meta])* $name:ident, $refusal:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The name as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = AgentRefusal;

            fn try_from(text: &str) -> Result<Self, Self::Error> {
                if is_kebab_case(text) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err(AgentRefusal::$refusal)
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Sealed for $name {}

        impl Printable for $name {
            fn printable(&self) -> &str {
                &self.0
            }
        }
    };
}

macro_rules! text_value {
    ($(#[$meta:meta])* $name:ident, $refusal:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The text as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = AgentRefusal;

            fn try_from(text: &str) -> Result<Self, Self::Error> {
                if text.trim().is_empty() {
                    Err(AgentRefusal::$refusal)
                } else {
                    Ok(Self(text.to_owned()))
                }
            }
        }
    };
}

kebab_name! {
    /// The name of an agent: kebab-case, equal to the file stem, unique after the merge (§7.2). It
    /// is the agent's identity across layers.
    AgentName, Name
}

kebab_name! {
    /// The name of a skill in an agent's `skills` list (§7.2): kebab-case.
    SkillName, Skill
}

text_value! {
    /// When to use an agent, `description`: text with a character that is not white space.
    AgentDescription, Description
}

text_value! {
    /// The `role` of an agent, which model routing reads (§7.8).
    Role, Role
}

text_value! {
    /// The `model` of an agent: `inherit` or a logical model id of `models.yaml` (§7.8, §7.2).
    ModelId, Model
}

/// `isolation`: whether the agent runs in its own work tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Isolation {
    /// `none`.
    None,
    /// `worktree`.
    Worktree,
}

impl TryFrom<&str> for Isolation {
    type Error = AgentRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "none" => Ok(Self::None),
            "worktree" => Ok(Self::Worktree),
            _ => Err(AgentRefusal::Isolation),
        }
    }
}

/// `memory`: where an agent keeps its memory. The spec lists `none`, `project` and `user`; the
/// reference also accepts `local`, which Claude has, so it stays valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl TryFrom<&str> for Memory {
    type Error = AgentRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "none" => Ok(Self::None),
            "project" => Ok(Self::Project),
            "user" => Ok(Self::User),
            "local" => Ok(Self::Local),
            _ => Err(AgentRefusal::Memory),
        }
    }
}

/// `permission_mode`: the vocabulary of the reference, which the spec does not list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionMode {
    /// `bypass_permissions`.
    BypassPermissions,
    /// `auto`.
    Auto,
    /// `accept_edits`.
    AcceptEdits,
    /// `default`.
    Default,
    /// `dont_ask`.
    DontAsk,
    /// `plan`.
    Plan,
}

impl TryFrom<&str> for PermissionMode {
    type Error = AgentRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "bypass_permissions" => Ok(Self::BypassPermissions),
            "auto" => Ok(Self::Auto),
            "accept_edits" => Ok(Self::AcceptEdits),
            "default" => Ok(Self::Default),
            "dont_ask" => Ok(Self::DontAsk),
            "plan" => Ok(Self::Plan),
            _ => Err(AgentRefusal::PermissionMode),
        }
    }
}

/// What an overlay's `mode` says it does to the body of the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayMode {
    /// `extend`, the default: the overlay's text is appended as a section of its layer.
    Extend,
    /// `replace`: the overlay's text is the whole body.
    Replace,
}

impl TryFrom<&str> for OverlayMode {
    type Error = AgentRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "extend" => Ok(Self::Extend),
            "replace" => Ok(Self::Replace),
            _ => Err(AgentRefusal::OverlayMode),
        }
    }
}
