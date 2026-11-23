//! The runner: the one way alz runs git for a layer source.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use alz_core::env::EnvVars;
use alz_core::git::{
    CloneTransport, GitVersion, ObjectCheck, ObjectContent, TreeEntry, UnreadableOutput,
    VersionRefusal, child_environment,
};
use alz_core::lock::ObjectId;
use alz_core::path::SafeRelPath;
use alz_core::stack::GitRef;
use thiserror::Error;

use super::call::{GitCall, Outcome, ReadScope};
use super::deadlines::Deadlines;
use super::detach::{COULD_NOT_RUN_GIT, Launcher};
use super::failure::GitFailure;
use super::spawn::{self, Invocation};
use crate::EnvSnapshot;

/// A runner that could not be made.
#[derive(Debug, Error)]
pub enum GitRunnerError {
    /// The running executable, which starts git without a terminal, cannot be found.
    #[error("cannot find the running executable")]
    Executable(#[source] io::Error),
    /// The git on `PATH` is refused (spec §6 "Version").
    #[error("{0}")]
    Refused(#[from] VersionRefusal),
}

/// Runs git for the sources of a stack. A runner exists only for a git that is at least
/// [`GitVersion::MINIMUM`], so no call can run before the version is checked. Each call is typed:
/// the caller names objects and paths by their types, never by a string of arguments.
#[derive(Debug)]
pub struct GitRunner {
    launcher: Launcher,
    environment: EnvVars,
    deadlines: Deadlines,
}

fn mismatch() -> GitFailure {
    GitFailure::Unreadable(UnreadableOutput::new("a result of another kind"))
}

impl GitRunner {
    /// A runner that starts git through the running executable, after checking the git version.
    ///
    /// # Errors
    ///
    /// [`GitRunnerError::Refused`] when git is older than 2.30, prints no version or cannot be
    /// started; [`GitRunnerError::Executable`] when the running executable is not known.
    pub fn new(environment: &EnvSnapshot, deadlines: Deadlines) -> Result<Self, GitRunnerError> {
        let launcher = Launcher::current_exe().map_err(GitRunnerError::Executable)?;
        Ok(Self::with_launcher(launcher, environment, deadlines)?)
    }

    /// A runner that starts git through `launcher`, after checking the git version.
    ///
    /// # Errors
    ///
    /// The [`VersionRefusal`] when git is older than 2.30, prints no version or cannot be started.
    pub fn with_launcher(
        launcher: Launcher,
        environment: &EnvSnapshot,
        deadlines: Deadlines,
    ) -> Result<Self, VersionRefusal> {
        let runner = Self {
            launcher,
            environment: environment.vars().clone(),
            deadlines,
        };
        match runner.run(&GitCall::Version) {
            Ok(Outcome::Version(output)) => GitVersion::accept(&output).map(|_| runner),
            Ok(_)
            | Err(
                GitFailure::Io(_)
                | GitFailure::TimedOut { .. }
                | GitFailure::OutputTooLarge
                | GitFailure::Unreadable(_),
            ) => Err(VersionRefusal::Unreadable),
            Err(GitFailure::Spawn(_)) => Err(VersionRefusal::NotStarted),
            Err(GitFailure::Exited { status, .. }) => {
                Err(if status.code() == Some(i32::from(COULD_NOT_RUN_GIT)) {
                    VersionRefusal::NotStarted
                } else {
                    VersionRefusal::Unreadable
                })
            }
        }
    }

    fn run(&self, call: &GitCall<'_>) -> Result<Outcome, GitFailure> {
        let arguments = call.arguments();
        let environment = child_environment(call.operation(), &self.environment);
        let completed = spawn::run(Invocation {
            launcher: &self.launcher,
            arguments: &arguments,
            environment: &environment,
            directory: call.directory(),
            input: call.input(),
            deadline: if call.is_clone() {
                self.deadlines.clone
            } else {
                self.deadlines.other
            },
            stdout_limit: call.limit().bytes(),
        })?;
        if completed.status.success() {
            if completed.stdout_truncated {
                return Err(GitFailure::OutputTooLarge);
            }
            call.read(&completed.stdout).map_err(GitFailure::from)
        } else if call.status_one_is_no_answer()
            && completed.status.code() == Some(1)
            && completed.stdout.is_empty()
        {
            Ok(Outcome::Commit(None))
        } else {
            Err(GitFailure::Exited {
                status: completed.status,
                stderr: completed.stderr,
            })
        }
    }

    /// Clones `source` into `destination` without checking anything out of the ordinary: a
    /// remote source is given `-c protocol.file.allow=never`.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn clone_source(
        &self,
        transport: CloneTransport,
        source: &OsStr,
        destination: &Path,
    ) -> Result<(), GitFailure> {
        self.run(&GitCall::Clone {
            transport,
            source,
            destination,
        })
        .map(|_| ())
    }

    /// The commit that `rev` names in `repository`, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn rev_parse_commit(
        &self,
        repository: &Path,
        scope: ReadScope,
        rev: &GitRef,
    ) -> Result<Option<ObjectId>, GitFailure> {
        match self.run(&GitCall::RevParseCommit {
            repository,
            scope,
            rev,
        })? {
            Outcome::Commit(commit) => Ok(commit),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Path(_)
            | Outcome::Prefix(_)
            | Outcome::Tree(_)
            | Outcome::Checks(_)
            | Outcome::Contents(_) => Err(mismatch()),
        }
    }

    /// Checks `commit` out in `repository`, detached.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn checkout(&self, repository: &Path, commit: &ObjectId) -> Result<(), GitFailure> {
        self.run(&GitCall::Checkout { repository, commit })
            .map(|_| ())
    }

    /// The top level of the work tree that holds `directory`.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn show_toplevel(&self, directory: &Path) -> Result<PathBuf, GitFailure> {
        match self.run(&GitCall::ShowToplevel { directory })? {
            Outcome::Path(path) => Ok(path),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Commit(_)
            | Outcome::Prefix(_)
            | Outcome::Tree(_)
            | Outcome::Checks(_)
            | Outcome::Contents(_) => Err(mismatch()),
        }
    }

    /// The path of `directory` below the top level of its work tree, with a trailing `/`, or
    /// empty at the top.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn show_prefix(&self, directory: &Path) -> Result<OsString, GitFailure> {
        match self.run(&GitCall::ShowPrefix { directory })? {
            Outcome::Prefix(prefix) => Ok(prefix),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Commit(_)
            | Outcome::Path(_)
            | Outcome::Tree(_)
            | Outcome::Checks(_)
            | Outcome::Contents(_) => Err(mismatch()),
        }
    }

    /// The entry of `path` in the tree of `commit`: none when the commit has no such path.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn ls_tree_at(
        &self,
        repository: &Path,
        commit: &ObjectId,
        path: &SafeRelPath,
    ) -> Result<Vec<TreeEntry>, GitFailure> {
        self.tree(&GitCall::LsTreeAt {
            repository,
            commit,
            path,
        })
    }

    /// Every file of `tree`, recursively, in git's order.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn ls_tree_recursive(
        &self,
        repository: &Path,
        tree: &ObjectId,
    ) -> Result<Vec<TreeEntry>, GitFailure> {
        self.tree(&GitCall::LsTreeRecursive { repository, tree })
    }

    fn tree(&self, call: &GitCall<'_>) -> Result<Vec<TreeEntry>, GitFailure> {
        match self.run(call)? {
            Outcome::Tree(entries) => Ok(entries),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Commit(_)
            | Outcome::Path(_)
            | Outcome::Prefix(_)
            | Outcome::Checks(_)
            | Outcome::Contents(_) => Err(mismatch()),
        }
    }

    /// Whether `repository` holds each of `objects`, in the order asked.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn cat_file_batch_check(
        &self,
        repository: &Path,
        objects: &[ObjectId],
    ) -> Result<Vec<ObjectCheck>, GitFailure> {
        match self.run(&GitCall::CatFileBatchCheck {
            repository,
            objects,
        })? {
            Outcome::Checks(checks) => Ok(checks),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Commit(_)
            | Outcome::Path(_)
            | Outcome::Prefix(_)
            | Outcome::Tree(_)
            | Outcome::Contents(_) => Err(mismatch()),
        }
    }

    /// The content of each of `objects`, in the order asked.
    ///
    /// # Errors
    ///
    /// The [`GitFailure`] of the call.
    pub fn cat_file_batch(
        &self,
        repository: &Path,
        objects: &[ObjectId],
    ) -> Result<Vec<ObjectContent>, GitFailure> {
        match self.run(&GitCall::CatFileBatch {
            repository,
            objects,
        })? {
            Outcome::Contents(contents) => Ok(contents),
            Outcome::Nothing
            | Outcome::Version(_)
            | Outcome::Commit(_)
            | Outcome::Path(_)
            | Outcome::Prefix(_)
            | Outcome::Tree(_)
            | Outcome::Checks(_) => Err(mismatch()),
        }
    }
}
