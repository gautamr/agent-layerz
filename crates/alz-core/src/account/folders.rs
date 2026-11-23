//! Which paths touch an account folder (spec §9.1, TASK-52). A pure comparison of path text: it
//! opens nothing, so asking the question never reads an account. Each component is compared in
//! Unicode NFC and lowercased (decision-37), so a case or form variant of `~/.claude-x`, which
//! APFS treats as the same folder, counts as the folder.

use std::fmt;

use unicode_normalization::UnicodeNormalization;

use crate::path::AbsolutePath;

/// How a path relates to the account folders. When several apply, the first of these in this
/// order is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Relation {
    /// At or below `~/.claude`, `~/.claude.json`, `~/.codex`, `~/.claude-*` or `~/.codex-*`.
    AccountFolder,
    /// At or below a folder that `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names.
    Configured,
    /// `$HOME` itself, or above it.
    Home,
    /// Holds a folder that `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names.
    Holds,
}

impl Relation {
    /// Whether a name below a path with this relation is inside an account folder. A path that
    /// is only `$HOME`, above it or above a configured folder can still be looked at; what it
    /// holds cannot be opened (spec §6 "Layers in or around an account folder").
    #[must_use]
    pub const fn is_inside(self) -> bool {
        match self {
            Self::AccountFolder | Self::Configured => true,
            Self::Home | Self::Holds => false,
        }
    }
}

impl fmt::Display for Relation {
    /// The reason, never the path.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AccountFolder => "inside an account folder",
            Self::Configured => "inside a folder that CLAUDE_CONFIG_DIR or CODEX_HOME names",
            Self::Home => "the home folder or above it",
            Self::Holds => "holds a folder that CLAUDE_CONFIG_DIR or CODEX_HOME names",
        })
    }
}

/// A path as its components, each normalised and lowercased, with `.` dropped and `..` applied.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Folded(Vec<String>);

impl Folded {
    fn of(path: &AbsolutePath) -> Self {
        Self(path.components().fold(Vec::new(), |mut parts, component| {
            match component {
                "." => {}
                ".." => {
                    parts.pop();
                }
                name => parts.push(fold(name)),
            }
            parts
        }))
    }

    /// Whether `self` is `base` or lies below it.
    fn is_within(&self, base: &Self) -> bool {
        self.0.starts_with(&base.0)
    }

    /// The first component of `self` below `base`, when `self` lies strictly below it.
    fn first_below(&self, base: &Self) -> Option<&str> {
        self.0
            .strip_prefix(base.0.as_slice())
            .and_then(<[String]>::first)
            .map(String::as_str)
    }
}

/// `NFC(lowercase(NFC(name)))`: the form in which names are compared.
fn fold(name: &str) -> String {
    name.nfc()
        .collect::<String>()
        .to_lowercase()
        .nfc()
        .collect()
}

/// Whether a component, already folded, names an account folder below `$HOME`.
fn names_account(folded: &str) -> bool {
    matches!(folded, ".claude" | ".codex" | ".claude.json")
        || folded.starts_with(".claude-")
        || folded.starts_with(".codex-")
}

/// Whether `path` is `base` or lies below it, compared per component in Unicode NFC and
/// lowercased, as the account folders are (decision-37). A caller that asks about a folder it
/// must keep out, such as the repository, gets the strict answer on a case-insensitive file
/// system.
#[must_use]
pub fn lies_within(path: &AbsolutePath, base: &AbsolutePath) -> bool {
    Folded::of(path).is_within(&Folded::of(base))
}

/// The account folders of one machine and one environment: `$HOME` and what is below it by name,
/// and the folders `CLAUDE_CONFIG_DIR` and `CODEX_HOME` name.
///
/// A folder can have two spellings, as written and as its real path (a link in the path, such as
/// `/var` on macOS). The caller resolves each once without looking inside it and adds the real
/// spelling with [`with_real_home`](Self::with_real_home) and
/// [`with_real_configured`](Self::with_real_configured).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFolders {
    homes: Vec<Folded>,
    configured: Vec<Folded>,
}

impl AccountFolders {
    /// The account folders for `HOME` and the two variables. A value that is empty, or that is
    /// relative and so has no meaning here, is the caller's to turn into `None`, or into an
    /// absolute path with the working directory, which this crate does not have.
    #[must_use]
    pub fn from_env(
        home: &AbsolutePath,
        claude_config_dir: Option<&AbsolutePath>,
        codex_home: Option<&AbsolutePath>,
    ) -> Self {
        Self {
            homes: vec![Folded::of(home)],
            configured: [claude_config_dir, codex_home]
                .into_iter()
                .flatten()
                .map(Folded::of)
                .collect(),
        }
    }

    /// Also treats `real` as `$HOME`: its real path.
    #[must_use]
    pub fn with_real_home(self, real: &AbsolutePath) -> Self {
        Self {
            homes: self.homes.into_iter().chain([Folded::of(real)]).collect(),
            ..self
        }
    }

    /// Also treats `real` as a configured folder: the real path of what `CLAUDE_CONFIG_DIR` or
    /// `CODEX_HOME` names.
    #[must_use]
    pub fn with_real_configured(self, real: &AbsolutePath) -> Self {
        Self {
            configured: self
                .configured
                .into_iter()
                .chain([Folded::of(real)])
                .collect(),
            ..self
        }
    }

    /// How `path` relates to the account folders, or `None` when it does not touch one.
    #[must_use]
    pub fn relation(&self, path: &AbsolutePath) -> Option<Relation> {
        let target = Folded::of(path);
        let in_account_folder = self
            .homes
            .iter()
            .any(|home| target.first_below(home).is_some_and(names_account));
        if in_account_folder {
            Some(Relation::AccountFolder)
        } else if self
            .configured
            .iter()
            .any(|folder| target.is_within(folder))
        {
            Some(Relation::Configured)
        } else if self.homes.iter().any(|home| home.is_within(&target)) {
            Some(Relation::Home)
        } else if self
            .configured
            .iter()
            .any(|folder| folder.is_within(&target))
        {
            Some(Relation::Holds)
        } else {
            None
        }
    }

    /// Whether `path` is an account folder or lies inside one, is `$HOME` or above it, or holds
    /// one.
    #[must_use]
    pub fn touches(&self, path: &AbsolutePath) -> bool {
        self.relation(path).is_some()
    }
}

#[cfg(test)]
mod tests;
