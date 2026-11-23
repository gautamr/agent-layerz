//! The repository root: the only capability that writes inside the repository (decision-27,
//! spec §4.3). It looks at paths one component at a time without following a link, and applies a
//! plan the pure core made. Every path it takes is a [`SafeRelPath`], so nothing names a place
//! outside the root.

mod error;
mod list;
mod probe;
mod write;

use std::collections::BTreeSet;

use alz_core::account::AccountFolders;
use alz_core::lock::Lock;
use alz_core::path::{AbsolutePath, SafeRelPath};
use alz_core::render::tree::{FileTree, OutputPath, fold};
use alz_core::render::write_plan::{ClearPlan, Disk, Observed, gitignore_path, lock_path};

use crate::fs::dir::Dir;
use crate::fs::walk_error::WalkError;

pub use error::RepoError;
pub use write::Applied;

/// A repository's root folder, opened by the component walk. It reads and writes below itself
/// and nowhere else.
#[derive(Debug)]
pub struct RepoRoot {
    root: Dir,
}

impl RepoRoot {
    /// The root capability for a directory the component walk found.
    #[must_use]
    pub const fn new(root: Dir) -> Self {
        Self { root }
    }

    /// Opens the repository at `path` with the component walk of [`Dir::open`].
    ///
    /// # Errors
    ///
    /// The errors of [`Dir::open`].
    pub fn open(path: &AbsolutePath, folders: &AccountFolders) -> Result<Self, WalkError> {
        Dir::open(path, folders).map(Self::new)
    }

    /// Looks at what the plan asks about, without following a link: each path of `tree`, each path
    /// the previous lock lists, `layer.lock` and `.gitignore`, every file in the
    /// `managed_folders`, and the names beside each generated path that differ from it only in
    /// case.
    ///
    /// # Errors
    ///
    /// [`RepoError`] when the operating system fails, when a managed folder holds an entry whose
    /// name is not a safe relative path, or when it holds too many entries.
    pub fn observe(
        &self,
        tree: &FileTree,
        previous: Option<&Lock>,
        managed_folders: &[SafeRelPath],
    ) -> Result<Disk, RepoError> {
        let handle = self.root.handle();
        let managed: BTreeSet<SafeRelPath> = managed_folders
            .iter()
            .map(|folder| list::files_below(handle, folder))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        let wanted: BTreeSet<SafeRelPath> = tree
            .paths()
            .map(|path| path.as_safe().clone())
            .chain(
                previous
                    .into_iter()
                    .flat_map(|lock| lock.files.keys().cloned()),
            )
            .chain([lock_path().clone(), gitignore_path().clone()])
            .chain(managed.iter().cloned())
            .collect();
        let seen = wanted
            .into_iter()
            .map(|path| probe::probe(handle, &path).map(|observed| (path, observed)))
            .collect::<Result<Vec<(SafeRelPath, Observed)>, _>>()?;
        let folded: BTreeSet<String> = tree.paths().map(OutputPath::folded).collect();
        let parents: BTreeSet<Option<SafeRelPath>> =
            tree.paths().map(|path| parent_of(path.as_safe())).collect();
        let listed = parents
            .iter()
            .map(|parent| list::names_in(handle, parent.as_ref()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .filter(|name| folded.contains(&fold(name)));
        Ok(Disk::new(seen, managed, listed))
    }

    /// Applies a plan that has no collision: each file as a temporary file in its folder renamed
    /// into place, the deletes, the folders they left empty, `.gitignore`, and `layer.lock` last.
    /// It stops at the first failure, which leaves every file either as it was or complete, and
    /// the lock as it was.
    ///
    /// # Errors
    ///
    /// [`RepoError`] for the first operation that fails, or [`RepoError::Changed`] when a folder
    /// above a path became a link or a file since the plan was made.
    pub fn apply(&self, plan: &ClearPlan) -> Result<Applied, RepoError> {
        write::apply(self.root.handle(), plan)
    }
}

/// The folder above `path`, or `None` for a path at the root.
fn parent_of(path: &SafeRelPath) -> Option<SafeRelPath> {
    path.as_str()
        .rsplit_once('/')
        .and_then(|(parent, _)| SafeRelPath::try_from(parent).ok())
}
