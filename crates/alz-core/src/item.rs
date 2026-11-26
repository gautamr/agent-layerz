//! The per-layer readers of the items of §7: each turns the files of one layer that belong to one
//! kind (see [`crate::layer::ItemPath`]) into validated values, or into diagnostics, without
//! looking at any other layer. The merge of the kind then folds these values over the stack.
//!
//! A reader reports every problem of every file in one pass. A file with a problem gives no item,
//! so a merged model never holds a half-read file.

mod agent;
mod env_var_name;
mod hook;
mod instruction;
mod kebab_case;
mod list_edit;
mod mcp;
mod model;
mod native_keys;
mod native_types;
mod passthrough;
mod permission;
mod protected_variable;
mod rule;
mod settings;
mod skill;

pub use agent::{
    AgentBase, AgentDescription, AgentItems, AgentName, AgentOverlay, AgentRefusal, AgentScalars,
    Isolation, McpToolName, Memory, ModelId, NativeFields, NativeMap, NativeValue, NeutralTool,
    OverlayMode, PermissionMode, Role, SkillName, ToolGrant, ToolName, ToolScope,
};
pub use env_var_name::{EnvVarName, EnvVarNameError};
pub use hook::{
    CommandLine, Hook, HookAction, HookEntry, HookId, HookItems, HookRefusal, HookScript, HookTool,
    LifecycleEvent, Lock, Matcher, NativeEvent, NativeEventName, NeutralEvent, OnFail, Seconds,
    ToolFilter, ToolMoment, Trigger,
};
pub use instruction::{
    ClaudeExtra, InstructionEdit, InstructionFile, InstructionItems, InstructionMode,
    InstructionModeError,
};
pub use list_edit::{EditSegment, EditSign, WrittenEdit};
pub use mcp::{McpItems, McpRefusal, McpServer, McpServerId, Transport};
pub use model::{BlankModelName, ConcreteModel, ModelAlias, ModelItems, Models};
pub use native_keys::{KeyRule, KeyRules, Looseness, Loosening, NativeKey, Step, Strictness};
pub use native_types::{CLAUDE_AGENT_FIELDS, CLAUDE_SETTINGS, Ends, Field, Shape, Span, Typed};
pub use passthrough::{PassthroughFile, PassthroughItems};
pub use permission::{
    Binding, DefaultMode, FloorFlag, PermissionItems, PermissionRefusal, PermissionRule,
    Permissions,
};
pub use rule::{
    Glob, GlobError, RuleBase, RuleEdit, RuleExtension, RuleFile, RuleItems, RuleMode,
    RuleModeError, RuleName, RuleNameError,
};
pub use settings::{Settings, SettingsItems, SettingsTree};
pub use skill::{
    ArgumentHint, Command, Skill, SkillDescription, SkillDescriptionError, SkillDirectory,
    SkillFile, SkillItems,
};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Where};
use crate::layer::LayerName;
use crate::markdown::MarkdownDoc;
use crate::path::SafeRelPath;
use crate::schema::SchemaErrors;
use crate::untrusted_text::Stripped;

/// The place of a file of a layer, `<layer>:<path>`.
pub(crate) fn place(layer: &LayerName, path: &SafeRelPath) -> Where {
    Where::layer_file(
        Stripped::from_untrusted(layer.as_str()),
        Stripped::from_untrusted(path.as_str()),
    )
}

/// The Markdown file at `path`, or the diagnostic that says why it cannot be read.
pub(crate) fn read_markdown(
    layer: &LayerName,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Result<MarkdownDoc, Diagnostic> {
    MarkdownDoc::parse(bytes).map_err(|error| error.into_diagnostic(place(layer, path)))
}

/// One error of `code` for each problem of a file's frontmatter, at the file and the line and
/// column of the problem.
pub(crate) fn schema_diagnostics(code: Code, at: &Where, errors: &SchemaErrors) -> Diagnostics {
    errors
        .iter()
        .map(|error| {
            Diagnostic::error(code, Some(at.clone().at(error.position())), error.message())
        })
        .collect()
}

/// Both values, or the diagnostics of both: unlike `?`, one failure does not hide the other.
pub(crate) fn zip_diagnostics<A, B>(
    first: Result<A, Diagnostics>,
    second: Result<B, Diagnostics>,
) -> Result<(A, B), Diagnostics> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok((first, second)),
        (Err(problems), Ok(_)) | (Ok(_), Err(problems)) => Err(problems),
        (Err(first), Err(second)) => Err(first.merge(second)),
    }
}
