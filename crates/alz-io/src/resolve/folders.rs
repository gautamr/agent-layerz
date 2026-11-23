//! The account folders of this machine (spec §9.1), as `alz-core`'s comparison needs them: each
//! folder as written and as its real path.

use std::ffi::OsStr;

use alz_core::account::AccountFolders;
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

/// The account folders for `HOME`, `CLAUDE_CONFIG_DIR` and `CODEX_HOME` of `env`, each also by
/// its real path (`/var` is a link on macOS). A value that is empty or relative has no meaning
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
        .fold(with_home, |folders, real| {
            folders.with_real_configured(&real)
        })
}
