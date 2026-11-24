//! Applying an adoption plan (spec §4.3): the layer files, the moves, the removals and the edit
//! of `layer.yaml`, each only for the paths the plan names and only while they hold the bytes the
//! plan was made from. Nothing is written until every one of them was checked; no step follows a
//! link or goes outside the root.

use std::collections::BTreeMap;
use std::io;
use std::os::fd::OwnedFd;

use alz_core::adopt::{AdoptionPlan, Move, NativeFile};
use alz_core::layer::LayerContents;
use alz_core::path::{SafeRelPath, SafeRelPathError};
use alz_core::render::write_plan::Observed;
use rustix::fs::{self, AtFlags};
use rustix::io::Errno;

use crate::fs::repo_root::create::create_new;
use crate::fs::repo_root::error::RepoError;
use crate::fs::repo_root::probe::{Below, folder_at, probe};
use crate::fs::repo_root::write::{delete_file, folder_for, prune, write_file};

/// What the plan was made from: the bytes of every file it moves or removes, of `layer.yaml`, and
/// the files the repo layer had.
#[derive(Debug, Clone, Copy)]
pub struct AdoptionBasis<'a> {
    /// The native files, with the bytes observed when the plan was made.
    pub natives: &'a BTreeMap<SafeRelPath, NativeFile>,
    /// The bytes of `layer.yaml` the manifest edit was made from.
    pub manifest_text: &'a [u8],
    /// The files the repo layer had when the plan was made.
    pub layer: &'a LayerContents,
}

/// The manifest's path.
const MANIFEST: &str = "layer.yaml";

/// Checks the whole plan against the repository, then applies it: layer files, moves, removals,
/// the folders they left empty, and the manifest.
pub(super) fn apply(
    root: &OwnedFd,
    plan: &AdoptionPlan,
    basis: &AdoptionBasis<'_>,
) -> Result<(), RepoError> {
    let layer_files = plan
        .layer_files()
        .iter()
        .map(|(path, bytes)| inside(plan.layer_dir(), path).map(|full| (full, path, bytes)))
        .collect::<Result<Vec<_>, _>>()?;
    let moves = plan
        .moves()
        .iter()
        .map(|moved| inside(plan.layer_dir(), moved.to()).map(|to| (moved.from(), to)))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = SafeRelPath::try_from(MANIFEST)
        .map_err(|SafeRelPathError| RepoError::changed_text(MANIFEST))?;

    // Everything is checked before the first write.
    layer_files
        .iter()
        .try_for_each(|(full, path, _)| match basis.layer.get(path) {
            None => require_absent(root, full),
            Some(old) => require_bytes(root, full, old),
        })?;
    moves.iter().try_for_each(|(from, to)| {
        require_bytes(root, from, native(basis, from)?.bytes())?;
        require_absent(root, to)
    })?;
    plan.removes()
        .iter()
        .try_for_each(|path| require_bytes(root, path, native(basis, path)?.bytes()))?;
    plan.manifest_edit()
        .map(|_| require_bytes(root, &manifest, basis.manifest_text))
        .transpose()?;

    layer_files
        .iter()
        .try_for_each(|(full, path, bytes)| match basis.layer.get(path) {
            None => create_new(root, full, bytes),
            Some(_) => write_file(root, full, bytes, false),
        })?;
    moves
        .iter()
        .try_for_each(|(from, to)| move_file(root, from, to))?;
    plan.removes()
        .iter()
        .try_for_each(|path| delete_file(root, path))?;
    prune(
        root,
        plan.moves().iter().map(Move::from).chain(plan.removes()),
    )?;
    plan.manifest_edit().map_or(Ok(()), |edit| {
        write_file(root, &manifest, edit.text().as_bytes(), false)
    })
}

/// `path` below the repo layer's folder.
fn inside(layer_dir: &SafeRelPath, path: &SafeRelPath) -> Result<SafeRelPath, RepoError> {
    let joined = format!("{}/{}", layer_dir.as_str(), path.as_str());
    SafeRelPath::try_from(joined.as_str())
        .map_err(|SafeRelPathError| RepoError::changed_text(&joined))
}

fn native<'a>(basis: &AdoptionBasis<'a>, path: &SafeRelPath) -> Result<&'a NativeFile, RepoError> {
    basis
        .natives
        .get(path)
        .ok_or_else(|| RepoError::changed(path))
}

/// Fails with [`RepoError::Exists`] unless nothing is at `path`.
fn require_absent(root: &OwnedFd, path: &SafeRelPath) -> Result<(), RepoError> {
    match probe(root, path)? {
        Observed::Absent => Ok(()),
        Observed::Regular { .. } | Observed::Link | Observed::NotRegular | Observed::TooLarge => {
            Err(RepoError::exists(path))
        }
    }
}

/// Fails with [`RepoError::Changed`] unless `path` is a regular file holding exactly `bytes`.
fn require_bytes(root: &OwnedFd, path: &SafeRelPath, bytes: &[u8]) -> Result<(), RepoError> {
    match probe(root, path)? {
        Observed::Regular { bytes: found, .. } if found == bytes => Ok(()),
        Observed::Regular { .. }
        | Observed::Absent
        | Observed::Link
        | Observed::NotRegular
        | Observed::TooLarge => Err(RepoError::changed(path)),
    }
}

/// Moves the file `from` to `to`, which does not exist: linked to its new name, which fails when
/// anything is there, then unlinked from the old one. The file keeps its mode, and no link is
/// followed.
fn move_file(root: &OwnedFd, from: &SafeRelPath, to: &SafeRelPath) -> Result<(), RepoError> {
    let segments: Vec<&str> = from.segments().collect();
    let targets: Vec<&str> = to.segments().collect();
    let (Some((name, parents)), Some((target, target_parents))) =
        (segments.split_last(), targets.split_last())
    else {
        return Err(RepoError::changed(from));
    };
    let source = match folder_at(root, parents.iter().copied())
        .map_err(|source| RepoError::io("reading", from, source))?
    {
        Below::Folder(directory) => directory,
        Below::Stop(_) => return Err(RepoError::changed(from)),
    };
    let destination = folder_for(root, target_parents, to)?;
    fs::linkat(&source, *name, &destination, *target, AtFlags::empty()).map_err(
        |errno| match errno {
            Errno::EXIST => RepoError::exists(to),
            other => RepoError::io("writing", to, io::Error::from(other)),
        },
    )?;
    fs::unlinkat(&source, *name, AtFlags::empty())
        .map_err(|errno| RepoError::io("removing", from, io::Error::from(errno)))
}
