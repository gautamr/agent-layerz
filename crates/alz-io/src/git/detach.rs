//! Starting git without a controlling terminal (spec §6 "Git subprocesses" "No terminal", owner
//! answer 1 of TASK-55.4, TASK-70).
//!
//! A program that can open `/dev/tty` can prompt there, outside alz's output and its
//! sanitising. The runner therefore starts a copy of this program, with [`INTERNAL_ARGUMENT`],
//! its own process id and the absolute path of git first. The copy, called early in `main`
//! through [`run_if_requested`], starts a new session with `setsid`, tells the runner so with one
//! byte, [`READY_BYTE`], on its standard output, and then runs that git as its child, in that
//! session. The runner treats the copy's process group as git's only after it has read the byte,
//! because before that the copy is not yet the leader of a group of its own. A session
//! leader without a terminal has no controlling terminal, and neither does anything it starts, so
//! git and `ssh` get ENXIO from `/dev/tty`. The copy is not a process group leader when it calls
//! `setsid`, which is what makes the call succeed. All of this is safe code: `rustix` wraps the
//! system calls.
//!
//! git runs in a session of its own, so an interrupt that reaches alz does not reach it, and alz
//! ending does not end it. The copy therefore stays as git's supervisor: it ends git's whole
//! process group, `ssh` included, when its parent is gone (a call abandoned because alz was
//! interrupted or killed), and otherwise ends with git's exit status.

use std::ffi::OsString;
use std::io::{self, Write};
use std::num::NonZero;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};
use std::thread;
use std::time::Duration;

use rustix::process::{Pid, Signal, getpid, getppid, kill_process_group, setsid};

/// How long the supervisor first waits between looks at git and at its own parent, and the
/// longest it waits: a call that is quick costs a millisecond, and a long one a look every 25 ms.
const FIRST_LOOK: Duration = Duration::from_millis(1);
const LONGEST_WAIT: Duration = Duration::from_millis(25);

/// The first argument of the copy. It is not part of the command-line interface: it runs `git`
/// and no other program, with the arguments that follow.
const INTERNAL_ARGUMENT: &str = "--alz-internal-run-git-without-terminal";

/// The one byte that the copy writes to its standard output once it has started its session. It
/// comes before anything git writes, and the runner takes it off the stream.
pub(super) const READY_BYTE: u8 = 0x06;

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

    /// The command that starts the copy, which then runs `git`, an absolute path, with
    /// `git_arguments`. The copy is told this process's id, so that it can tell when this process
    /// is gone.
    pub(super) fn command(&self, git: &Path, git_arguments: &[OsString]) -> Command {
        let mut command = Command::new(&self.0);
        command
            .arg(INTERNAL_ARGUMENT)
            .arg(std::process::id().to_string())
            .arg(git)
            .args(git_arguments);
        command
    }
}

/// When this process was started by a [`Launcher`], starts a new session, runs `git` in it and
/// ends with git's status; it returns the exit code to end with. When this is an ordinary run it
/// returns `None`, and the caller goes on. Call it first in `main`, before any argument parsing.
#[must_use]
pub fn run_if_requested() -> Option<ExitCode> {
    let mut arguments = std::env::args_os().skip(1);
    (arguments.next()? == INTERNAL_ARGUMENT).then(|| {
        let parent = arguments
            .next()
            .and_then(|text| text.to_str().and_then(|text| text.parse::<i32>().ok()));
        let git = arguments.next().map(PathBuf::from);
        supervise_git(parent, git, arguments.collect())
    })
}

/// Runs `git`, an absolute path, in a new session and waits for it, ending its whole process
/// group when `parent`, the process that asked for the call, is gone.
fn supervise_git(
    parent: Option<i32>,
    git: Option<PathBuf>,
    git_arguments: Vec<OsString>,
) -> ExitCode {
    let started = git
        .ok_or((
            "cannot run git",
            io::Error::other("no path of git was given"),
        ))
        .and_then(|git| {
            setsid()
                .map_err(|error| ("cannot start a new session", io::Error::from(error)))
                .and_then(|_session| {
                    announce_session().map_err(|error| ("cannot tell the caller", error))
                })
                .and_then(|()| {
                    Command::new(git)
                        .args(git_arguments)
                        .spawn()
                        .map_err(|error| ("cannot run git", error))
                })
        });
    match started {
        Ok(git) => wait_for(git, parent),
        Err((what, error)) => {
            // The line goes to the caller as git's standard error would; a failure to write it
            // changes nothing, because the exit code still says that git did not run.
            writeln!(io::stderr(), "alz: {what}: {error}").unwrap_or_default();
            ExitCode::from(COULD_NOT_RUN_GIT)
        }
    }
}

/// Writes [`READY_BYTE`] to the standard output, where the caller reads it first.
fn announce_session() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(&[READY_BYTE])?;
    stdout.flush()
}

/// Whether the process that started this copy is still its parent. A process whose parent ended
/// is adopted by another, so its parent id changes.
fn parent_is_gone(parent: Option<i32>) -> bool {
    parent.is_none_or(|expected| {
        getppid().map(Pid::as_raw_nonzero).map(NonZero::get) != Some(expected)
    })
}

/// Waits for `git`; the copy is the leader of git's process group, so ending the group ends both.
fn wait_for(mut git: Child, parent: Option<i32>) -> ExitCode {
    let mut pause = FIRST_LOOK;
    loop {
        match git.try_wait() {
            Ok(Some(status)) => return exit_code_of(status),
            Ok(None) if !parent_is_gone(parent) => {
                thread::sleep(pause);
                pause = (pause * 2).min(LONGEST_WAIT);
            }
            Ok(None) | Err(_) => return abandon(),
        }
    }
}

/// Ends the whole process group, this copy included. The copy is its leader, so the group is
/// exactly git and what git started.
fn abandon() -> ExitCode {
    kill_process_group(getpid(), Signal::KILL).unwrap_or_default();
    // Only reached when the group could not be signalled.
    ExitCode::from(COULD_NOT_RUN_GIT)
}

/// git's exit code, or 128 plus the signal that ended it, as a shell reports it.
fn exit_code_of(status: ExitStatus) -> ExitCode {
    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| 128_i32.saturating_add(signal)))
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(COULD_NOT_RUN_GIT);
    ExitCode::from(code)
}
