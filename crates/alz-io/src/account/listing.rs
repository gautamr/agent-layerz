//! The account folders that exist, from one listing of `$HOME` (spec §7.13 "Resolution", §9.1).
//!
//! A folder or a link named `.claude-<name>` or `.codex-<name>` exists. Each entry is looked at
//! by name and by the kind the listing reports; nothing is opened, `stat`ed or followed, so no
//! account folder is read. The reserved name `alz-blocked` never counts as existing, and nothing
//! alz creates is ever called that.

use std::collections::BTreeSet;
use std::fs::{self, FileType};

use alz_core::account::AccountName;
use alz_core::path::AbsolutePath;
use alz_core::target::TargetName;

/// The reserved name of the blocked folders, `~/.claude-alz-blocked` and `~/.codex-alz-blocked`.
const BLOCKED: &str = "alz-blocked";

/// The names of the account folders that exist, per target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountFolderNames {
    claude: BTreeSet<AccountName>,
    codex: BTreeSet<AccountName>,
}

impl AccountFolderNames {
    /// Lists `home` once. A `$HOME` that cannot be listed has no accounts.
    #[must_use]
    pub fn list(home: &AbsolutePath) -> Self {
        fs::read_dir(home.as_str())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(counts))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .fold(Self::default(), |names, entry| names.with_entry(&entry))
    }

    fn with_entry(mut self, entry: &str) -> Self {
        let named = |prefix: &str| {
            entry
                .strip_prefix(prefix)
                .filter(|name| *name != BLOCKED)
                .and_then(|name| AccountName::try_from(name).ok())
        };
        if let Some(name) = named(".claude-") {
            self.claude.insert(name);
        }
        if let Some(name) = named(".codex-") {
            self.codex.insert(name);
        }
        self
    }

    /// The names that exist for `target`, in name order.
    pub fn names(&self, target: TargetName) -> impl Iterator<Item = &AccountName> + '_ {
        match target {
            TargetName::Claude => self.claude.iter(),
            TargetName::Codex => self.codex.iter(),
        }
    }

    /// Whether the folder of `name` for `target` exists.
    #[must_use]
    pub fn exists(&self, target: TargetName, name: &AccountName) -> bool {
        match target {
            TargetName::Claude => self.claude.contains(name),
            TargetName::Codex => self.codex.contains(name),
        }
    }
}

/// A directory or a link.
fn counts(kind: FileType) -> bool {
    kind.is_dir() || kind.is_symlink()
}
