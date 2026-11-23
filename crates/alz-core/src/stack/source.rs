//! Where a layer comes from (spec §5.2): `path:<dir>`, `github:<owner>/<repo>@<ref>` or
//! `git:<url>@<ref>`. A source prints back as the text it was read from, byte for byte, because
//! `layer.lock` and `allowed.yaml` store that text.

use std::fmt;

use thiserror::Error;

use crate::stack::git_ref::{GitRef, GitRefError};

/// Why a `source` is refused. No variant holds input: a source can hold a token, so no message
/// prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SourceError {
    /// A source that starts with none of `path:`, `github:` and `git:`.
    #[error("a source starts with path:, github: or git:")]
    UnknownScheme,
    /// `path:` with nothing after it.
    #[error("a path: source names a directory")]
    EmptyPath,
    /// A `github:` or `git:` source without `@<ref>`.
    #[error("a github: or git: source ends with @<ref>")]
    MissingRef,
    /// The ref after `@` is not a git ref.
    #[error("the ref of a source is not a git ref")]
    InvalidRef(GitRefError),
    /// A URL that starts with `-`, which git could read as an option.
    #[error("the URL of a source starts with '-'")]
    UrlStartsWithDash,
    /// A `github:` source that is not `<owner>/<repo>`.
    #[error("a github: source names <owner>/<repo>")]
    GitHubShape,
    /// A `git:` URL that holds credentials (§5.2 "Credentials in a source").
    #[error("a git: source holds credentials in its URL")]
    Credentials,
    /// A `git:` URL that names a transport helper, `<helper>::<address>` (§5.2 "Transports").
    #[error("the URL of a git: source names a transport helper")]
    TransportHelper,
}

/// The message of a `git:` source that holds credentials (§5.2). It never prints the source.
pub(crate) const CREDENTIALS_MESSAGE: &str = "a git: source in layer.yaml holds credentials in its URL; remove them and use a git credential helper";

impl SourceError {
    /// The rule, as the end of a sentence about the field. Credentials have a message of their
    /// own (§5.2), which the schema error uses instead.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::UnknownScheme => "must start with path:, github: or git:",
            Self::EmptyPath => "must name a directory after path:",
            Self::MissingRef => "must end with @<ref>: <url>@<tag or commit>",
            Self::InvalidRef(error) => error.rule(),
            Self::UrlStartsWithDash => "must have a URL that does not start with '-'",
            Self::GitHubShape => "must be github:<owner>/<repo>@<ref>",
            Self::Credentials => CREDENTIALS_MESSAGE,
            Self::TransportHelper => {
                "must have a URL that is not a transport helper (<helper>::<address>)"
            }
        }
    }
}

impl From<GitRefError> for SourceError {
    fn from(error: GitRefError) -> Self {
        Self::InvalidRef(error)
    }
}

/// The directory of a `path:` source as written: non-empty. Which directory it is, and whether it
/// may be used, is decided when the source is resolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceDir(String);

impl SourceDir {
    /// The directory as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The URL of a `git:` source as written: non-empty, not starting with `-`, not a transport
/// helper and without credentials (§5.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitUrl(String);

impl GitUrl {
    /// The URL as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether git would read `url` as `<helper>::<address>`: a run of ASCII letters, digits, `+`,
/// `-` and `.` (possibly none) and then `::`. git then runs `git-remote-<helper>`, and for `ext`
/// a command line; alz never uses a helper (§5.2 "Transports").
fn names_transport_helper(url: &str) -> bool {
    url.trim_start_matches(|character: char| {
        character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
    })
    .starts_with("::")
}

/// The URL of `spec`, which is `<url>[@<ref>]`: `spec` without its ref when the last `@` starts
/// a ref that can be one (not empty, no `:`, no `/`), otherwise the whole `spec`. A `/` rules the
/// split out because `me:token@host/o/r` must not read as the URL `me:token` and the ref
/// `host/o/r` (§5.2 "Credentials in a source").
fn address_of(spec: &str) -> &str {
    spec.rsplit_once('@')
        .filter(|(url, git_ref)| {
            !url.is_empty() && !git_ref.is_empty() && !git_ref.contains([':', '/'])
        })
        .map_or(spec, |(url, _)| url)
}

/// The scheme and the rest of `text`, when `text` is `<scheme>://<rest>`.
fn split_scheme(text: &str) -> Option<(&str, &str)> {
    text.split_once("://")
        .filter(|(scheme, _)| !scheme.contains('/'))
}

/// Whether `url`, a `git:` URL without its ref, holds credentials, by §5.2:
/// - a URL with `://`: an `http://` or `https://` URL with anything before an `@` in its
///   authority, or any other scheme whose user part holds a `:`;
/// - any other address, a scp-like `[user@]host:path`: a user part, which is what precedes the
///   first `@` of its first `/`-separated segment, that holds a `:`.
fn holds_credentials(url: &str) -> bool {
    match split_scheme(url) {
        Some((scheme, rest)) => {
            let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or_default();
            let web = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
            authority
                .rsplit_once('@')
                .is_some_and(|(user, _)| web || user.contains(':'))
        }
        None => url
            .split('/')
            .next()
            .and_then(|segment| segment.split_once('@'))
            .is_some_and(|(user, _)| user.contains(':')),
    }
}

/// Whether the text after `git:`, which may end with `@<ref>`, holds credentials. A URL with
/// `://` is checked whole, because without a ref its last `@` is the credentials' own. Any other
/// text is checked without the ref it ends with, so `x:y@v1` is the address `x:y` and a ref,
/// while `me:token@host/o/r` is checked whole.
fn spec_holds_credentials(spec: &str) -> bool {
    holds_credentials(if split_scheme(spec).is_some() {
        spec
    } else {
        address_of(spec)
    })
}

impl TryFrom<&str> for GitUrl {
    type Error = SourceError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.starts_with('-') {
            Err(SourceError::UrlStartsWithDash)
        } else if names_transport_helper(text) {
            Err(SourceError::TransportHelper)
        } else if holds_credentials(text) {
            Err(SourceError::Credentials)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// The owner or the repository of a `github:` source: ASCII letters, digits, `.`, `_` and `-`,
/// not starting with `-` and not `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitHubName(String);

impl GitHubName {
    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for GitHubName {
    type Error = SourceError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let well_formed = !matches!(text, "" | "." | "..")
            && !text.starts_with('-')
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(SourceError::GitHubShape)
        }
    }
}

/// A layer's origin, as the stack entry names it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// `path:<dir>`: a directory, which `ref:` may pin when it is inside a git repository (§6).
    Path {
        /// The directory as written.
        dir: SourceDir,
    },
    /// `git:<url>@<ref>`.
    Git {
        /// The remote or local repository.
        url: GitUrl,
        /// The tag, branch or commit.
        git_ref: GitRef,
    },
    /// `github:<owner>/<repo>@<ref>`.
    GitHub {
        /// The account or organisation.
        owner: GitHubName,
        /// The repository.
        repo: GitHubName,
        /// The tag, branch or commit.
        git_ref: GitRef,
    },
}

/// `<url>@<ref>` as its two parts, split at the last `@`. A ref holding `:` means that the `@`
/// was the user of a scp-like address and no ref was given.
fn split_ref(spec: &str) -> Result<(&str, GitRef), SourceError> {
    let (url, git_ref) = spec
        .rsplit_once('@')
        .filter(|(url, git_ref)| !url.is_empty() && !git_ref.is_empty() && !git_ref.contains(':'))
        .ok_or(SourceError::MissingRef)?;
    Ok((url, GitRef::try_from(git_ref)?))
}

impl TryFrom<&str> for Source {
    type Error = SourceError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if let Some(dir) = text.strip_prefix("path:") {
            if dir.is_empty() {
                Err(SourceError::EmptyPath)
            } else {
                Ok(Self::Path {
                    dir: SourceDir(dir.to_owned()),
                })
            }
        } else if let Some(spec) = text.strip_prefix("github:") {
            let (repo_text, git_ref) = split_ref(spec)?;
            let (owner, repo) = repo_text.split_once('/').ok_or(SourceError::GitHubShape)?;
            Ok(Self::GitHub {
                owner: GitHubName::try_from(owner)?,
                repo: GitHubName::try_from(repo)?,
                git_ref,
            })
        } else if let Some(spec) = text.strip_prefix("git:") {
            // The whole text is checked too: without a ref, the last `@` is the credentials' own
            // and the split below would cut the URL at it (§5.2). The transport is read from the
            // start of the text, so a helper is refused before anything else is.
            if names_transport_helper(spec) {
                return Err(SourceError::TransportHelper);
            }
            if spec_holds_credentials(spec) {
                return Err(SourceError::Credentials);
            }
            let (url, git_ref) = split_ref(spec)?;
            Ok(Self::Git {
                url: GitUrl::try_from(url)?,
                git_ref,
            })
        } else {
            Err(SourceError::UnknownScheme)
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { dir } => write!(formatter, "path:{}", dir.0),
            Self::Git { url, git_ref } => write!(formatter, "git:{}@{git_ref}", url.0),
            Self::GitHub {
                owner,
                repo,
                git_ref,
            } => write!(formatter, "github:{}/{}@{git_ref}", owner.0, repo.0),
        }
    }
}

#[cfg(test)]
mod tests;
