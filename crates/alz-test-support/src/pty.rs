//! A pseudo-terminal driver for tests of programs that run only for a person at a terminal.
//!
//! [`run`] gives a program a pseudo-terminal as its standard input, with the terminal as its
//! controlling terminal in a session of its own, and forwards the bytes of an input stream to that
//! terminal. The program's standard output and standard error stay the driver's own, so a test
//! that starts the `pty-run` binary with pipes still tells the two streams apart. The bytes a test
//! sends are what a person types: a line, Ctrl-C (`0x03`), or the end of input, which the driver
//! sends as the terminal's end-of-file character (`0x04`) once the input stream ends.
//!
//! The terminal's echo of what is typed is read and dropped, so a full terminal buffer never
//! blocks the program. Everything is rustix and the standard library; there is no `unsafe` code.

mod invocation;
mod outcome;
mod terminal;

pub use invocation::{DEFAULT_DEADLINE, Invocation, UsageError};
pub use outcome::{DEADLINE_CODE, Outcome};

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use rustix::process::{Pid, Signal, kill_process_group};
use rustix::termios::tcsetpgrp;

/// The terminal's end-of-file character in its default settings.
const END_OF_FILE: u8 = 0x04;

/// What stops the driver from running the program.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    /// The terminal pair or the session could not be made.
    #[error("cannot set up the pseudo-terminal: {0}")]
    Terminal(#[source] io::Error),
    /// The program could not be started.
    #[error("cannot start the program: {0}")]
    Start(#[source] io::Error),
    /// The terminal could not be handed to the program's process group.
    #[error("cannot give the terminal to the program: {0}")]
    Foreground(#[source] io::Error),
    /// Waiting for the program failed.
    #[error("cannot wait for the program: {0}")]
    Wait(#[source] io::Error),
}

/// A [`PtyError`] that keeps the terminal open until it is dropped. Closing the terminal hangs it
/// up, and the hangup ends the driver by a signal while its own process group is the foreground
/// group, which it is until the program is started; holding the terminal lets the caller report
/// the error first.
#[derive(Debug, thiserror::Error)]
#[error("{error}")]
pub struct Failure {
    #[source]
    error: PtyError,
    _terminal: Option<terminal::Pair>,
}

impl Failure {
    /// What went wrong.
    #[must_use]
    pub fn error(&self) -> &PtyError {
        &self.error
    }
}

/// Runs the program on a new pseudo-terminal, feeding it the bytes of `input`, and reports how it
/// ended. At the end of `input` the terminal's end-of-file character is sent once. The program's
/// process group is killed if it runs past the invocation's deadline.
///
/// This makes the calling process the leader of a new session. Call it from a process made for
/// the purpose, as `pty-run` is, never from a test process.
///
/// # Errors
///
/// A [`Failure`] for each step that can fail before the program ends.
pub fn run(invocation: &Invocation, input: impl Read + Send + 'static) -> Result<Outcome, Failure> {
    let pair = terminal::open_pair().map_err(|error| Failure {
        error: PtyError::Terminal(error),
        _terminal: None,
    })?;
    match start(invocation, &pair) {
        Ok(child) => wait_for(invocation, input, pair, child).map_err(|error| Failure {
            error,
            _terminal: None,
        }),
        Err(error) => Err(Failure {
            error,
            _terminal: Some(pair),
        }),
    }
}

/// Starts the program on the terminal and makes its group the foreground group.
fn start(invocation: &Invocation, pair: &terminal::Pair) -> Result<Child, PtyError> {
    terminal::take_as_controlling_terminal(&pair.slave).map_err(PtyError::Terminal)?;
    let program_input = pair.slave.try_clone().map_err(PtyError::Terminal)?;
    let mut child = Command::new(invocation.program())
        .args(invocation.arguments())
        .stdin(Stdio::from(program_input))
        // Its own process group, so that the interrupt key reaches the program and not the driver.
        .process_group(0)
        .spawn()
        .map_err(PtyError::Start)?;
    let group = Pid::from_child(&child);
    give_foreground(&pair.slave, group, &mut child).inspect_err(|_| kill_group(group))?;
    Ok(child)
}

/// Feeds the terminal, drains its echo and waits for the program until the deadline.
fn wait_for(
    invocation: &Invocation,
    input: impl Read + Send + 'static,
    pair: terminal::Pair,
    child: Child,
) -> Result<Outcome, PtyError> {
    let group = Pid::from_child(&child);
    let typed = File::from(pair.master.try_clone().map_err(PtyError::Terminal)?);
    let echoed = File::from(pair.master);
    thread::spawn(move || type_into(input, typed));
    thread::spawn(move || drain(echoed));

    let (ended, waited) = mpsc::channel();
    let mut child = child;
    thread::spawn(move || {
        // The receiver is gone only after the deadline, when nobody wants the status any more.
        drop(ended.send(child.wait()));
    });
    match waited.recv_timeout(invocation.deadline()) {
        Ok(status) => status.map(Outcome::of).map_err(PtyError::Wait),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            kill_group(group);
            Ok(Outcome::KilledAtDeadline)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(PtyError::Wait(io::Error::other(
            "the thread that waits for the program ended",
        ))),
    }
}

/// Makes the program's group the foreground group of the terminal, then wakes it in case it
/// already tried to read before it was and was stopped for it.
fn give_foreground(
    slave: &std::os::fd::OwnedFd,
    group: Pid,
    child: &mut Child,
) -> Result<(), PtyError> {
    match tcsetpgrp(slave, group) {
        Ok(()) => {}
        // A program that has already ended has no group left to give the terminal to.
        Err(error) => {
            if child.try_wait().map_err(PtyError::Wait)?.is_none() {
                return Err(PtyError::Foreground(error.into()));
            }
        }
    }
    // A group that has already ended cannot be woken, and has no need of it.
    kill_process_group(group, Signal::CONT).unwrap_or_default();
    Ok(())
}

/// Kills the program and everything it started in its group.
fn kill_group(group: Pid) {
    // The group may have ended in the meantime, which is the aim.
    kill_process_group(group, Signal::KILL).unwrap_or_default();
}

/// Writes what the test sends to the terminal, then the end-of-file character when it ends.
fn type_into(mut input: impl Read, mut terminal: File) {
    let mut chunk = [0_u8; 4096];
    loop {
        match input.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let Some(bytes) = chunk.get(..read) else {
                    break;
                };
                if terminal.write_all(bytes).is_err() {
                    return;
                }
            }
        }
    }
    // The program may already have ended and closed its end.
    drop(terminal.write_all(&[END_OF_FILE]));
}

/// Reads and drops what the terminal gives back (the echo of what was typed).
fn drain(mut terminal: File) {
    let mut chunk = [0_u8; 4096];
    while matches!(terminal.read(&mut chunk), Ok(read) if read > 0) {}
}
