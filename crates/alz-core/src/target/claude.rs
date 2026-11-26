//! The Claude adapter's files for agents, skills, commands, slot stubs and rules (spec §8.1),
//! made from the model of the adapter contract alone (`alz_adapter::Model`, spec §8.0), never from
//! the merge types of the core. The result is a [`FileTree`] and the diagnostics of the
//! rendering; nothing is written.
//!
//! Only what `docs/target-facts.md` records about Claude Code drives the output: the agent
//! frontmatter fields and tool names (§1), skills and commands (§5) and `paths` in a rule (§9).

mod agents;
mod document;
pub mod facts;
mod frontmatter;
mod hooks;
mod mcp;
mod permissions;
mod rules;
mod settings;
mod skills;
mod tools;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::render::tree::{FileTree, GeneratedFile, OutputPath};
use alz_adapter::{Context, Model};

pub use tools::{is_model_value, neutral_permission_mode, neutral_tool};

/// The files of the Claude adapter for agents, skills, commands, slot stubs and rules, and what
/// the rendering found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeFiles {
    tree: FileTree,
    diagnostics: Diagnostics,
}

impl ClaudeFiles {
    /// Renders every agent, skill, command, slot stub and rule of `model`.
    #[must_use]
    pub fn of(model: &Model) -> Self {
        let (tree, diagnostics) = assemble(vec![
            agents::render(model),
            skills::render(model),
            rules::render(model),
        ]);
        Self { tree, diagnostics }
    }

    /// The files, by path.
    #[must_use]
    pub const fn tree(&self) -> &FileTree {
        &self.tree
    }

    /// The errors and warnings of the rendering.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The tree and the diagnostics, handed over.
    #[must_use]
    pub fn into_parts(self) -> (FileTree, Diagnostics) {
        (self.tree, self.diagnostics)
    }
}

/// The files of the Claude adapter for `settings.json`, the hook scripts and `.mcp.json`, and what
/// the rendering found (spec §8.1). `settings.json` is made from the target's merged settings and
/// the first-class items (permissions, hooks, the default model, the workspace folders), never
/// from any other source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeSettingsFiles {
    tree: FileTree,
    diagnostics: Diagnostics,
}

impl ClaudeSettingsFiles {
    /// Renders `.claude/settings.json`, the copied hook scripts and `.mcp.json`.
    #[must_use]
    pub fn of(model: &Model, context: &Context) -> Self {
        let (tree, diagnostics) =
            assemble(vec![settings::render(model, context), mcp::render(model)]);
        Self { tree, diagnostics }
    }

    /// The files, by path.
    #[must_use]
    pub const fn tree(&self) -> &FileTree {
        &self.tree
    }

    /// The errors and warnings of the rendering.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The tree and the diagnostics, handed over.
    #[must_use]
    pub fn into_parts(self) -> (FileTree, Diagnostics) {
        (self.tree, self.diagnostics)
    }
}

/// The files of every output in one tree, two outputs of one path a `collision` error, and all
/// the diagnostics.
fn assemble(made: Vec<Output>) -> (FileTree, Diagnostics) {
    let (tree, collisions) = made.iter().flat_map(|output| output.files.iter()).fold(
        (FileTree::new(), Diagnostics::new()),
        |(mut tree, collisions), (path, file)| match tree.insert(path.clone(), file.clone()) {
            Ok(()) => (tree, collisions),
            Err(collision) => (
                tree,
                collisions.with(Diagnostic::error(
                    Code::Collision,
                    None,
                    Message::new("").then(crate::Stripped::from_untrusted(&collision.to_string())),
                )),
            ),
        },
    );
    let diagnostics = made
        .into_iter()
        .fold(Diagnostics::new(), |all, output| {
            all.merge(output.diagnostics)
        })
        .merge(collisions);
    (tree, diagnostics)
}

/// What one kind of item rendered into: files in the order made, and diagnostics.
#[derive(Debug, Default)]
struct Output {
    files: Vec<(OutputPath, GeneratedFile)>,
    diagnostics: Diagnostics,
}

impl Output {
    /// The output of an item that gave one file and its diagnostics.
    fn one(path: Result<OutputPath, Diagnostic>, file: GeneratedFile, found: Diagnostics) -> Self {
        match path {
            Ok(path) => Self {
                files: vec![(path, file)],
                diagnostics: found,
            },
            Err(problem) => Self {
                files: Vec::new(),
                diagnostics: found.with(problem),
            },
        }
    }

    /// Both outputs, the first one's files first.
    fn then(self, later: Self) -> Self {
        Self {
            files: self.files.into_iter().chain(later.files).collect(),
            diagnostics: self.diagnostics.merge(later.diagnostics),
        }
    }
}

/// The path `text`, or the error `layer` for a name that cannot be an output path.
fn output_path(text: &str) -> Result<OutputPath, Diagnostic> {
    OutputPath::try_from(text).map_err(|problem| {
        Diagnostic::error(
            Code::Layer,
            None,
            Message::new("the generated path ")
                .then(crate::Stripped::from_untrusted(text))
                .then(" is not allowed: ")
                .then(crate::Stripped::from_untrusted(&problem.to_string())),
        )
    })
}

#[cfg(test)]
pub(crate) mod tests;
