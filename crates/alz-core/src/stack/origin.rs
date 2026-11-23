//! What `git clone` is given for a `github:` or `git:` source, and how it is reached (spec §6
//! "Layers in or around an account folder", §7.12 "Who may declare it"; `docs/target-facts.md`
//! §11 "Clone argument is tried as a local path first"), decided without touching the file
//! system.
//!
//! `git clone` looks for its argument on disk, relative to the working directory, before it
//! reads it as a URL, so `x:y` can be a folder of the repository being built. alz therefore
//! asks, for each path git would try, whether it exists ([`Probe`]); the answers come from the
//! caller, which gives a canonical path for each one that does. Only when nothing exists is a
//! `file://` URL its absolute path and a `scheme://` or `host:path` argument a remote.

use crate::cache::CloneKey;
use crate::diagnostic::Diagnostic;
use crate::git::CloneTransport;
use crate::path::AbsolutePath;
use crate::stack::resolution_failure::{
    invalid_file_url, layer_touches_account_folder, local_git_path_has_dotdot,
};
use crate::stack::source::{GitHubName, GitUrl};
use crate::untrusted_text::Stripped;

/// The paths `git clone` tries for a local repository argument (`get_repo_path` in git's
/// `clone.c`), appended to the argument, in its order.
const SUFFIXES: [&str; 5] = ["/.git", "", ".git/.git", ".git", ".bundle"];

/// What `git clone` is given, and how it gets there. For a local source the argument is the
/// canonical absolute path, never the URL as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneOrigin {
    argument: String,
    transport: CloneTransport,
}

impl CloneOrigin {
    /// The text passed to `git clone`, after `--`.
    #[must_use]
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// Whether the clone is guarded (a remote) or not (a path on this machine).
    #[must_use]
    pub const fn transport(&self) -> CloneTransport {
        self.transport
    }

    /// The key of the cache entry: the SHA-256 of the argument as it is passed.
    #[must_use]
    pub fn key(&self) -> CloneKey {
        CloneKey::of(&self.argument)
    }

    fn remote(argument: &str) -> Self {
        Self {
            argument: argument.to_owned(),
            transport: CloneTransport::Remote,
        }
    }

    fn local(path: &AbsolutePath) -> Self {
        Self {
            argument: path.as_str().to_owned(),
            transport: CloneTransport::Local,
        }
    }
}

/// The origin of `github:<owner>/<repo>`: `https://github.com/<owner>/<repo>.git`, a remote.
#[must_use]
pub fn github_origin(owner: &GitHubName, repo: &GitHubName) -> CloneOrigin {
    CloneOrigin::remote(&format!(
        "https://github.com/{}/{}.git",
        owner.as_str(),
        repo.as_str()
    ))
}

/// The caller's answer to "does a path of this name exist?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// There is nothing at that path.
    Absent,
    /// There is something, and this is its canonical path.
    Found(AbsolutePath),
    /// The path touches an account folder or the home folder (§9.1), so nothing was looked at.
    Refused,
}

/// Why a `git:` source cannot be classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginRefusal {
    /// A local path with a `..` segment: text and disk can disagree about where it leads.
    DotDot,
    /// A `file://` URL that does not name a path.
    InvalidFileUrl,
    /// A path git would read touches an account folder or the home folder.
    AccountFolder,
}

impl OriginRefusal {
    /// The error `source`, naming `source`.
    #[must_use]
    pub fn into_diagnostic(self, source: Stripped) -> Diagnostic {
        match self {
            Self::DotDot => local_git_path_has_dotdot(source),
            Self::InvalidFileUrl => invalid_file_url(source),
            Self::AccountFolder => layer_touches_account_folder(source),
        }
    }
}

fn has_dot_dot(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

/// `decodeURIComponent`: each `%XX` is a byte, and the bytes must be UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let mut parts = text.split('%');
    let first = parts.next()?.as_bytes().to_vec();
    parts
        .try_fold(first, |mut decoded, part| {
            let hex = part
                .get(..2)
                .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            decoded.extend_from_slice(part.get(2..)?.as_bytes());
            Some(decoded)
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// The path of a `file://` URL as written, with no normalisation, so `..` stays visible. The
/// host, if any, is dropped.
fn file_url_path(url: &str) -> Option<AbsolutePath> {
    let rest = url.strip_prefix("file://")?;
    let path = if rest.starts_with('/') {
        rest
    } else {
        rest.get(rest.find('/')?..)?
    };
    percent_decode(path).and_then(|decoded| AbsolutePath::try_from(decoded.as_str()).ok())
}

/// `base` and `relative` joined and normalised lexically; `relative` holds no `..`.
fn resolve(base: &AbsolutePath, relative: &str) -> Option<AbsolutePath> {
    let parts = base
        .components()
        .chain(relative.split('/'))
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    AbsolutePath::try_from(format!("/{}", parts.join("/")).as_str()).ok()
}

/// The canonical path of every path git would try for `base` that exists, in git's order.
fn found_on_disk(
    base: &str,
    probe: &mut impl FnMut(&AbsolutePath) -> Probe,
) -> Result<Vec<AbsolutePath>, OriginRefusal> {
    SUFFIXES
        .iter()
        .filter_map(|suffix| AbsolutePath::try_from(format!("{base}{suffix}").as_str()).ok())
        .map(|candidate| probe(&candidate))
        .try_fold(Vec::new(), |mut found, answer| match answer {
            Probe::Absent => Ok(found),
            Probe::Found(path) => {
                found.push(path);
                Ok(found)
            }
            Probe::Refused => Err(OriginRefusal::AccountFolder),
        })
}

/// Classifies the URL of a `git:` source, in this order:
/// 1. a path of that name exists relative to `repo_root` (as given or with git's suffixes): it is
///    local, whatever it looks like, unless it holds a `..` segment, which is refused;
/// 2. a `file://` URL is its absolute path (refused with a `..` segment or when it names no
///    path), canonical when it exists;
/// 3. a `scheme://` or `host:path` argument is a remote, cloned as written;
/// 4. anything else is a local path relative to `repo_root`, refused with a `..` segment.
///
/// `probe` is asked about each path git would try, before any other; it is the caller's lookup
/// on the real file system, which resolves `..` through links exactly as git will.
///
/// # Errors
///
/// An [`OriginRefusal`] for a `..` segment in a local path, a `file://` URL without a path, or a
/// path that the probe refused.
pub fn locate_git(
    url: &GitUrl,
    repo_root: &AbsolutePath,
    probe: &mut impl FnMut(&AbsolutePath) -> Probe,
) -> Result<CloneOrigin, OriginRefusal> {
    let text = url.as_str();
    let raw = if text.starts_with('/') {
        text.to_owned()
    } else {
        format!("{}/{text}", repo_root.as_str())
    };
    let found = found_on_disk(&raw, probe)?;
    if let Some(first) = found.first() {
        return if has_dot_dot(text) {
            Err(OriginRefusal::DotDot)
        } else {
            Ok(CloneOrigin::local(first))
        };
    }
    if text.starts_with("file://") {
        let path = file_url_path(text).ok_or(OriginRefusal::InvalidFileUrl)?;
        if has_dot_dot(path.as_str()) {
            return Err(OriginRefusal::DotDot);
        }
        return Ok(CloneOrigin::local(
            found_on_disk(path.as_str(), probe)?
                .first()
                .unwrap_or(&path),
        ));
    }
    if text
        .split('/')
        .next()
        .is_some_and(|head| head.contains(':'))
    {
        return Ok(CloneOrigin::remote(text));
    }
    if has_dot_dot(text) {
        return Err(OriginRefusal::DotDot);
    }
    resolve(repo_root, text)
        .map(|path| CloneOrigin::local(&path))
        .ok_or(OriginRefusal::InvalidFileUrl)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{CloneOrigin, OriginRefusal, Probe, github_origin, locate_git};
    use crate::git::CloneTransport;
    use crate::path::AbsolutePath;
    use crate::stack::source::{GitHubName, GitUrl};

    fn absolute(text: &str) -> AbsolutePath {
        AbsolutePath::try_from(text).expect("absolute")
    }

    /// A file system of `existing` paths, each with the canonical path it answers; `refused`
    /// paths answer `Refused`. Records what was asked.
    fn locate(
        url: &str,
        existing: &[(&str, &str)],
        refused: &[&str],
    ) -> (Result<CloneOrigin, OriginRefusal>, Vec<String>) {
        let files: BTreeMap<&str, &str> = existing.iter().copied().collect();
        let mut asked = Vec::new();
        let url = GitUrl::try_from(url).expect("a url");
        let result = locate_git(&url, &absolute("/repo"), &mut |path| {
            asked.push(path.to_string());
            if refused.contains(&path.as_str()) {
                Probe::Refused
            } else {
                files
                    .get(path.as_str())
                    .map_or(Probe::Absent, |real| Probe::Found(absolute(real)))
            }
        });
        (result, asked)
    }

    fn argument(result: Result<CloneOrigin, OriginRefusal>) -> (String, CloneTransport) {
        let origin = result.expect("an origin");
        (origin.argument().to_owned(), origin.transport())
    }

    #[test]
    fn a_github_source_is_cloned_from_https_and_is_a_remote() {
        let origin = github_origin(
            &GitHubName::try_from("owner").expect("a name"),
            &GitHubName::try_from("repo").expect("a name"),
        );
        assert_eq!(origin.argument(), "https://github.com/owner/repo.git");
        assert_eq!(origin.transport(), CloneTransport::Remote);
    }

    #[test]
    fn a_path_that_exists_wins_over_how_the_url_reads_and_its_canonical_path_is_cloned() {
        let (result, asked) = locate("x:y", &[("/repo/x:y", "/real/x:y")], &[]);
        assert_eq!(
            argument(result),
            ("/real/x:y".to_owned(), CloneTransport::Local)
        );
        assert_eq!(
            asked,
            [
                "/repo/x:y/.git",
                "/repo/x:y",
                "/repo/x:y.git/.git",
                "/repo/x:y.git",
                "/repo/x:y.bundle"
            ]
        );
    }

    #[test]
    fn git_s_suffixes_are_tried_in_its_order_and_the_first_found_is_cloned() {
        let (result, _) = locate(
            "hub",
            &[
                ("/repo/hub.git", "/real/hub.git"),
                ("/repo/hub", "/real/hub"),
            ],
            &[],
        );
        assert_eq!(argument(result).0, "/real/hub");
    }

    #[test]
    fn an_absolute_path_is_looked_up_as_it_is() {
        let (result, asked) = locate("/srv/hub", &[("/srv/hub", "/srv/hub")], &[]);
        assert_eq!(
            argument(result),
            ("/srv/hub".to_owned(), CloneTransport::Local)
        );
        assert_eq!(asked.first().map(String::as_str), Some("/srv/hub/.git"));
    }

    #[test]
    fn a_found_local_path_with_a_dot_dot_segment_is_refused() {
        let (result, _) = locate("../hub", &[("/repo/../hub", "/hub")], &[]);
        assert_eq!(result, Err(OriginRefusal::DotDot));
    }

    #[test]
    fn a_file_url_is_its_absolute_path_and_canonical_when_it_exists() {
        let (result, _) = locate(
            "file:///srv/hub.git",
            &[("/srv/hub.git", "/real/hub.git")],
            &[],
        );
        assert_eq!(
            argument(result),
            ("/real/hub.git".to_owned(), CloneTransport::Local)
        );
        let (result, _) = locate("file:///nowhere/r", &[], &[]);
        assert_eq!(
            argument(result),
            ("/nowhere/r".to_owned(), CloneTransport::Local)
        );
        let (result, _) = locate("file://localhost/a%20b/r", &[], &[]);
        assert_eq!(argument(result).0, "/a b/r");
    }

    #[test]
    fn a_file_url_with_a_dot_dot_segment_or_no_path_is_refused() {
        assert_eq!(
            locate("file:///a/../b", &[], &[]).0,
            Err(OriginRefusal::DotDot)
        );
        assert_eq!(
            locate("file:///a/%2e%2e/b", &[], &[]).0,
            Err(OriginRefusal::DotDot)
        );
        for bad in ["file://host", "file:///a%zz", "file:///a%ff"] {
            assert_eq!(
                locate(bad, &[], &[]).0,
                Err(OriginRefusal::InvalidFileUrl),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_scheme_or_host_path_that_is_not_on_disk_is_a_remote_cloned_as_written() {
        for url in [
            "https://example.com/o/r.git",
            "ssh://git@example.com/o/r.git",
            "git@example.com:o/r.git",
            "example.com:o/r",
        ] {
            let (result, _) = locate(url, &[], &[]);
            assert_eq!(
                argument(result),
                (url.to_owned(), CloneTransport::Remote),
                "{url}"
            );
        }
    }

    #[test]
    fn any_other_text_is_a_local_path_relative_to_the_repository() {
        let (result, _) = locate("./sub/../x", &[], &[]);
        assert_eq!(result, Err(OriginRefusal::DotDot));
        let (result, _) = locate("./sub/x", &[], &[]);
        assert_eq!(
            argument(result),
            ("/repo/sub/x".to_owned(), CloneTransport::Local)
        );
    }

    #[test]
    fn a_path_the_probe_refuses_stops_the_classification() {
        let (result, _) = locate("hub", &[("/repo/hub", "/real/hub")], &["/repo/hub.git"]);
        assert_eq!(result, Err(OriginRefusal::AccountFolder));
    }

    #[test]
    fn the_key_is_the_hash_of_the_argument_as_passed() {
        let (result, _) = locate("https://example.com/o/r.git", &[], &[]);
        assert_eq!(
            result.expect("an origin").key(),
            crate::cache::CloneKey::of("https://example.com/o/r.git")
        );
    }
}
