//! The account folders of this machine (spec §9.1), as `alz-core`'s comparison needs them: each
//! folder as written and as its real path.

use std::ffi::OsStr;

use alz_core::account::{AccountFolders, names_account_folder};
use alz_core::path::AbsolutePath;

use crate::EnvSnapshot;

/// The real path of `path`, with every link resolved. Only the names on the way are looked at,
/// never what a folder holds.
pub(super) fn real_path(path: &AbsolutePath) -> Option<AbsolutePath> {
    std::fs::canonicalize(path.as_str()).ok().and_then(|real| {
        real.to_str()
            .and_then(|text| AbsolutePath::try_from(text).ok())
    })
}

fn absolute(value: Option<&OsStr>) -> Option<AbsolutePath> {
    value
        .and_then(OsStr::to_str)
        .and_then(|text| AbsolutePath::try_from(text).ok())
}

/// The real path of every named account entry of `home` that is a link: `~/.claude`,
/// `~/.claude.json`, `~/.codex`, `~/.claude-*` and `~/.codex-*`. Only the entries of `home` are
/// listed, each is looked at with `lstat` and a link is resolved by its names; nothing inside an
/// account folder is opened (spec §9.1). A folder that is reached through such a link is the
/// account folder, whatever its own path says.
fn linked_account_folders(home: &AbsolutePath) -> Vec<AbsolutePath> {
    std::fs::read_dir(home.as_str())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_name().to_str().is_some_and(names_account_folder))
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_symlink()))
        .filter_map(|entry| {
            entry
                .path()
                .to_str()
                .and_then(|text| AbsolutePath::try_from(text).ok())
        })
        .filter_map(|link| real_path(&link))
        .collect()
}

/// The account folders for `HOME`, `CLAUDE_CONFIG_DIR` and `CODEX_HOME` of `env`, each also by
/// its real path (`/var` is a link on macOS), and the real path of each account entry of `HOME`
/// that is a link, so a folder reached through such a link is refused. A value that is empty or relative has no meaning
/// here and is left out; a `HOME` that is not an absolute path leaves `/` as the home folder, so
/// no folder is known by name and only the configured ones count.
pub(super) fn account_folders(env: &EnvSnapshot) -> AccountFolders {
    let home = absolute(env.home()).unwrap_or_else(AbsolutePath::root);
    let claude = absolute(env.claude_config_dir());
    let codex = absolute(env.codex_home());
    let named = AccountFolders::from_env(&home, claude.as_ref(), codex.as_ref());
    let with_home =
        real_path(&home).map_or_else(|| named.clone(), |real| named.clone().with_real_home(&real));
    [claude, codex]
        .into_iter()
        .flatten()
        .filter_map(|folder| real_path(&folder))
        .chain(linked_account_folders(&home))
        .fold(with_home, |folders, real| {
            folders.with_real_configured(&real)
        })
}
