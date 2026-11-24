//! The instruction files of a layer (spec §7.1): `instructions/<name>.md`, each a block of text
//! that may add to, replace or remove the block of the same file name of a more general layer, and
//! `targets/claude/CLAUDE.extra.md`, the Claude-specific text for `CLAUDE.md`.

use std::fmt;

use crate::diagnostic::{Code, Diagnostics};
use crate::layer::{ItemPath, LayerName};
use crate::path::SafeRelPath;
use crate::schema::{Checked, refine, unknown_fields, zip};
use crate::stack::ResolvedLayer;
use crate::yaml::fields::Section;

use super::{place, read_markdown, schema_diagnostics};

/// The only field an instruction file knows.
const FIELDS: [&str; 1] = ["mode"];

/// What `mode:` says an instruction file does to a block of a more general layer. Without the
/// field a file adds its own block, which is no mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionMode {
    /// `append`: the file adds its own block, as a file without `mode` does. The spec
    /// accepts it and layers write it, so it stays valid (decision of the lead, TASK-55.5.4).
    Append,
    /// `replace`: the block takes the place of the block of the same file name.
    Replace,
    /// `remove`: the block of the same file name is dropped.
    Remove,
}

/// A `mode` that is none of `append`, `replace` and `remove`. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionModeError;

impl InstructionModeError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be append, replace or remove"
    }
}

impl TryFrom<&str> for InstructionMode {
    type Error = InstructionModeError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "append" => Ok(Self::Append),
            "replace" => Ok(Self::Replace),
            "remove" => Ok(Self::Remove),
            _ => Err(InstructionModeError),
        }
    }
}

/// What one instruction file does: exactly one of three things, with the text only where there is
/// text. A `remove` file's own text is ignored, so it is not held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionEdit {
    /// Adds a block: no `mode`.
    Add(String),
    /// Replaces the block of the same file name of a more general layer: `mode: replace`.
    Replace(String),
    /// Drops the block of the same file name of a more general layer: `mode: remove`.
    Remove,
}

/// One `instructions/<name>.md` of a layer: the file, and what it does. The text is trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionFile {
    file: SafeRelPath,
    edit: InstructionEdit,
}

impl InstructionFile {
    /// The path in the layer, `instructions/<name>.md`. The file name with `.md` is the file's
    /// identity: `replace` and `remove` act on the block with the same one.
    #[must_use]
    pub const fn file(&self) -> &SafeRelPath {
        &self.file
    }

    /// What the file does.
    #[must_use]
    pub const fn edit(&self) -> &InstructionEdit {
        &self.edit
    }
}

impl fmt::Display for InstructionFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.file.fmt(formatter)
    }
}

/// The text of `targets/claude/CLAUDE.extra.md` of a layer, trimmed and not empty. Its frontmatter
/// is read for errors and holds nothing alz uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeExtra {
    layer: LayerName,
    body: String,
}

impl ClaudeExtra {
    /// The layer the text comes from.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        &self.layer
    }

    /// The text.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// What one layer holds of §7.1: its instruction files in name order (§6), and its Claude text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstructionItems {
    files: Vec<InstructionFile>,
    claude_extra: Option<ClaudeExtra>,
}

impl InstructionItems {
    /// The instruction files in name order.
    #[must_use]
    pub fn files(&self) -> &[InstructionFile] {
        &self.files
    }

    /// The layer's Claude-specific text, if it has any.
    #[must_use]
    pub const fn claude_extra(&self) -> Option<&ClaudeExtra> {
        self.claude_extra.as_ref()
    }

    /// Reads the instruction files and the Claude text of `layer`. Every problem is reported; a
    /// file with a problem gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        let name = layer.meta().name();
        let (files, claude_extra, diagnostics) = layer.contents().iter().fold(
            (Vec::new(), None, Diagnostics::new()),
            |(files, extra, found), (path, bytes)| match ItemPath::of(path) {
                ItemPath::Instruction { .. } => match read_file(name, path, bytes) {
                    Ok(file) => (files.into_iter().chain([file]).collect(), extra, found),
                    Err(problems) => (files, extra, found.merge(problems)),
                },
                ItemPath::ClaudeExtra => match read_claude_extra(name, path, bytes) {
                    Ok(text) => (files, text, found),
                    Err(problems) => (files, extra, found.merge(problems)),
                },
                ItemPath::LayerYaml
                | ItemPath::Agent { .. }
                | ItemPath::AgentOverlay { .. }
                | ItemPath::SkillFile { .. }
                | ItemPath::Command { .. }
                | ItemPath::Rule { .. }
                | ItemPath::Hooks
                | ItemPath::HookScript { .. }
                | ItemPath::Permissions
                | ItemPath::Mcp
                | ItemPath::Models
                | ItemPath::Settings
                | ItemPath::Passthrough { .. }
                | ItemPath::Other => (files, extra, found),
            },
        );
        (
            Self {
                files,
                claude_extra,
            },
            diagnostics,
        )
    }
}

/// The frontmatter of an instruction file: its `mode`, and no other field.
fn mode_of(section: &Section<'_>) -> Checked<Option<InstructionMode>> {
    zip(unknown_fields(section, &FIELDS), refine(section, "mode")).map(|((), mode)| mode)
}

fn read_file(
    layer: &LayerName,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Result<InstructionFile, Diagnostics> {
    let at = place(layer, path);
    let doc =
        read_markdown(layer, path, bytes).map_err(|problem| Diagnostics::new().with(problem))?;
    let mode = mode_of(&doc.front().section())
        .map_err(|errors| schema_diagnostics(Code::Instructions, &at, &errors))?;
    let body = doc.body().as_str().trim().to_owned();
    Ok(InstructionFile {
        file: path.clone(),
        edit: match mode {
            None | Some(InstructionMode::Append) => InstructionEdit::Add(body),
            Some(InstructionMode::Replace) => InstructionEdit::Replace(body),
            Some(InstructionMode::Remove) => InstructionEdit::Remove,
        },
    })
}

/// The Claude text of a layer: `None` when it is empty after trimming.
fn read_claude_extra(
    layer: &LayerName,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Result<Option<ClaudeExtra>, Diagnostics> {
    read_markdown(layer, path, bytes)
        .map_err(|problem| Diagnostics::new().with(problem))
        .map(|doc| {
            Some(doc.body().as_str().trim())
                .filter(|body| !body.is_empty())
                .map(|body| ClaudeExtra {
                    layer: layer.clone(),
                    body: body.to_owned(),
                })
        })
}

#[cfg(test)]
mod tests;
