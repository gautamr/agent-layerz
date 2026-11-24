//! Finding `claude` as an absolute path (spec §10.3 `alz ls --live`).
//!
//! `ALZ_CLAUDE_BIN` names the program: an absolute path is used as it is, a bare name is looked up
//! on `PATH`, and anything else (a relative path) names nothing. Without it the name is `claude`.
//! A lookup takes the first regular file with an execute bit among the absolute entries of
//! `PATH`; an empty or relative entry names the working directory, which may be a repository that
//! alz is about to read, and is skipped.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::env::EnvSnapshot;

/// The program when `ALZ_CLAUDE_BIN` is not set.
const DEFAULT_NAME: &str = "claude";

/// The folders of `path`, a `:`-separated list: the absolute entries only, in order.
pub(super) fn absolute_entries(path: &OsStr) -> impl Iterator<Item = &Path> + '_ {
    path.as_bytes()
        .split(|byte| *byte == b':')
        .map(|entry| Path::new(OsStr::from_bytes(entry)))
        .filter(|entry| entry.is_absolute())
}

fn is_executable_file(candidate: &Path) -> bool {
    fs::metadata(candidate)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// The first candidate `name` in the absolute entries of `path` that `accepts`.
fn lookup(path: &OsStr, name: &OsStr, accepts: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    absolute_entries(path)
        .map(|entry| entry.join(name))
        .find(|candidate| accepts(candidate))
}

fn requested(
    requested: &OsStr,
    path: Option<&OsStr>,
    accepts: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let program = Path::new(requested);
    if program.is_absolute() {
        accepts(program).then(|| program.to_path_buf())
    } else if requested.is_empty() || requested.as_bytes().contains(&b'/') {
        None
    } else {
        path.and_then(|path| lookup(path, requested, accepts))
    }
}

/// The absolute path of the program, or `None` when there is none.
pub(super) fn locate(env: &EnvSnapshot) -> Option<PathBuf> {
    let name = env
        .vars()
        .get("ALZ_CLAUDE_BIN")
        .unwrap_or_else(|| OsStr::new(DEFAULT_NAME));
    requested(name, env.vars().get("PATH"), is_executable_file)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    use super::{lookup, requested};

    const EVERYTHING: fn(&Path) -> bool = |_| true;

    #[test]
    fn only_absolute_entries_are_searched_in_order() {
        let only_second = |candidate: &Path| candidate == Path::new("/b/claude");
        assert_eq!(
            lookup(
                OsStr::new(".:/a::bin:/b:/c"),
                OsStr::new("claude"),
                only_second
            ),
            Some(PathBuf::from("/b/claude"))
        );
        assert_eq!(
            lookup(OsStr::new(".:bin:"), OsStr::new("claude"), EVERYTHING),
            None
        );
    }

    #[test]
    fn an_absolute_request_is_used_as_it_is_and_a_name_is_looked_up() {
        assert_eq!(
            requested(OsStr::new("/x/fake"), None, EVERYTHING),
            Some(PathBuf::from("/x/fake"))
        );
        assert_eq!(
            requested(OsStr::new("fake"), Some(OsStr::new("/p")), EVERYTHING),
            Some(PathBuf::from("/p/fake"))
        );
    }

    #[test]
    fn a_relative_or_empty_request_names_nothing() {
        for request in ["", "./fake", "bin/fake", "../fake"] {
            assert_eq!(
                requested(OsStr::new(request), Some(OsStr::new("/p")), EVERYTHING),
                None,
                "{request:?}"
            );
        }
    }
}
