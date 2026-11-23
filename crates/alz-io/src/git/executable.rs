//! Finding git once, as an absolute path (spec §6 "Git subprocesses", "Which git").
//!
//! The runner looks `git` up in the `PATH` of the environment snapshot, a folder at a time, and
//! uses the first regular file that anyone may execute. An entry that is empty or relative names
//! the working directory, which can be a repository that alz is about to read; such an entry is
//! skipped. The path that results is passed to the copy that runs git, and the version check runs
//! on that same path, so the git that was checked is the git that runs.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The folders of `path`, a `:`-separated list, as the candidates for `git`: the absolute entries
/// only, in order, each joined with `git`.
fn candidates(path: &OsStr) -> impl Iterator<Item = PathBuf> + '_ {
    path.as_bytes()
        .split(|byte| *byte == b':')
        .map(|entry| Path::new(OsStr::from_bytes(entry)))
        .filter(|entry| entry.is_absolute())
        .map(|entry| entry.join("git"))
}

/// The first candidate that `is_git` accepts.
fn first_git(path: &OsStr, is_git: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    candidates(path).find(|candidate| is_git(candidate))
}

/// Whether `candidate` is a regular file, after links, that has an execute bit.
fn is_executable_file(candidate: &Path) -> bool {
    fs::metadata(candidate)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// The absolute path of the `git` that `path` leads to, or `None` when it leads to none.
pub(super) fn find_git(path: Option<&OsStr>) -> Option<PathBuf> {
    path.and_then(|path| first_git(path, is_executable_file))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    use super::{candidates, first_git};

    #[test]
    fn only_absolute_entries_are_candidates_in_order() {
        let found: Vec<PathBuf> =
            candidates(OsStr::new(":.:bin::/usr/local/bin:relative/x:/usr/bin:")).collect();
        assert_eq!(
            found,
            [
                PathBuf::from("/usr/local/bin/git"),
                PathBuf::from("/usr/bin/git")
            ]
        );
    }

    #[test]
    fn an_empty_path_has_no_candidate() {
        assert_eq!(candidates(OsStr::new("")).count(), 0);
        assert_eq!(candidates(OsStr::new("::")).count(), 0);
    }

    #[test]
    fn a_git_in_the_working_directory_is_never_taken_even_when_it_would_be_first() {
        let everything = |_: &Path| true;
        assert_eq!(
            first_git(OsStr::new(".:/usr/bin"), everything),
            Some(PathBuf::from("/usr/bin/git"))
        );
        assert_eq!(first_git(OsStr::new(".:bin:"), everything), None);
    }

    #[test]
    fn the_first_absolute_candidate_that_is_a_git_wins() {
        let only_second = |candidate: &Path| candidate == Path::new("/b/git");
        assert_eq!(
            first_git(OsStr::new("/a:/b:/c"), only_second),
            Some(PathBuf::from("/b/git"))
        );
    }
}
