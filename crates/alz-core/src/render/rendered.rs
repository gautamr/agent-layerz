//! The pure build (spec §6, §8.1): [`render`] makes every generated file of a merged stack, and
//! what it found, without touching the file system.
//!
//! The order is: `AGENTS.md`; then the adapter of each target (`CLAUDE.md`, agents, skills,
//! commands, rules, `settings.json`, the hook scripts, `.mcp.json`, all returned by the adapter
//! and written by the core); then the passthrough files
//! of the target; then the protected drop, so a protected path leaves the tree whichever source
//! made it. A [`Rendered`] with an error is not written: [`Rendered::to_write`] gives the tree
//! only when no error was found.

use crate::adapter_model::model_for;
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::lock::Lock;
use crate::merge::Merged;
use crate::path::SafeRelPath;
use crate::render::adapter_output;
use crate::render::instructions::{AgentsMd, agents_md_path};
use crate::render::lock_of::lock_of;
use crate::render::passthrough;
use crate::render::protected::Protected;
use crate::render::tree::{FileTree, GeneratedFile, OutputPath};
use crate::stack::{Manifest, ResolvedStack};
use crate::target::Registry;
use crate::untrusted_text::Stripped;
use alz_adapter::{Context, TargetId};

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

/// Renders `merged` for the targets of `manifest`, each by its adapter from `registry`. Pure and
/// deterministic: the same inputs give the same bytes and the same diagnostics in the same order.
#[must_use]
pub fn render(registry: &Registry, merged: &Merged, manifest: &Manifest) -> Rendered {
    let targets = manifest.targets();
    let (agents, agents_found) = agents_md(merged, manifest);
    let made = targets
        .iter()
        .map(|target| adapter(registry, target, merged, manifest))
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
    let descriptors: Vec<&alz_adapter::Descriptor> = targets
        .iter()
        .filter_map(|target| registry.descriptor(target))
        .collect();
    let patterns: Vec<&str> = descriptors
        .iter()
        .flat_map(|descriptor| descriptor.protected_patterns.iter().map(String::as_str))
        .collect();
    let protected = Protected::new(&patterns, manifest.protected_paths());
    let (tree, dropped) = protected.drop_from(tree);
    Rendered {
        tree,
        diagnostics: found.merge(dropped),
        managed_folders: descriptors
            .iter()
            .flat_map(|descriptor| descriptor.managed_folders.iter())
            .filter_map(|folder| SafeRelPath::try_from(folder.as_str()).ok())
            .collect(),
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

/// The files of one target's adapter, and its diagnostics: the adapter renders the model, and
/// the core writes what it returned (§8.0). A target without an adapter is the error `target`.
fn adapter(
    registry: &Registry,
    target: &TargetId,
    merged: &Merged,
    manifest: &Manifest,
) -> (FileTree, Diagnostics) {
    registry.adapter(target).map_or_else(
        || {
            (
                FileTree::new(),
                Diagnostics::new().with(registry.refusal(target)),
            )
        },
        |adapter| {
            let context = Context::new(
                manifest
                    .workspace()
                    .add_dirs()
                    .iter()
                    .map(|dir| dir.as_str().to_owned())
                    .collect(),
            );
            let output = adapter.render(&model_for(merged, target), &context);
            adapter_output::emit(target, &output)
        },
    )
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
