//! The git calls alz makes to resolve a source (spec §6 "Git subprocesses"), each a value that
//! says its arguments, its input, its environment and how its output reads. Nothing here starts
//! a process.
//!
//! No caller writes an argument: the arguments follow from the typed values in the call, a
//! revision always follows `--end-of-options`, and no shell is involved.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use alz_core::git::{
    CloneTransport, GitOperation, ObjectCheck, ObjectContent, TreeEntry, UnreadableOutput,
};
use alz_core::lock::ObjectId;
use alz_core::path::SafeRelPath;
use alz_core::stack::GitRef;

/// Where a `rev-parse` runs, which decides its environment (spec §6 "Environment").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadScope {
    /// In a temporary clone or a cache entry.
    CacheEntry,
    /// In the repository that holds a pinned `path:` layer's tree.
    PinnedTree,
}

/// How much standard output a call accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputLimit {
    /// One line: an id, a path or a version.
    Line,
    /// A listing or the contents of objects.
    Listing,
}

impl OutputLimit {
    /// The most bytes of standard output that are read.
    pub(super) const fn bytes(self) -> u64 {
        match self {
            Self::Line => 64 * 1024,
            Self::Listing => 64 * 1024 * 1024,
        }
    }
}

/// A git call, with the repository it runs in.
#[derive(Debug, Clone, Copy)]
pub(super) enum GitCall<'a> {
    /// `git version`.
    Version,
    /// `git clone` of `source` into `destination`.
    Clone {
        transport: CloneTransport,
        source: &'a OsStr,
        destination: &'a Path,
    },
    /// The commit that `rev` names.
    RevParseCommit {
        repository: &'a Path,
        scope: ReadScope,
        rev: &'a GitRef,
    },
    /// The tree of `commit`.
    RevParseTree {
        repository: &'a Path,
        commit: &'a ObjectId,
    },
    /// Check out `commit`, detached.
    Checkout {
        repository: &'a Path,
        commit: &'a ObjectId,
    },
    /// The top level of the work tree that holds `directory`.
    ShowToplevel { directory: &'a Path },
    /// The path of `directory` below that top level.
    ShowPrefix { directory: &'a Path },
    /// The entry at `path` in `commit`'s tree.
    LsTreeAt {
        repository: &'a Path,
        commit: &'a ObjectId,
        path: &'a SafeRelPath,
    },
    /// Every file of `tree`, recursively.
    LsTreeRecursive {
        repository: &'a Path,
        tree: &'a ObjectId,
    },
    /// Whether the repository holds each of `objects`.
    CatFileBatchCheck {
        repository: &'a Path,
        objects: &'a [ObjectId],
    },
    /// The content of each of `objects`.
    CatFileBatch {
        repository: &'a Path,
        objects: &'a [ObjectId],
    },
}

/// What a call gave, read into a typed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Outcome {
    Nothing,
    Version(Vec<u8>),
    Object(Option<ObjectId>),
    Path(PathBuf),
    Prefix(OsString),
    Tree(Vec<TreeEntry>),
    Checks(Vec<ObjectCheck>),
    Contents(Vec<ObjectContent>),
}

/// The `-c` settings of a remote clone: no transport unless it is named, then https and ssh.
const REMOTE_CLONE_CONFIGURATION: [&str; 3] = [
    "protocol.allow=never",
    "protocol.https.allow=always",
    "protocol.ssh.allow=always",
];

fn os(text: &str) -> OsString {
    OsString::from(text)
}

/// The objects as git's batch commands read them: one id per line.
fn batch_input(objects: &[ObjectId]) -> Vec<u8> {
    objects
        .iter()
        .flat_map(|object| format!("{object}\n").into_bytes())
        .collect()
}

/// One line of output without its line feed.
fn single_line(output: &[u8]) -> &[u8] {
    output.strip_suffix(b"\n").unwrap_or(output)
}

impl GitCall<'_> {
    /// The environment this call gets (R3.2).
    pub(super) const fn operation(&self) -> GitOperation {
        match self {
            Self::Clone { transport, .. } => GitOperation::Clone {
                transport: *transport,
            },
            Self::RevParseCommit {
                scope: ReadScope::CacheEntry,
                ..
            }
            | Self::Checkout { .. }
            | Self::Version => GitOperation::CacheRead,
            Self::RevParseCommit {
                scope: ReadScope::PinnedTree,
                ..
            }
            | Self::RevParseTree { .. }
            | Self::ShowToplevel { .. }
            | Self::ShowPrefix { .. }
            | Self::LsTreeAt { .. }
            | Self::LsTreeRecursive { .. }
            | Self::CatFileBatchCheck { .. }
            | Self::CatFileBatch { .. } => GitOperation::TreeRead,
        }
    }

    /// The arguments after `git`.
    pub(super) fn arguments(&self) -> Vec<OsString> {
        match self {
            Self::Version => vec![os("version")],
            Self::Clone {
                transport,
                source,
                destination,
            } => {
                // A remote clone may use https and ssh and nothing else, not even a transport
                // that the user's configuration allows (spec §6 "Git subprocesses"); a local
                // one goes through git's own transport and not a copy of the files, so that
                // hard links never make the clone share the source's objects.
                let (guard, local) = match transport {
                    CloneTransport::Remote => (
                        REMOTE_CLONE_CONFIGURATION
                            .iter()
                            .flat_map(|setting| [os("-c"), os(setting)])
                            .collect(),
                        vec![],
                    ),
                    CloneTransport::Local => (vec![], vec![os("--no-local")]),
                };
                [
                    guard,
                    vec![os("clone"), os("--quiet")],
                    local,
                    vec![os("--"), (*source).to_owned()],
                    vec![destination.as_os_str().to_owned()],
                ]
                .concat()
            }
            Self::RevParseCommit { rev, .. } => vec![
                os("rev-parse"),
                os("--verify"),
                os("--quiet"),
                os("--end-of-options"),
                OsString::from(format!("{}^{{commit}}", rev.as_str())),
            ],
            Self::RevParseTree { commit, .. } => vec![
                os("rev-parse"),
                os("--verify"),
                os("--quiet"),
                os("--end-of-options"),
                OsString::from(format!("{commit}^{{tree}}")),
            ],
            Self::Checkout { commit, .. } => vec![
                os("-c"),
                os("advice.detachedHead=false"),
                os("checkout"),
                os("--quiet"),
                os("--end-of-options"),
                OsString::from(commit.to_string()),
            ],
            Self::ShowToplevel { .. } => vec![os("rev-parse"), os("--show-toplevel")],
            Self::ShowPrefix { .. } => vec![os("rev-parse"), os("--show-prefix")],
            Self::LsTreeAt { commit, path, .. } => vec![
                os("ls-tree"),
                os("-z"),
                os("--end-of-options"),
                OsString::from(commit.to_string()),
                OsString::from(path.to_string()),
            ],
            Self::LsTreeRecursive { tree, .. } => vec![
                os("ls-tree"),
                os("-r"),
                os("-z"),
                os("--end-of-options"),
                OsString::from(tree.to_string()),
            ],
            Self::CatFileBatchCheck { .. } => vec![os("cat-file"), os("--batch-check")],
            Self::CatFileBatch { .. } => vec![os("cat-file"), os("--batch")],
        }
    }

    /// The directory the call runs in: none for `git version` and a clone, which name theirs.
    pub(super) fn directory(&self) -> Option<&Path> {
        match self {
            Self::Version | Self::Clone { .. } => None,
            Self::RevParseCommit { repository, .. }
            | Self::RevParseTree { repository, .. }
            | Self::Checkout { repository, .. }
            | Self::LsTreeAt { repository, .. }
            | Self::LsTreeRecursive { repository, .. }
            | Self::CatFileBatchCheck { repository, .. }
            | Self::CatFileBatch { repository, .. } => Some(repository),
            Self::ShowToplevel { directory } | Self::ShowPrefix { directory } => Some(directory),
        }
    }

    /// What is written to git's standard input; none means the null device.
    pub(super) fn input(&self) -> Option<Vec<u8>> {
        match self {
            Self::CatFileBatchCheck { objects, .. } | Self::CatFileBatch { objects, .. } => {
                Some(batch_input(objects))
            }
            Self::Version
            | Self::Clone { .. }
            | Self::RevParseCommit { .. }
            | Self::RevParseTree { .. }
            | Self::Checkout { .. }
            | Self::ShowToplevel { .. }
            | Self::ShowPrefix { .. }
            | Self::LsTreeAt { .. }
            | Self::LsTreeRecursive { .. } => None,
        }
    }

    /// Whether the call is a clone, which has the longer deadline.
    pub(super) const fn is_clone(&self) -> bool {
        matches!(self, Self::Clone { .. })
    }

    /// How much standard output the call accepts.
    pub(super) const fn limit(&self) -> OutputLimit {
        match self {
            Self::LsTreeRecursive { .. }
            | Self::LsTreeAt { .. }
            | Self::CatFileBatchCheck { .. }
            | Self::CatFileBatch { .. } => OutputLimit::Listing,
            Self::Version
            | Self::Clone { .. }
            | Self::RevParseCommit { .. }
            | Self::RevParseTree { .. }
            | Self::Checkout { .. }
            | Self::ShowToplevel { .. }
            | Self::ShowPrefix { .. } => OutputLimit::Line,
        }
    }

    /// Whether git's exit status 1 with no output is an answer (`rev-parse --verify --quiet`
    /// of a revision that is not there) and not a failure.
    pub(super) const fn status_one_is_no_answer(&self) -> bool {
        matches!(
            self,
            Self::RevParseCommit { .. } | Self::RevParseTree { .. }
        )
    }

    /// Reads `output`, the standard output of a call that succeeded, into its typed result.
    pub(super) fn read(&self, output: &[u8]) -> Result<Outcome, UnreadableOutput> {
        match self {
            Self::Version => Ok(Outcome::Version(output.to_vec())),
            Self::Clone { .. } | Self::Checkout { .. } => Ok(Outcome::Nothing),
            Self::RevParseCommit { .. } | Self::RevParseTree { .. } => {
                std::str::from_utf8(single_line(output))
                    .ok()
                    .and_then(|text| ObjectId::try_from(text).ok())
                    .map(|id| Outcome::Object(Some(id)))
                    .ok_or(UnreadableOutput::new("a rev-parse object id"))
            }
            Self::ShowToplevel { .. } => Ok(Outcome::Path(PathBuf::from(OsStr::from_bytes(
                single_line(output),
            )))),
            Self::ShowPrefix { .. } => Ok(Outcome::Prefix(
                OsStr::from_bytes(single_line(output)).to_owned(),
            )),
            Self::LsTreeAt { .. } | Self::LsTreeRecursive { .. } => {
                TreeEntry::parse_all(output).map(Outcome::Tree)
            }
            Self::CatFileBatchCheck { .. } => ObjectCheck::parse_all(output).map(Outcome::Checks),
            Self::CatFileBatch { .. } => ObjectContent::parse_all(output).map(Outcome::Contents),
        }
    }
}

#[cfg(test)]
mod tests;
