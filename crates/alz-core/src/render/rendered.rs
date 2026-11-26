//! The pure build (spec §6, §8.1): [`render`] makes every generated file of a merged stack, and
//! what it found, without touching the file system.
//!
//! The order is: `AGENTS.md`; then the adapter of each target (`CLAUDE.md`, agents, skills,
//! commands, rules, `settings.json`, the hook scripts, `.mcp.json`); then the passthrough files
//! of the target; then the protected drop, so a protected path leaves the tree whichever source
//! made it. A [`Rendered`] with an error is not written: [`Rendered::to_write`] gives the tree
//! only when no error was found.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::lock::Lock;
use crate::merge::Merged;
use crate::path::SafeRelPath;
use crate::render::instructions::{AgentsMd, agents_md_path, claude_md, claude_md_path};
use crate::render::lock_of::lock_of;
use crate::render::passthrough;
use crate::render::protected::{CLAUDE_MEMORY_PATTERNS, Protected};
use crate::render::tree::{FileTree, GeneratedFile, OutputPath};
use crate::stack::{Manifest, ResolvedStack};
use crate::target::REGISTERED;
use crate::target::claude::{ClaudeFiles, ClaudeSettingsFiles};
use crate::untrusted_text::Stripped;
use alz_adapter::TargetId;

/// The folders the Claude adapter owns, where a file the build does not make is a collision
/// (§4.3, §8.1).
const CLAUDE_MANAGED_FOLDERS: [&str; 4] = [
    ".claude/agents",
    ".claude/skills",
    ".claude/rules",
    ".claude/hooks",
];

/// What a build made: the generated files, what it found, the folders the adapters manage and the
/// protected paths. It has no public constructor: [`render`] makes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    tree: FileTree,
    diagnostics: Diagnostics,
    managed_folders: Vec<SafeRelPath>,
    protected: Protected,
}

impl Rendered {
    /// The generated files, without the protected paths.
    #[must_use]
    pub const fn tree(&self) -> &FileTree {
        &self.tree
    }

    /// The errors and warnings of the build, in the order of the steps.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The files to write: the tree when the build found no error, and `None` when it found one,
    /// because a build with an error writes nothing.
    #[must_use]
    pub fn to_write(&self) -> Option<&FileTree> {
        (!self.diagnostics.has_errors()).then_some(&self.tree)
    }

    /// The folders of the adapters, in which a file the build does not make is a collision
    /// (§4.3).
    #[must_use]
    pub fn managed_folders(&self) -> &[SafeRelPath] {
        &self.managed_folders
    }

    /// The patterns the build leaves alone, for the writer: the defaults, the targets' own and
    /// the manifest's.
    #[must_use]
    pub const fn protected(&self) -> &Protected {
        &self.protected
    }

    /// The lock of the build for `stack` (§5.3), which lists exactly the tree's files.
    #[must_use]
    pub fn lock(&self, stack: &ResolvedStack) -> Lock {
        lock_of(&self.tree, stack)
    }
}

/// Renders `merged` for the targets of `manifest`. Pure and deterministic: the same inputs give
/// the same bytes and the same diagnostics in the same order.
#[must_use]
pub fn render(merged: &Merged, manifest: &Manifest) -> Rendered {
    let targets = manifest.targets();
    let (agents, agents_found) = agents_md(merged, manifest);
    let made = targets
        .iter()
        .map(|target| adapter(target, merged, manifest))
        .fold(
            (agents, agents_found),
            |(tree, found), (more, more_found)| {
                let (tree, collisions) = join(tree, &more);
                (tree, found.merge(more_found).merge(collisions))
            },
        );
    let (tree, found) = targets.iter().fold(made, |(tree, found), target| {
        let (tree, more) = passthrough::join(tree, merged.passthrough(), target);
        (tree, found.merge(more))
    });
    let claude = targets.contains(crate::target::claude());
    let protected = Protected::new(
        if claude { &CLAUDE_MEMORY_PATTERNS } else { &[] },
        manifest.protected_paths(),
    );
    let (tree, dropped) = protected.drop_from(tree);
    Rendered {
        tree,
        diagnostics: found.merge(dropped),
        managed_folders: if claude {
            CLAUDE_MANAGED_FOLDERS
                .iter()
                .filter_map(|folder| SafeRelPath::try_from(*folder).ok())
                .collect()
        } else {
            Vec::new()
        },
        protected,
    }
}

/// `AGENTS.md` and its budget problems; no file when the header cannot be made.
fn agents_md(merged: &Merged, manifest: &Manifest) -> (FileTree, Diagnostics) {
    match AgentsMd::compose(merged) {
        Ok(agents) => {
            let over = agents.budget_problems(manifest.budgets());
            (single(agents_md_path(), agents.text()), over)
        }
        Err(problems) => (FileTree::new(), problems),
    }
}

/// The files of one target's adapter, and its diagnostics.
fn adapter(target: &TargetId, merged: &Merged, manifest: &Manifest) -> (FileTree, Diagnostics) {
    match target.as_str() {
        "claude" => claude(merged, manifest),
        "codex" => refused(
            Message::new("the '")
                .then(MessagePart::printable(target))
                .then("' adapter is not implemented yet (implemented: claude)"),
        ),
        _ => refused(
            Message::new("no adapter for '")
                .then(MessagePart::printable(target))
                .then("' (available: ")
                .then(Stripped::from_untrusted(&REGISTERED.join(", ")))
                .then(")"),
        ),
    }
}

/// No files, and the error `target` at `layer.yaml` with `message` (§8 **Targets without an
/// adapter**).
fn refused(message: Message) -> (FileTree, Diagnostics) {
    (
        FileTree::new(),
        Diagnostics::new().with(Diagnostic::error(
            Code::Target,
            Some(Where::repository_file(Stripped::from_untrusted(
                "layer.yaml",
            ))),
            message,
        )),
    )
}

/// `CLAUDE.md`, the agents, skills, commands and rules, `settings.json`, the hooks and `.mcp.json`.
fn claude(merged: &Merged, manifest: &Manifest) -> (FileTree, Diagnostics) {
    let (thin, thin_found) = match claude_md(merged) {
        Ok(text) => (single(claude_md_path(), &text), Diagnostics::new()),
        Err(problems) => (FileTree::new(), problems),
    };
    let (files, files_found) = ClaudeFiles::of(merged).into_parts();
    let (settings, settings_found) =
        ClaudeSettingsFiles::of(merged, manifest.workspace()).into_parts();
    [(files, files_found), (settings, settings_found)]
        .into_iter()
        .fold((thin, thin_found), |(tree, found), (more, more_found)| {
            let (tree, collisions) = join(tree, &more);
            (tree, found.merge(more_found).merge(collisions))
        })
}

/// A tree of the one file `path` with the text `text`.
fn single(path: OutputPath, text: &str) -> FileTree {
    // One path cannot collide with itself, so the tree is always made.
    FileTree::from_files([(path, GeneratedFile::text(text))]).unwrap_or_default()
}

/// `first` with the files of `more` added, and an error `collision` for each file of `more` whose
/// path is the same file as one already there; the file already there stays.
fn join(first: FileTree, more: &FileTree) -> (FileTree, Diagnostics) {
    more.iter().fold(
        (first, Diagnostics::new()),
        |(mut tree, collisions), (path, file)| match tree.insert(path.clone(), file.clone()) {
            Ok(()) => (tree, collisions),
            Err(collision) => (
                tree,
                collisions.with(Diagnostic::error(
                    Code::Collision,
                    None,
                    Message::new("").then(Stripped::from_untrusted(&collision.to_string())),
                )),
            ),
        },
    )
}

#[cfg(test)]
mod tests;
