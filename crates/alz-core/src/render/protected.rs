//! Protected paths (spec §4.5): repository-owned content that the build never generates, even
//! when a layer ships a file at the same path.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::render::glob::PathGlob;
use crate::render::tree::{FileTree, OutputPath};
use crate::stack::ProtectedPattern;
use crate::untrusted_text::Stripped;

/// §4.5: task boards, docs, specs and local files.
pub const DEFAULT_PATTERNS: [&str; 4] = ["backlog/**", "docs/**", "specs/**", "*.local.*"];

/// The folders where Claude keeps per-agent memory (target-facts §9): repository-owned lessons.
pub const CLAUDE_MEMORY_PATTERNS: [&str; 2] =
    [".claude/agent-memory/**", ".claude/agent-memory-local/**"];

/// The patterns a build leaves alone: the defaults, the target's own and the manifest's
/// `protected_paths`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protected(Vec<PathGlob>);

impl Protected {
    /// The defaults, `target_patterns` and then `extra`.
    #[must_use]
    pub fn new(target_patterns: &[&str], extra: &[ProtectedPattern]) -> Self {
        Self(
            DEFAULT_PATTERNS
                .iter()
                .chain(target_patterns)
                .map(|pattern| PathGlob::parse(pattern))
                .chain(extra.iter().map(PathGlob::from))
                .collect(),
        )
    }

    /// Whether a pattern covers `path`.
    #[must_use]
    pub fn covers(&self, path: &OutputPath) -> bool {
        self.0.iter().any(|glob| glob.matches(path.as_str()))
    }

    /// The tree without its protected paths, and one `protected` warning for each, in path
    /// order.
    #[must_use]
    pub fn drop_from(&self, tree: FileTree) -> (FileTree, Diagnostics) {
        let (kept, dropped) = tree.without(|path| self.covers(path));
        let warnings = dropped
            .iter()
            .fold(Diagnostics::default(), |warnings, path| {
                warnings.with(Diagnostic::warning(
                    Code::Protected,
                    Some(Where::repository_file(Stripped::from_untrusted(
                        path.as_str(),
                    ))),
                    Message::from(
                        "protected path; a layer ships it, but the build leaves it alone",
                    ),
                ))
            });
        (kept, warnings)
    }
}

#[cfg(test)]
mod tests;
