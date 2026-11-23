//! Starting git without a controlling terminal (spec §6 "Git subprocesses" "No terminal", owner
//! answer 1 of TASK-55.4, TASK-70).
//!
//! A program that can open `/dev/tty` can prompt there, outside alz's output and its
//! sanitising. The runner therefore starts a copy of this program, with [`INTERNAL_ARGUMENT`]
//! first. The copy, called early in `main` through [`run_if_requested`], starts a new session
//! with `setsid` and then replaces itself with `git`. A session leader without a terminal has no
//! controlling terminal, and neither does anything it starts, so git and `ssh` get ENXIO from
//! `/dev/tty`. The copy is not a process group leader when it calls `setsid`, which is what makes
//! the call succeed. All of this is safe code: `rustix` wraps the system calls.

use std::ffi::OsString;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use rustix::process::setsid;

/// The first argument of the copy. It is not part of the command-line interface: it runs `git`
/// and no other program, with the arguments that follow.
const INTERNAL_ARGUMENT: &str = "--alz-internal-run-git-without-terminal";

/// The exit status of a copy that could not run git, as a shell's "command not found".
pub(super) const COULD_NOT_RUN_GIT: u8 = 127;

/// The program that [`run_if_requested`] runs in: the `alz` binary. The runner starts it once
/// for each git call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher(PathBuf);

impl Launcher {
    /// The running executable, which is the one that calls [`run_if_requested`].
    ///
    /// # Errors
    ///
    /// The error of finding the running executable.
    pub fn current_exe() -> io::Result<Self> {
        std::env::current_exe().map(Self)
    }

    /// An executable that calls [`run_if_requested`] first in its `main`.
    #[must_use]
    pub fn at(executable: impl Into<PathBuf>) -> Self {
        Self(executable.into())
    }

    /// The command that starts the copy, which then runs git with `git_arguments`.
    pub(super) fn command(&self, git_arguments: &[OsString]) -> Command {
        let mut command = Command::new(&self.0);
        command.arg(INTERNAL_ARGUMENT).args(git_arguments);
        command
    }
}

/// When this process was started by a [`Launcher`], starts a new session and replaces the process
/// with `git`; it returns only if that fails, with the exit code to end with. When this is an
/// ordinary run it returns `None`, and the caller goes on. Call it first in `main`, before any
/// argument parsing.
#[must_use]
pub fn run_if_requested() -> Option<ExitCode> {
    let mut arguments = std::env::args_os().skip(1);
    (arguments.next()? == INTERNAL_ARGUMENT).then(|| exec_git(arguments.collect()))
}

/// Runs `git` in a new session; the result is the failure to do so.
fn exec_git(git_arguments: Vec<OsString>) -> ExitCode {
    let failure = match setsid() {
        Err(error) => ("cannot start a new session", io::Error::from(error)),
        Ok(_session) => (
            "cannot run git",
            Command::new("git").args(git_arguments).exec(),
        ),
    };
    // The line goes to the caller as git's standard error would; a failure to write it changes
    // nothing, because the exit code still says that git did not run.
    writeln!(io::stderr(), "alz: {}: {}", failure.0, failure.1).unwrap_or_default();
    ExitCode::from(COULD_NOT_RUN_GIT)
}
