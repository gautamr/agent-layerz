//! Answers the questions of alz-core's classification of a `git:` URL with the file system
//! (spec §6 "Layers in or around an account folder"): does a path git would try exist, and what
//! is its canonical path. Every lookup goes through the component walk, so a path in or around an
//! account folder is refused before anything in it is looked at.

use alz_core::account::AccountFolders;
use alz_core::path::AbsolutePath;
use alz_core::stack::{CloneOrigin, GitUrl, OriginRefusal, Probe, locate_git};

use crate::fs::{Dir, WalkError};

/// What `git clone` is given for the `git:` source with `url`, found relative to `repo_root`.
///
/// # Errors
///
/// An [`OriginRefusal`]: a `..` segment in a local path, a `file://` URL without a path, or a
/// path that touches an account folder or the home folder.
pub fn locate_git_source(
    url: &GitUrl,
    repo_root: &AbsolutePath,
    folders: &AccountFolders,
) -> Result<CloneOrigin, OriginRefusal> {
    locate_git(url, repo_root, &mut |path| probe(path, folders))
}

/// Whether something exists at `path`, and where it really is.
fn probe(path: &AbsolutePath, folders: &AccountFolders) -> Probe {
    match Dir::open_with_real_path(path, folders) {
        Ok((_, real)) => Probe::Found(real),
        Err(WalkError::Refused(_)) => Probe::Refused,
        Err(WalkError::NotADirectory) => probe_file(path, folders),
        Err(WalkError::Missing | WalkError::TooManyLinks | WalkError::Io(_)) => Probe::Absent,
    }
}

/// A path that is not a folder may still be a file, such as a `.bundle`: its folder is walked and
/// the name is looked up there, relative to the folder's handle and without following a link. Only
/// a regular file counts; a link, a pipe or a device is absent.
fn probe_file(path: &AbsolutePath, folders: &AccountFolders) -> Probe {
    let text = path.as_str().trim_end_matches('/');
    let Some((parent, name)) = text.rsplit_once('/') else {
        return Probe::Absent;
    };
    let Ok(parent) = AbsolutePath::try_from(if parent.is_empty() { "/" } else { parent }) else {
        return Probe::Absent;
    };
    match Dir::open_with_real_path(&parent, folders) {
        Ok((folder, real)) if folder.holds_regular_file(name) => {
            let joined = format!("{}/{name}", real.as_str().trim_end_matches('/'));
            AbsolutePath::try_from(joined.as_str()).map_or(Probe::Absent, Probe::Found)
        }
        Err(WalkError::Refused(_)) => Probe::Refused,
        Ok(_)
        | Err(
            WalkError::Missing
            | WalkError::NotADirectory
            | WalkError::TooManyLinks
            | WalkError::Io(_),
        ) => Probe::Absent,
    }
}
