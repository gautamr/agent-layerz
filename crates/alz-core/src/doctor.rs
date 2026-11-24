//! `alz doctor` (spec §8.4, §10.3): what the report says, decided from facts that the caller
//! observed. Nothing here touches the file system or runs a program.
//!
//! The facts are plain records: every combination of their fields is a legal observation. A name
//! or a path in a fact is untrusted text and arrives [`Stripped`]; the one name that is compared
//! with a built name is kept as it is, and is printed only when it equals a built name.

use std::collections::BTreeSet;
use std::fmt;

use crate::diagnostic::Diagnostics;
use crate::live::Built;
use crate::target::claude::facts::{VERIFIED_RANGE, Version};
use crate::untrusted_text::Stripped;

/// A Claude account folder, read for names only (spec §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFolder {
    /// The folder.
    pub path: Stripped,
    /// The names of the agent files in `agents/`, without `.md`.
    pub agents: BTreeSet<String>,
    /// The names of the folders and links in `skills/`.
    pub skills: BTreeSet<String>,
    /// Whether `CLAUDE.md` exists.
    pub has_claude_md: bool,
}

/// What `<bin> --version` gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// No program, a failed call, or output without a version.
    NotFound,
    /// The version it printed.
    Version(Version),
}

/// What is known about Claude Code on this machine, for a repository that builds for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeFacts {
    /// The account folders that exist, in the order they are reported.
    pub accounts: Vec<AccountFolder>,
    /// The installed version.
    pub installed: Installed,
    /// The names the build makes; empty when the build has an error.
    pub built: Built,
}

/// The folder above the repositories of a hub (spec §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRoot {
    /// The folder.
    pub path: Stripped,
    /// Whether it holds a `CLAUDE.md`.
    pub has_claude_md: bool,
    /// Whether it holds an `AGENTS.md`.
    pub has_agents_md: bool,
    /// Whether it holds a `.git`.
    pub is_git_repository: bool,
}

/// What a workspace directory is on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceState {
    /// There is no such folder.
    Missing,
    /// It exists; `loaded` names the folders of its `.claude` that exist, among `skills`,
    /// `agents` and `commands`, in that order.
    Present {
        /// The kinds Claude would load if the folder were passed with `--add-dir`.
        loaded: Vec<&'static str>,
    },
}

/// One `workspace.add_dirs` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFolder {
    /// As written in the manifest.
    pub declared: Stripped,
    /// Resolved against the repository.
    pub resolved: Stripped,
    /// What is there.
    pub state: WorkspaceState,
}

/// A folder above the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ancestor {
    /// The folder.
    pub path: Stripped,
    /// Whether it holds a `CLAUDE.md`.
    pub has_claude_md: bool,
    /// Whether it is a git repository that also holds a `.claude`.
    pub is_git_repository_with_claude: bool,
}

/// Everything the report is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The repository's root.
    pub root: Stripped,
    /// Present when the stack builds for Claude.
    pub claude: Option<ClaudeFacts>,
    /// Present when the manifest names a hub.
    pub client_root: Option<ClientRoot>,
    /// The `workspace.add_dirs`.
    pub workspace: Vec<WorkspaceFolder>,
    /// The folders above the repository except the client root, nearest first.
    pub ancestors: Vec<Ancestor>,
    /// The blocked folders (spec §7.13) that exist, `~/.claude-alz-blocked` first. They are not
    /// account folders, whatever the targets.
    pub blocked: Vec<Stripped>,
    /// What the build found.
    pub build: Diagnostics,
}

/// The report: its lines and its problems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    lines: Vec<String>,
    problems: usize,
}

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn collisions(folder: &AccountFolder, built: &Built) -> Vec<String> {
    let path = folder.path.as_str();
    let skills = folder.skills.intersection(built.skills()).map(|name| {
        format!(
            "collision: skill '{}' also exists in {path}/skills (Claude: a user skill beats the project one, silently)",
            Stripped::from_untrusted(name)
        )
    });
    let agents = folder.agents.intersection(built.agents()).map(|name| {
        format!(
            "collision: agent '{}' also exists in {path}/agents (move it into a layer)",
            Stripped::from_untrusted(name)
        )
    });
    skills.chain(agents).collect()
}

/// The lines of one account folder and the number of collisions in it.
fn account_lines(folder: &AccountFolder, built: &Built) -> (Vec<String>, usize) {
    let found = collisions(folder, built);
    let count = found.len();
    let heading = format!(
        "  {}: {}",
        folder.path,
        if count == 0 {
            "no collisions".to_owned()
        } else {
            plural(count, "collision")
        }
    );
    let note = folder.has_claude_md.then(|| {
        format!(
            "    note: {}/CLAUDE.md is loaded into every session as well",
            folder.path
        )
    });
    let lines = std::iter::once(heading)
        .chain(found.into_iter().map(|line| format!("    {line}")))
        .chain(note)
        .collect();
    (lines, count)
}

fn version_line(installed: Installed) -> String {
    match installed {
        Installed::NotFound => "claude: not found; version check skipped".to_owned(),
        Installed::Version(version) if VERIFIED_RANGE.contains(version) => {
            format!("claude: {version} (verified range {VERIFIED_RANGE})")
        }
        Installed::Version(version) => format!(
            "warning: claude {version} is outside the verified range {VERIFIED_RANGE}; re-check docs/target-facts.md"
        ),
    }
}

/// The lines of the claude section and its collisions.
fn claude_lines(claude: &ClaudeFacts) -> (Vec<String>, usize) {
    let accounts: Vec<(Vec<String>, usize)> = claude
        .accounts
        .iter()
        .map(|folder| account_lines(folder, &claude.built))
        .collect();
    let problems = accounts.iter().map(|(_, count)| count).sum();
    let listed = if accounts.is_empty() {
        vec!["  none found".to_owned()]
    } else {
        accounts.into_iter().flat_map(|(lines, _)| lines).collect()
    };
    let lines = std::iter::once("claude account dirs:".to_owned())
        .chain(listed)
        .chain([version_line(claude.installed)])
        .collect();
    (lines, problems)
}

fn blocked_line(folder: &Stripped) -> String {
    format!("warning: {folder} is a blocked folder (§7.13), not an account; delete it")
}

fn client_root_lines(root: &ClientRoot) -> Vec<String> {
    let files = [
        (root.has_claude_md, "CLAUDE.md"),
        (root.has_agents_md, "AGENTS.md"),
    ]
    .into_iter()
    .filter(|(exists, _)| *exists)
    .map(|(_, file)| {
        format!(
            "warning: client root {}/{file} exists; remove it (put client content in the hub layer)",
            root.path
        )
    });
    let git = root.is_git_repository.then(|| {
        format!(
            "warning: client root {} is a git repo; it should not be (its .claude/ and AGENTS.md would load in every repo)",
            root.path
        )
    });
    files.chain(git).collect()
}

fn workspace_line(folder: &WorkspaceFolder) -> Option<String> {
    match &folder.state {
        WorkspaceState::Missing => Some(format!(
            "warning: workspace dir {} does not exist ({})",
            folder.declared, folder.resolved
        )),
        WorkspaceState::Present { loaded } if loaded.is_empty() => None,
        WorkspaceState::Present { loaded } => Some(format!(
            "note: {}/.claude has {}; they load only if you pass --add-dir {} yourself",
            folder.declared,
            loaded.join(", "),
            folder.declared
        )),
    }
}

fn ancestor_lines(ancestor: &Ancestor) -> Vec<String> {
    let claude_md = ancestor.has_claude_md.then(|| {
        format!(
            "warning: {}/CLAUDE.md is loaded too (ancestor directory)",
            ancestor.path
        )
    });
    let git = ancestor.is_git_repository_with_claude.then(|| {
        format!(
            "warning: {} is a git repo with .claude/; its skills or agents may load in this repo",
            ancestor.path
        )
    });
    claude_md.into_iter().chain(git).collect()
}

impl Report {
    /// The report for `facts`. A problem is a collision or a build error.
    #[must_use]
    pub fn of(facts: &Facts) -> Self {
        let (claude, collision_count) = facts.claude.as_ref().map_or((Vec::new(), 0), claude_lines);
        let build = facts.build.error_count();
        let lines: Vec<String> = std::iter::once(format!("agent-layerz doctor: {}", facts.root))
            .chain(claude)
            .chain(facts.blocked.iter().map(blocked_line))
            .chain(facts.client_root.iter().flat_map(client_root_lines))
            .chain(facts.workspace.iter().filter_map(workspace_line))
            .chain(facts.ancestors.iter().flat_map(ancestor_lines))
            .chain(std::iter::once(format!(
                "build: {}, {}",
                plural(build, "error"),
                plural(facts.build.warning_count(), "warning")
            )))
            .chain(facts.build.iter().map(|item| format!("  {item}")))
            .collect();
        let problems = collision_count + build;
        Self { lines, problems }
    }

    /// How many problems the report found: collisions and build errors.
    #[must_use]
    pub const fn problems(&self) -> usize {
        self.problems
    }
}

impl fmt::Display for Report {
    /// Each line, then `doctor: ok` or `doctor: <n> problems`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.lines
            .iter()
            .try_for_each(|line| writeln!(formatter, "{line}"))?;
        if self.problems == 0 {
            writeln!(formatter, "doctor: ok")
        } else {
            writeln!(formatter, "doctor: {}", plural(self.problems, "problem"))
        }
    }
}

#[cfg(test)]
mod tests;
