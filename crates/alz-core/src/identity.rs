//! The commit identity of a layer (spec §7.12) as validated types, and the pure decisions of the
//! repository's git identity (spec §9.4): reading what git reports as typed findings that hold
//! no value, the check against the declaration, the setup plan and the report lines. Nothing here
//! runs git or reads a file; `alz-io` does and hands the results to these functions.

mod check;
mod commit_identity;
mod declared;
mod effective;
mod finding;
mod origin;
mod report;
mod setup;
mod shell_safe;
mod slot;
#[cfg(test)]
mod tests;

pub use check::{Command, Findings, IdentityCheck, RepoState, Verdicts};
pub use commit_identity::{
    CommitIdentity, EmailAddress, EmailError, IdentityName, IdentityNameError,
};
pub use declared::{DeclaredBy, DeclaredIdentity};
pub use effective::{Agreement, Effective};
pub use finding::{ConfigAnswer, Finding, Presence, Source, SourceKey, locate};
pub use origin::{ConfigOrigin, GitScope};
pub use report::{Home, Report};
pub use setup::{Blocker, GitConfigPlace, KeyAction, KeyState, SetupDecision, SetupPlan};
pub use shell_safe::{FixCommand, FixScope, NotShellSafe, ShellSafe};
pub use slot::{ConfigKey, Field, Role, Slot};
