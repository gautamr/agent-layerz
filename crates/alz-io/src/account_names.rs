//! The Claude account folders and the names they hold, for `alz doctor` (spec §8.4, §9.1).
//!
//! The only reads are the ones spec §9.1 allows: listing `$HOME` for names that start with
//! `.claude-`, listing `agents/` and `skills/` for the names they hold, and asking whether
//! `CLAUDE.md` exists with `lstat`. No file in an account folder is opened, so nothing about a
//! login or a trusted repository can be learned there.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use alz_core::Stripped;
use alz_core::doctor::AccountFolder;

use crate::env::EnvSnapshot;

/// The name of the folder `CLAUDE_CONFIG_DIR` is not set for.
const DEFAULT_FOLDER: &str = ".claude";
/// What the folders of other accounts start with.
const ACCOUNT_PREFIX: &str = ".claude-";

fn absolute(value: Option<&OsStr>) -> Option<PathBuf> {
    value
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
}

/// The names of the entries of `folder` that `wanted` turns into a name; none when it cannot be
/// listed.
fn names_in(
    folder: &Path,
    wanted: impl Fn(fs::FileType, &str) -> Option<String>,
) -> BTreeSet<String> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            let name = entry.file_name().into_string().ok()?;
            wanted(kind, &name)
        })
        .collect()
}

/// `agents/<name>.md` files, as `<name>`. A link is not a file here.
fn agent_name(kind: fs::FileType, name: &str) -> Option<String> {
    kind.is_file()
        .then(|| name.strip_suffix(".md"))
        .flatten()
        .map(str::to_owned)
}

/// `skills/<name>`: a folder or a link.
fn skill_name(kind: fs::FileType, name: &str) -> Option<String> {
    (kind.is_dir() || kind.is_symlink()).then(|| name.to_owned())
}

fn observe(folder: &Path) -> AccountFolder {
    AccountFolder {
        path: Stripped::from_untrusted(&folder.to_string_lossy()),
        agents: names_in(&folder.join("agents"), agent_name),
        skills: names_in(&folder.join("skills"), skill_name),
        has_claude_md: fs::symlink_metadata(folder.join("CLAUDE.md")).is_ok(),
    }
}

/// The candidates in report order: `CLAUDE_CONFIG_DIR`, `~/.claude`, then each `~/.claude-*` in
/// name order.
fn candidates(env: &EnvSnapshot) -> Vec<PathBuf> {
    let configured = absolute(env.claude_config_dir());
    let home = absolute(env.home());
    let default = home.iter().map(|home| home.join(DEFAULT_FOLDER));
    let others = home.iter().flat_map(|home| {
        let named: BTreeSet<String> = fs::read_dir(home)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.to_lowercase().starts_with(ACCOUNT_PREFIX))
            .collect();
        named.into_iter().map(|name| home.join(name))
    });
    let all = configured.into_iter().chain(default).chain(others);
    all.fold(Vec::new(), |mut unique, path| {
        if !unique.contains(&path) {
            unique.push(path);
        }
        unique
    })
}

/// The Claude account folders that exist, each with the names it holds.
#[must_use]
pub fn claude_account_folders(env: &EnvSnapshot) -> Vec<AccountFolder> {
    candidates(env)
        .iter()
        .filter(|folder| fs::metadata(folder).is_ok_and(|metadata| metadata.is_dir()))
        .map(|folder| observe(folder))
        .collect()
}
