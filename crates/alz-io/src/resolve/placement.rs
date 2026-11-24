//! Where each layer of a stack lies in relation to the repository (spec §7.12 "Who may declare
//! it"): the file system's answer, which alz-core's rule for `commit_identity` and `account`
//! takes as a fact.
//!
//! A layer is outside the repository when its source is `github:`, a `git:` URL that is a remote,
//! or a directory that is outside the repository root both as written and after following links.
//! A case variant of a path inside the repository and a link outside it that points into it are
//! inside: the comparison folds case and Unicode form (`lies_within`), and the real path is the
//! one the component walk found.

use alz_core::account::{AccountFolders, lies_within};
use alz_core::git::CloneTransport;
use alz_core::merge::{Placement, Placements};
use alz_core::path::AbsolutePath;
use alz_core::stack::{ResolvedStack, Source};

use crate::fs::{Dir, RepositoryPaths};
use crate::resolve::locate::locate_git_source;

/// Where the directory `directory` lies in relation to `repository`: inside when it, or its real
/// path, lies within the repository as written or by its real path. A directory that cannot be
/// opened (missing, refused by the walk) counts as inside, the stricter answer: such a layer was
/// not resolved and can declare nothing.
#[must_use]
pub fn placement_of_directory(
    directory: &AbsolutePath,
    repository: &RepositoryPaths,
    folders: &AccountFolders,
) -> Placement {
    Dir::open_with_real_path(directory, folders).map_or(Placement::Inside, |(_, real)| {
        let inside = [directory, &real].into_iter().any(|path| {
            [repository.as_written(), repository.real()]
                .into_iter()
                .any(|base| lies_within(path, base))
        });
        if inside {
            Placement::Inside
        } else {
            Placement::Outside
        }
    })
}

/// Where the layer of `source` lies. A `path:` source is the directory it names, read against the
/// repository root; a `git:` source is a remote unless `git clone` would find a local path of that
/// name (§7.12), which is then placed by its canonical path; `github:` is always a remote.
#[must_use]
pub fn placement_of_source(
    source: &Source,
    repository: &RepositoryPaths,
    folders: &AccountFolders,
) -> Placement {
    let local = |path: Option<AbsolutePath>| {
        path.map_or(Placement::Inside, |path| {
            placement_of_directory(&path, repository, folders)
        })
    };
    match source {
        Source::Path { dir } => local(path_of(dir.as_str(), repository.as_written())),
        Source::Git { url, .. } => locate_git_source(url, repository.as_written(), folders).map_or(
            Placement::Inside,
            |origin| match origin.transport() {
                CloneTransport::Remote => Placement::Outside,
                CloneTransport::Local => local(AbsolutePath::try_from(origin.argument()).ok()),
            },
        ),
        // Always a remote, cloned over `https` and never from a local path (§7.12).
        Source::GitHub { .. } => Placement::Outside,
    }
}

/// The placement of every layer of `stack`, by name.
#[must_use]
pub fn placements_of_stack(
    stack: &ResolvedStack,
    repository: &RepositoryPaths,
    folders: &AccountFolders,
) -> Placements {
    stack
        .layers()
        .iter()
        .map(|layer| {
            (
                layer.meta().name().clone(),
                placement_of_source(layer.source(), repository, folders),
            )
        })
        .collect()
}

/// The absolute path of a `path:` source: relative to the repository root, as written.
pub(crate) fn path_of(written: &str, repository: &AbsolutePath) -> Option<AbsolutePath> {
    let joined = if written.starts_with('/') {
        written.to_owned()
    } else {
        format!("{}/{written}", repository.as_str().trim_end_matches('/'))
    };
    AbsolutePath::try_from(joined.as_str()).ok()
}
