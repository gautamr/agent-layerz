//! The repository's stack manifest (spec §5.2): where each layer comes from and how the build is
//! configured, read from the YAML tree into validated types. Resolving a source to a directory
//! is R3's.

mod budgets;
mod entry;
mod git_ref;
mod manifest;
mod origin;
pub mod resolution_failure;
mod resolved;
mod source;
mod validate;
mod workspace;

pub use budgets::{BudgetError, Budgets};
pub use entry::{DeclaredStack, StackEntry, StackError};
pub use git_ref::{GitRef, GitRefError};
pub use manifest::{Manifest, McpName, Options, ProtectedPattern};
pub use origin::{CloneOrigin, OriginRefusal, Probe, github_origin, locate_git};
pub use resolved::{FailedEntry, Pin, ResolvedEntry, ResolvedLayer, ResolvedStack, StackReport};
pub use source::{GitHubName, GitUrl, Source, SourceDir, SourceError};
pub use workspace::{Workspace, WorkspaceDir, WorkspaceDirError};

#[cfg(test)]
mod tests;
