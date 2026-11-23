//! Starting one git process and waiting for it, within a deadline (spec §6 "Git subprocesses").
//!
//! The process is the launcher's copy of alz (see `detach`), started with a cleared environment,
//! the null device or a pipe for standard input, and pipes for the other two streams. Each stream
//! is handled on a thread of its own, so a full pipe can never stop git, and the caller waits on
//! channels with the one deadline. When the deadline passes, git's whole process group is ended.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use alz_core::env::EnvVars;
use rustix::process::{Pid, Signal, kill_process, kill_process_group};

use super::detach::Launcher;
use super::failure::GitFailure;

/// How much of git's standard error is kept: the first 64 KiB.
const STDERR_KEPT: u64 = 64 * 1024;

/// How long to wait, after ending the process group, for git to be reaped.
const REAP_GRACE: Duration = Duration::from_secs(5);

/// One process to run.
#[derive(Debug)]
pub(super) struct Invocation<'a> {
    pub launcher: &'a Launcher,
    pub arguments: &'a [OsString],
    pub environment: &'a EnvVars,
    pub directory: Option<&'a Path>,
    pub input: Option<Vec<u8>>,
    pub deadline: Duration,
    pub stdout_limit: u64,
}

/// A process that ended by itself.
#[derive(Debug)]
pub(super) struct Completed {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    /// Whether git wrote more than the limit; the rest was read and dropped.
    pub stdout_truncated: bool,
    pub stderr: Vec<u8>,
}

/// A stream read up to a limit.
struct Capped {
    bytes: Vec<u8>,
    truncated: bool,
}

/// The first `limit` bytes of `pipe`; the rest is read to the end and dropped, so the writer never
/// blocks.
fn read_capped(mut pipe: impl Read, limit: u64) -> io::Result<Capped> {
    let mut bytes = Vec::new();
    pipe.by_ref().take(limit).read_to_end(&mut bytes)?;
    let dropped = io::copy(&mut pipe, &mut io::sink())?;
    Ok(Capped {
        bytes,
        truncated: dropped > 0,
    })
}

/// Runs `work` on a thread of its own; its result arrives on the receiver. The thread is not
/// joined: when a stream stays open past the deadline (a descendant that left the group), the
/// thread ends with the process.
fn in_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> io::Result<Receiver<T>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new().spawn(move || {
        // A caller that stopped waiting dropped the receiver, and nobody needs the result.
        sender.send(work()).unwrap_or_default();
    })?;
    Ok(receiver)
}

/// What the caller waits for.
struct Watch {
    status: Receiver<io::Result<ExitStatus>>,
    stdout: Receiver<io::Result<Capped>>,
    stderr: Receiver<io::Result<Capped>>,
}

fn missing_pipe() -> io::Error {
    io::Error::other("a pipe of the git process is missing")
}

/// Starts a thread for each stream and for the wait.
fn watch(mut child: Child, input: Option<Vec<u8>>, stdout_limit: u64) -> io::Result<Watch> {
    let stdout = child.stdout.take().ok_or_else(missing_pipe)?;
    let stderr = child.stderr.take().ok_or_else(missing_pipe)?;
    if let Some(bytes) = input {
        let mut stdin = child.stdin.take().ok_or_else(missing_pipe)?;
        // A git that stops reading before the end is told by its exit status, so a broken pipe
        // here is not an error of its own.
        in_thread(move || stdin.write_all(&bytes).unwrap_or_default())?;
    }
    Ok(Watch {
        stdout: in_thread(move || read_capped(stdout, stdout_limit))?,
        stderr: in_thread(move || read_capped(stderr, STDERR_KEPT))?,
        status: in_thread(move || child.wait())?,
    })
}

/// Why waiting stopped before git finished.
enum Stop {
    Expired,
    Failed(GitFailure),
}

/// The next value on `receiver`, if it comes before `until`.
fn next<T>(receiver: &Receiver<io::Result<T>>, until: Instant) -> Result<T, Stop> {
    match receiver.recv_timeout(until.saturating_duration_since(Instant::now())) {
        Ok(result) => result.map_err(|error| Stop::Failed(GitFailure::Io(error))),
        Err(RecvTimeoutError::Timeout) => Err(Stop::Expired),
        Err(RecvTimeoutError::Disconnected) => Err(Stop::Failed(GitFailure::Io(io::Error::other(
            "a watcher of the git process stopped",
        )))),
    }
}

/// Ends the process group of the copy, which became the leader of its own session when it ran
/// `setsid`. If it did not get that far, the copy alone is ended. A process that is already gone
/// is not an error.
fn end_group(process: u32) {
    if let Some(process) = i32::try_from(process).ok().and_then(Pid::from_raw) {
        kill_process_group(process, Signal::KILL)
            .or_else(|_| kill_process(process, Signal::KILL))
            .unwrap_or_default();
    }
}

/// Runs `invocation` and returns how it ended.
///
/// # Errors
///
/// [`GitFailure::Spawn`] when the launcher cannot be started, [`GitFailure::TimedOut`] when the
/// deadline passes (git's process group is ended first) and [`GitFailure::Io`] when a stream
/// fails.
pub(super) fn run(invocation: Invocation<'_>) -> Result<Completed, GitFailure> {
    let mut command = invocation.launcher.command(invocation.arguments);
    command
        .env_clear()
        .envs(invocation.environment.iter())
        .stdin(if invocation.input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = invocation.directory {
        command.current_dir(directory);
    }
    let until = Instant::now() + invocation.deadline;
    let child = command.spawn().map_err(GitFailure::Spawn)?;
    let process = child.id();
    let watch = watch(child, invocation.input, invocation.stdout_limit).map_err(|error| {
        end_group(process);
        GitFailure::Io(error)
    })?;
    let completed = next(&watch.status, until).and_then(|status| {
        let stdout = next(&watch.stdout, until)?;
        let stderr = next(&watch.stderr, until)?;
        Ok(Completed {
            status,
            stdout: stdout.bytes,
            stdout_truncated: stdout.truncated,
            stderr: stderr.bytes,
        })
    });
    match completed {
        Ok(completed) => Ok(completed),
        Err(Stop::Failed(failure)) => {
            end_group(process);
            Err(failure)
        }
        Err(Stop::Expired) => {
            end_group(process);
            // Reap the copy so that no zombie stays behind; its result is of no use now.
            drop(watch.status.recv_timeout(REAP_GRACE));
            Err(GitFailure::TimedOut {
                after: invocation.deadline,
            })
        }
    }
}
