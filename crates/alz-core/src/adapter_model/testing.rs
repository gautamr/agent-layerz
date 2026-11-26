//! The pipeline of the Claude adapter's tests: stacks built from text, merged by the core, given to
//! the adapter as the model of the contract, and its output written by the core's emitter. The
//! tests of the adapter state their input as the files of a layer, as the acceptance fixtures do,
//! and reach the merge only through this module, so the adapter's own code never names a merge
//! type.

use alz_adapter::{Adapter, Context, Output, OutputFile};

use crate::adapter_model::model_for;
use crate::diagnostic::Diagnostics;
use crate::render::adapter_output::emit;
use crate::render::tree::FileTree;
use crate::stack::Workspace;
use crate::target::claude::ClaudeAdapter;

pub(crate) use crate::merge::Merged;
pub(crate) use crate::merge::testing::{
    layer_executable, layer_from_yaml, merged as merged_stack, merged_with,
};

/// The files and diagnostics of the Claude adapter's output, as the core writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Emitted {
    tree: FileTree,
    diagnostics: Diagnostics,
}

impl Emitted {
    /// The files, by path.
    pub(crate) const fn tree(&self) -> &FileTree {
        &self.tree
    }

    /// The errors and warnings of the rendering.
    pub(crate) const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The tree and the diagnostics, handed over.
    pub(crate) fn into_parts(self) -> (FileTree, Diagnostics) {
        (self.tree, self.diagnostics)
    }
}

/// The paths of the agents, skills, commands, slot stubs and rules.
const ITEM_FOLDERS: [&str; 3] = [".claude/agents/", ".claude/skills/", ".claude/rules/"];

/// The adapter's whole output for `merged` under `context`.
fn output_of(merged: &Merged, context: &Context) -> Output {
    ClaudeAdapter::new().render(&model_for(merged, crate::target::claude()), context)
}

/// The part of `output` whose files `keep` accepts, with the diagnostics whose code `keep_code`
/// accepts, emitted by the core.
fn part(
    output: &Output,
    keep: impl Fn(&OutputFile) -> bool,
    keep_code: impl Fn(&str) -> bool,
) -> Emitted {
    let files = output
        .files
        .iter()
        .filter(|file| keep(file))
        .cloned()
        .collect();
    let diagnostics = output
        .diagnostics
        .iter()
        .filter(|found| keep_code(&found.code))
        .cloned()
        .collect();
    let (tree, diagnostics) = emit(
        crate::target::claude(),
        &Output::new(files, diagnostics, output.coverage.clone()),
    );
    Emitted { tree, diagnostics }
}

/// The Claude agents, skills, commands, slot stubs and rules of a merged stack.
pub(crate) fn files_of(merged: &Merged) -> Emitted {
    part(
        &output_of(merged, &Context::default()),
        |file| {
            ITEM_FOLDERS
                .iter()
                .any(|folder| file.path.starts_with(folder))
        },
        |code| code != "hook-on-fail",
    )
}

/// The Claude settings, hook scripts and `.mcp.json` of a merged stack under a workspace.
pub(crate) fn settings_files_of(merged: &Merged, workspace: &Workspace) -> Emitted {
    let context = Context::new(
        workspace
            .add_dirs()
            .iter()
            .map(|dir| dir.as_str().to_owned())
            .collect(),
    );
    part(
        &output_of(merged, &context),
        |file| {
            file.path == ".claude/settings.json"
                || file.path == ".mcp.json"
                || file.path.starts_with(".claude/hooks/")
        },
        |code| code == "hook-on-fail",
    )
}

/// `CLAUDE.md` of a merged stack, or the errors that kept it from being written.
pub(crate) fn claude_md_of(merged: &Merged) -> Result<String, Diagnostics> {
    let output = output_of(merged, &Context::default());
    let (tree, found) = emit(
        crate::target::claude(),
        &Output::new(
            output
                .files
                .iter()
                .filter(|file| file.path == "CLAUDE.md")
                .cloned()
                .collect(),
            Vec::new(),
            output.coverage.clone(),
        ),
    );
    let path = crate::render::tree::OutputPath::try_from("CLAUDE.md").ok();
    match path.and_then(|path| tree.get(&path).map(|file| file.bytes.clone())) {
        Some(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        None => Err(found),
    }
}
