//! The repository root: the only capability that writes inside the repository (decision-27,
//! spec §4.3). It looks at paths one component at a time without following a link, and applies a
//! plan the pure core made. Every path it takes is a [`SafeRelPath`], so nothing names a place
//! outside the root.

mod adopt;
mod create;
mod error;
mod git_config;
mod list;
mod probe;
mod write;

use std::collections::BTreeSet;

use alz_core::Stripped;
use alz_core::account::AccountFolders;
use alz_core::adopt::AdoptionPlan;
use alz_core::identity::{Blocker, GitConfigPlace};
use alz_core::lock::Lock;
use alz_core::path::{AbsolutePath, SafeRelPath};
use alz_core::render::tree::{FileTree, OutputPath, fold};
use alz_core::render::write_plan::{ClearPlan, Disk, Observed, gitignore_path, lock_path};

use crate::EnvSnapshot;
use crate::fs::dir::Dir;
use crate::fs::walk_error::WalkError;
use crate::resolve::folders::account_folders;

pub use adopt::AdoptionBasis;
pub use error::RepoError;
pub use git_config::{Changed, GitConfigSeal};
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

    /// Opens the repository at `directory` with the account folders of `env`.
    ///
    /// # Errors
    ///
    /// The errors of [`Dir::open`].
    pub fn at(env: &EnvSnapshot, directory: &AbsolutePath) -> Result<Self, WalkError> {
        Self::open(directory, &account_folders(env))
    }

    /// What is at `path`, looked at without following a link: nothing, a regular file, a link or
    /// something else.
    ///
    /// # Errors
    ///
    /// [`RepoError`] when the operating system fails.
    pub fn look_at(&self, path: &SafeRelPath) -> Result<Observed, RepoError> {
        probe::probe(self.root.handle(), path)
    }

    /// Creates the file `path` with `bytes`, which must not exist: a file, a folder or a link
    /// there is [`RepoError::Exists`], and a link in a folder above it is
    /// [`RepoError::Changed`]. Missing folders are made.
    ///
    /// # Errors
    ///
    /// [`RepoError`] for the reasons above and for a failure of the operating system.
    pub fn create_new(&self, path: &SafeRelPath, bytes: &[u8]) -> Result<(), RepoError> {
        create::create_new(self.root.handle(), path, bytes)
    }

    /// Looks at what the plan asks about, without following a link: each path of `tree`, each path
    /// the previous lock lists, `layer.lock` and `.gitignore`, every file in the
    /// `managed_folders`, and the names beside each generated path that differ from it only in
    /// case.
    ///
    /// # Errors
    ///
    /// [`RepoError`] when the operating system fails, when a managed folder holds an entry whose
    /// name is not a safe relative path.
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

    /// Whether `.git/config` of this root may be written: the root holds a real `.git` folder,
    /// which makes it the top level of its work tree, and `.git/config` is absent or a regular
    /// file. Nothing is followed, and nothing is written.
    #[must_use]
    pub fn git_config_place(&self) -> GitConfigPlace {
        git_config::place(self.root.handle())
    }

    /// Checks the place as [`git_config_place`](Self::git_config_place) does, right before a
    /// write, and remembers `.git` for the check after it.
    ///
    /// # Errors
    ///
    /// The [`Blocker`] that says why nothing may be written.
    pub fn seal_git_config(&self) -> Result<GitConfigSeal, Blocker> {
        git_config::seal(self.root.handle())
    }

    /// Checks the place again, right after a write: `.git` is still the folder that was sealed
    /// and `.git/config` is still not a link.
    ///
    /// # Errors
    ///
    /// What changed: git may have written through a link.
    pub fn verify_git_config(&self, seal: GitConfigSeal) -> Result<(), Changed> {
        git_config::verify(self.root.handle(), seal)
    }

    /// Whether `path`, followed to the end, is the folder this root was opened on.
    ///
    /// # Errors
    ///
    /// The error of looking at either folder.
    pub fn is_this_folder(&self, path: &std::path::Path) -> std::io::Result<bool> {
        let mine = rustix::fs::fstat(self.root.handle())?;
        let theirs = rustix::fs::stat(path)?;
        Ok(mine.st_dev == theirs.st_dev && mine.st_ino == theirs.st_ino)
    }

    /// Every file named `name` anywhere in the repository, in path order, found without following
    /// a link and without entering `.git`, `.DS_Store` or `node_modules`. A folder may hold any
    /// number of entries and sit at any depth, and its name need not be valid Unicode: the paths
    /// are given as they are shown, not as safe relative paths.
    ///
    /// # Errors
    ///
    /// [`RepoError`] when the operating system fails.
    pub fn files_named(&self, name: &str) -> Result<Vec<Stripped>, RepoError> {
        list::files_named(self.root.handle(), name)
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

    /// Applies an adoption plan: the layer files, the files that move into the layer, the files
    /// that are removed and the folders that leaves empty, then the edit of `layer.yaml`. Every
    /// path is checked against `basis` first, and a path that is not as the plan found it is
    /// [`RepoError::Changed`] (or [`RepoError::Exists`] for a file that is to be created) with
    /// nothing written. No link is followed.
    ///
    /// # Errors
    ///
    /// [`RepoError`] for the reasons above and for a failure of the operating system, which can
    /// leave the adoption partly applied: every file is then either as it was or complete.
    pub fn adopt(&self, plan: &AdoptionPlan, basis: &AdoptionBasis<'_>) -> Result<(), RepoError> {
        adopt::apply(self.root.handle(), plan, basis)
    }
}

/// The folder above `path`, or `None` for a path at the root.
fn parent_of(path: &SafeRelPath) -> Option<SafeRelPath> {
    path.as_str()
        .rsplit_once('/')
        .and_then(|(parent, _)| SafeRelPath::try_from(parent).ok())
}
