//! The per-layer readers of the items of §7: each turns the files of one layer that belong to one
//! kind (see [`crate::layer::ItemPath`]) into validated values, or into diagnostics, without
//! looking at any other layer. The merge of the kind then folds these values over the stack.
//!
//! A reader reports every problem of every file in one pass. A file with a problem gives no item,
//! so a merged model never holds a half-read file.

mod agent;
mod instruction;
mod kebab_case;
mod rule;

pub use agent::{
    AgentBase, AgentDescription, AgentItems, AgentName, AgentOverlay, AgentRefusal, AgentScalars,
    Isolation, McpToolName, Memory, ModelId, NativeFields, NativeMap, NativeValue, NeutralTool,
    OverlayMode, PermissionMode, Role, SkillName, ToolGrant, ToolName, ToolScope,
};
pub use instruction::{
    ClaudeExtra, InstructionEdit, InstructionFile, InstructionItems, InstructionMode,
    InstructionModeError,
};
pub use rule::{
    Glob, GlobError, RuleBase, RuleEdit, RuleExtension, RuleFile, RuleItems, RuleMode,
    RuleModeError, RuleName, RuleNameError,
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
