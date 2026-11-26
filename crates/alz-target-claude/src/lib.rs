//! The Claude adapter (spec §8.1): it implements the contract of `alz-adapter` (§8.0). From the
//! model of the adapter contract (`alz_adapter::Model`), never from the merge types of the core,
//! it returns structured files: JSON values, Markdown with its frontmatter and the sources of its
//! header, and bytes. The core emits them and writes every header (§4.3); nothing here writes one.
//!
//! Only what `docs/target-facts.md` records about Claude Code drives the output: the agent
//! frontmatter fields and tool names (§1), skills and commands (§5) and `paths` in a rule (§9).
//!
//! The crate depends on `alz-adapter` and on no other alz crate (decision-40), so the compiler
//! keeps it from seeing the internals of `alz-core` and from reaching `alz-io`; its `clippy.toml`
//! bans the file system, process and environment functions of the standard library. The tests of
//! the adapter on whole stacks need the merge of the core and live in `alz-core`.

#![forbid(unsafe_code)]

mod agents;
mod descriptor;
mod document;
mod frontmatter;
mod hooks;
mod instructions;
mod key_rules;
mod mcp;
mod permissions;
mod rules;
mod settings;
mod skills;
mod tools;

use alz_adapter::{
    Adapter, Content, Context, Coverage, Descriptor, Diagnostic, Enforcement, Level, Model, Output,
    OutputFile, Source,
};

pub use tools::{is_model_value, neutral_permission_mode, neutral_tool};

/// The target id of this adapter, `claude`.
const TARGET: &str = "claude";

/// The Claude adapter.
#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    descriptor: Descriptor,
}

impl ClaudeAdapter {
    /// The adapter, with its descriptor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
        }
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for ClaudeAdapter {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    fn render(&self, model: &Model, context: &Context) -> Output {
        let made = [
            instructions::render(model),
            agents::render(model),
            skills::render(model),
            rules::render(model),
            settings::render(model, context),
            mcp::render(model),
        ]
        .into_iter()
        .fold(Made::default(), Made::then);
        Output::new(
            made.files,
            made.diagnostics,
            Coverage::all(model.security.ids(), Enforcement::Expressed),
        )
    }
}

/// What one kind of item rendered into: files in the order made, and diagnostics.
#[derive(Debug, Default)]
struct Made {
    files: Vec<OutputFile>,
    diagnostics: Vec<Diagnostic>,
}

impl Made {
    /// One file.
    fn file(path: impl Into<String>, content: Content) -> Self {
        Self {
            files: vec![OutputFile::new(path, content)],
            diagnostics: Vec::new(),
        }
    }

    /// One diagnostic and no file.
    fn problem(diagnostic: Diagnostic) -> Self {
        Self {
            files: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }

    /// The same output with `diagnostics` before its own.
    fn after(self, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics: diagnostics.into_iter().chain(self.diagnostics).collect(),
            ..self
        }
    }

    /// Both outputs, the first one's files first.
    fn then(self, later: Self) -> Self {
        Self {
            files: self.files.into_iter().chain(later.files).collect(),
            diagnostics: self
                .diagnostics
                .into_iter()
                .chain(later.diagnostics)
                .collect(),
        }
    }
}

/// A diagnostic of the adapter about the layer file `at`, when it has one.
fn found(level: Level, code: &str, at: Option<&Source>, message: String) -> Diagnostic {
    let diagnostic = Diagnostic::new(level, code, message);
    match at {
        Some(source) => diagnostic.about(source.clone()),
        None => diagnostic,
    }
}

/// The file an item is about: the first of its sources.
fn place(named: &[Source]) -> Option<&Source> {
    named.first()
}

#[cfg(test)]
mod tests;
