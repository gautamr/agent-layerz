//! Starting one git process and waiting for it, within a deadline (spec §6 "Git subprocesses").
//!
//! The process is the launcher's copy of alz (see `detach`), started with a cleared environment,
//! the null device or a pipe for standard input, and pipes for the other two streams. Each stream
//! is read on a thread of its own, so a full pipe can never stop git, and the caller waits on
//! channels with the one deadline. When the deadline passes, git's whole process group is ended.
//!
//! Once the copy has been waited for, its process id is never signalled again, because the system
//! may have given the id to another process. The streams are then read for a short grace period
//! only: a descendant of git that left the process group and keeps a pipe open (an `ssh` that
//! stays on for later connections) does not hold the call until its deadline.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use alz_core::env::EnvVars;
use rustix::process::{Pid, Signal, kill_process, kill_process_group};

use super::detach::{Launcher, READY_BYTE};
use super::failure::GitFailure;

/// How much of git's standard error is kept: the first 64 KiB.
const STDERR_KEPT: u64 = 64 * 1024;

/// How long to wait, after ending the process group, for git to be reaped.
const REAP_GRACE: Duration = Duration::from_secs(5);

/// How long the streams are read after the copy has ended, when a descendant keeps them open.
const PIPE_GRACE: Duration = Duration::from_millis(500);

/// How long, after the copy was ended, to wait for it to have announced its session.
const ANNOUNCEMENT_GRACE: Duration = Duration::from_secs(1);

/// What a read of a stream takes at a time.
const CHUNK: usize = 8 * 1024;

/// One process to run.
#[derive(Debug)]
pub(super) struct Invocation<'a> {
    pub launcher: &'a Launcher,
    /// The absolute path of the git that the copy runs.
    pub git: &'a Path,
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

/// What a stream's thread hands over.
enum Piece {
    /// The next bytes, within the limit.
    Bytes(Vec<u8>),
    /// A byte past the limit was read and dropped.
    Dropped,
    /// The stream failed.
    Failed(io::Error),
}

/// Runs `work` on a thread of its own; its result arrives on the receiver. The thread is not
/// joined: when it has nothing to deliver to a caller that stopped waiting, it ends on its own.
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

/// `chunk` without its first byte when that is [`READY_BYTE`], and whether it was.
fn without_announcement(chunk: &[u8]) -> (&[u8], bool) {
    match chunk.split_first() {
        Some((&READY_BYTE, rest)) => (rest, true),
        Some(_) | None => (chunk, false),
    }
}

/// Reads `pipe` to its end. The bytes within `limit` go to `pieces`, the rest is dropped, so the
/// writer never blocks and memory stays bounded. When `announcement` is given, the first byte of
/// the stream is expected to be [`READY_BYTE`]: it is taken off and reported there. Reading goes
/// on after the caller stopped listening, so a writer that outlives the call is not cut off.
fn pump(mut pipe: impl Read, limit: u64, announcement: Option<Sender<()>>, pieces: &Sender<Piece>) {
    let mut room = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut dropped = false;
    let mut announcement = announcement;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return,
            Ok(count) => {
                let chunk = buffer.get(..count).unwrap_or_default();
                let chunk = match announcement.take() {
                    Some(ready) => {
                        let (rest, announced) = without_announcement(chunk);
                        if announced {
                            ready.send(()).unwrap_or_default();
                        }
                        rest
                    }
                    None => chunk,
                };
                let kept = chunk.len().min(room);
                room -= kept;
                if kept > 0 {
                    pieces
                        .send(Piece::Bytes(chunk.get(..kept).unwrap_or_default().to_vec()))
                        .unwrap_or_default();
                }
                if kept < chunk.len() && !dropped {
                    dropped = true;
                    pieces.send(Piece::Dropped).unwrap_or_default();
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                pieces.send(Piece::Failed(error)).unwrap_or_default();
                return;
            }
        }
    }
}

/// Starts the thread that reads `pipe`.
fn stream(
    pipe: impl Read + Send + 'static,
    limit: u64,
    announcement: Option<Sender<()>>,
) -> io::Result<Receiver<Piece>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new().spawn(move || pump(pipe, limit, announcement, &sender))?;
    Ok(receiver)
}

/// A stream as far as it was read.
struct Gathered {
    bytes: Vec<u8>,
    truncated: bool,
}

/// Everything that arrives on `pieces` until the stream ends or `until` passes. A stream that has
/// not ended by then is taken as it is.
fn gather(pieces: &Receiver<Piece>, until: Instant) -> Result<Gathered, GitFailure> {
    let mut gathered = Gathered {
        bytes: Vec::new(),
        truncated: false,
    };
    loop {
        match pieces.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(Piece::Bytes(bytes)) => gathered.bytes.extend_from_slice(&bytes),
            Ok(Piece::Dropped) => gathered.truncated = true,
            Ok(Piece::Failed(error)) => return Err(GitFailure::Io(error)),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                return Ok(gathered);
            }
        }
    }
}

/// What the caller waits for.
struct Watch {
    status: Receiver<io::Result<ExitStatus>>,
    stdout: Receiver<Piece>,
    stderr: Receiver<Piece>,
}

fn missing_pipe() -> io::Error {
    io::Error::other("a pipe of the git process is missing")
}

/// Starts a thread for each stream and for the wait. `announcement` receives a message when the
/// copy has started its own session.
fn watch(
    mut child: Child,
    input: Option<Vec<u8>>,
    stdout_limit: u64,
    announcement: Sender<()>,
) -> io::Result<Watch> {
    let stdout = child.stdout.take().ok_or_else(missing_pipe)?;
    let stderr = child.stderr.take().ok_or_else(missing_pipe)?;
    if let Some(bytes) = input {
        let mut stdin = child.stdin.take().ok_or_else(missing_pipe)?;
        // A git that stops reading before the end is told by its exit status, so a broken pipe
        // here is not an error of its own.
        in_thread(move || stdin.write_all(&bytes).unwrap_or_default())?;
    }
    Ok(Watch {
        stdout: stream(stdout, stdout_limit, Some(announcement))?,
        stderr: stream(stderr, STDERR_KEPT, None)?,
        status: in_thread(move || child.wait())?,
    })
}

/// Why waiting stopped before git finished.
enum Stop {
    Expired,
    Failed(GitFailure),
}

/// The status of the copy, if it comes before `until`.
fn next_status(
    receiver: &Receiver<io::Result<ExitStatus>>,
    until: Instant,
) -> Result<ExitStatus, Stop> {
    match receiver.recv_timeout(until.saturating_duration_since(Instant::now())) {
        Ok(result) => result.map_err(|error| Stop::Failed(GitFailure::Io(error))),
        Err(RecvTimeoutError::Timeout) => Err(Stop::Expired),
        Err(RecvTimeoutError::Disconnected) => Err(Stop::Failed(GitFailure::Io(io::Error::other(
            "a watcher of the git process stopped",
        )))),
    }
}

/// Ends the copy, and git's whole process group once the copy has announced that it leads one.
///
/// The group is the copy's process id only after the copy ran `setsid`, which it reports with one
/// byte (see `detach`). If the byte has arrived, the group is ended first, then the copy. If not,
/// the copy is ended alone, and then the byte is waited for a moment: the copy may have got as far
/// as starting git before it was ended, and git is then alone in a group nobody else would end. A
/// process that is already gone is not an error.
fn end_call(process: u32, announcement: &Receiver<()>) {
    if let Some(process) = i32::try_from(process).ok().and_then(Pid::from_raw) {
        let announced = announcement.try_recv().is_ok();
        if announced {
            kill_process_group(process, Signal::KILL).unwrap_or_default();
        }
        kill_process(process, Signal::KILL).unwrap_or_default();
        if !announced && announcement.recv_timeout(ANNOUNCEMENT_GRACE).is_ok() {
            kill_process_group(process, Signal::KILL).unwrap_or_default();
        }
    }
}

/// A call that has started and whose copy has not yet been waited for. When it is dropped before
/// that, the call was abandoned (an error, a panic, a thread that unwinds), and git's whole
/// process group is ended, so no git or `ssh` is left behind. The copy itself also ends the group
/// when this process is gone altogether (see `detach`).
struct Running {
    process: Option<u32>,
    announcement: Receiver<()>,
}

impl Running {
    /// Ends the call now, and nothing more happens when this is dropped.
    fn end(&mut self) {
        if let Some(process) = self.process.take() {
            end_call(process, &self.announcement);
        }
    }

    /// The copy was waited for: its process id may belong to another process from now on, so it
    /// is never signalled again.
    fn waited_for(&mut self) {
        self.process = None;
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.end();
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
    let mut command = invocation
        .launcher
        .command(invocation.git, invocation.arguments);
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
    let (announce, announcement) = mpsc::channel();
    let mut running = Running {
        process: Some(child.id()),
        announcement,
    };
    let watch = watch(child, invocation.input, invocation.stdout_limit, announce)
        .map_err(GitFailure::Io)?;
    match next_status(&watch.status, until) {
        Ok(status) => {
            running.waited_for();
            let pipes_until = until.min(Instant::now() + PIPE_GRACE);
            let stdout = gather(&watch.stdout, pipes_until)?;
            let stderr = gather(&watch.stderr, pipes_until)?;
            Ok(Completed {
                status,
                stdout: stdout.bytes,
                stdout_truncated: stdout.truncated,
                stderr: stderr.bytes,
            })
        }
        // Dropping `running` ends the call.
        Err(Stop::Failed(failure)) => Err(failure),
        Err(Stop::Expired) => {
            running.end();
            // Reap the copy so that no zombie stays behind; its result is of no use now.
            drop(watch.status.recv_timeout(REAP_GRACE));
            Err(GitFailure::TimedOut {
                after: invocation.deadline,
            })
        }
    }
}
