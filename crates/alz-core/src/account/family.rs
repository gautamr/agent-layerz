//! The families of account folders (spec §7.13): `claude` and `codex`. A family is not an adapter
//! and not a target id (§5.2 **Target ids**): it names where a tool keeps its account folders
//! (`~/.claude-<name>`, `~/.codex-<name>`) and the variable that points at one, so it stays a
//! closed set however many adapters exist.

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Printable, sealed::Sealed};

/// A family of account folders: `claude` or `codex`. Each variant is named in the `account` map
/// of a layer and in `alz use` by its lower-case word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountFamily {
    /// Claude Code's folders, `~/.claude-<name>`.
    Claude,
    /// Codex's folders, `~/.codex-<name>`.
    Codex,
}

impl AccountFamily {
    /// Every family, in name order.
    pub const ALL: [Self; 2] = [Self::Claude, Self::Codex];

    /// The family's name as the spec writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

/// Text that is not the name of a family. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not an account family: claude or codex")]
pub struct AccountFamilyError;

impl TryFrom<&str> for AccountFamily {
    type Error = AccountFamilyError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|family| family.as_str() == text)
            .ok_or(AccountFamilyError)
    }
}

impl fmt::Display for AccountFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Sealed for AccountFamily {}

impl Printable for AccountFamily {
    fn printable(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountFamily, AccountFamilyError};

    #[test]
    fn the_families_are_listed_in_name_order() {
        let names: Vec<&str> = AccountFamily::ALL
            .iter()
            .map(|family| family.as_str())
            .collect();
        assert_eq!(names, ["claude", "codex"]);
        assert!(AccountFamily::Claude < AccountFamily::Codex);
    }

    #[test]
    fn a_family_is_read_from_its_exact_lower_case_name() {
        assert_eq!(AccountFamily::try_from("codex"), Ok(AccountFamily::Codex));
        assert_eq!(AccountFamily::try_from("Claude"), Err(AccountFamilyError));
        assert_eq!(
            AccountFamily::try_from("antigravity"),
            Err(AccountFamilyError)
        );
        assert_eq!(AccountFamily::try_from(""), Err(AccountFamilyError));
    }
}
