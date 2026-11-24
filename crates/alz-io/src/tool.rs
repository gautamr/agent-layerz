//! Running the `claude` program (spec §10.3 `alz doctor`, `alz ls --live`): one absolute path
//! found on a validated `PATH`, an argument array and no shell, a cleared environment with only
//! what the call needs, a deadline that ends the program's whole process group, and output that
//! is read within a cap. The bytes that come back are untrusted; the caller strips what it shows.

mod environment;
mod locate;
mod pipes;
mod provider;

use std::ffi::OsString;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process_group};
use thiserror::Error;

use crate::env::EnvSnapshot;

use self::environment::child_environment;
use self::locate::locate;
use self::pipes::{Gathered, gather, stream};

pub use self::environment::Purpose;

/// How long to wait, after the process group was ended, for the program to be reaped.
const REAP_GRACE: Duration = Duration::from_secs(5);

/// How long the streams are read after the program ended, when a descendant keeps them open.
const PIPE_GRACE: Duration = Duration::from_millis(500);

/// Why a run did not give its output.
#[derive(Debug, Error)]
pub enum ToolError {
    /// No executable file of that name is on `PATH`, or the configured path is not one.
    #[error("not found")]
    NotFound,
    /// The program could not be started.
    #[error("cannot start it")]
    Spawn(#[source] io::Error),
    /// A pipe failed, or a thread for it could not be started.
    #[error("cannot talk to it")]
    Io(#[source] io::Error),
    /// The deadline passed; the program's process group was ended.
    #[error("it did not finish within {} seconds", .after.as_secs())]
    TimedOut {
        /// The deadline that passed.
        after: Duration,
    },
}

/// What a program that ended by itself wrote. Untrusted bytes.
#[derive(Debug)]
pub struct ToolOutput {
    /// Whether it exited with status 0.
    pub success: bool,
    /// Standard output, at most the cap of the purpose.
    pub stdout: Vec<u8>,
    /// Standard error, at most 64 KiB.
    pub stderr: Vec<u8>,
}

/// The `claude` program, found once.
#[derive(Debug, Clone)]
pub struct ToolRunner {
    program: PathBuf,
    purpose: Purpose,
    deadline: Duration,
    environment: alz_core::env::EnvVars,
}

impl ToolRunner {
    /// Finds `claude` for `purpose`: the program that `ALZ_CLAUDE_BIN` names (an absolute path,
    /// or a name looked up on `PATH`), else `claude` on `PATH`. Only the absolute entries of
    /// `PATH` count, so a program in the working directory, which may be a repository about to be
    /// read, is never taken.
    ///
    /// # Errors
    ///
    /// [`ToolError::NotFound`] when no executable regular file is found.
    pub fn claude(env: &EnvSnapshot, purpose: Purpose) -> Result<Self, ToolError> {
        let program = locate(env).ok_or(ToolError::NotFound)?;
        Ok(Self {
            program,
            purpose,
            deadline: purpose.deadline(),
            environment: child_environment(purpose, env.vars()),
        })
    }

    /// The same runner with another deadline, for a test.
    #[must_use]
    pub const fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// The absolute path of the program that runs.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Runs the program with `arguments` in `directory` and waits for it.
    ///
    /// # Errors
    ///
    /// [`ToolError::Spawn`] when it cannot be started, [`ToolError::TimedOut`] when the deadline
    /// passes (its process group is ended first) and [`ToolError::Io`] when a stream fails.
    pub fn run(&self, arguments: &[OsString], directory: &Path) -> Result<ToolOutput, ToolError> {
        let mut command = Command::new(&self.program);
        command
            .args(arguments)
            .env_clear()
            .envs(self.environment.iter())
            .current_dir(directory)
            // Its own process group, so the deadline can end the program and what it started.
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let until = Instant::now() + self.deadline;
        let mut child = command.spawn().map_err(ToolError::Spawn)?;
        let group = i32::try_from(child.id()).ok().and_then(Pid::from_raw);
        let missing = || ToolError::Io(io::Error::other("a pipe of the program is missing"));
        let stdout = child.stdout.take().ok_or_else(missing)?;
        let stderr = child.stderr.take().ok_or_else(missing)?;
        let stdout = stream(stdout, self.purpose.stdout_cap()).map_err(ToolError::Io)?;
        let stderr = stream(stderr, Purpose::STDERR_CAP).map_err(ToolError::Io)?;
        let (sender, status) = mpsc::channel();
        thread::Builder::new()
            .spawn(move || sender.send(child.wait()).unwrap_or_default())
            .map_err(ToolError::Io)?;
        match status.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(waited) => {
                let status = waited.map_err(ToolError::Io)?;
                let pipes_until = until.min(Instant::now() + PIPE_GRACE);
                let Gathered { bytes: stdout } = gather(&stdout, pipes_until)?;
                let Gathered { bytes: stderr } = gather(&stderr, pipes_until)?;
                Ok(ToolOutput {
                    success: status.success(),
                    stdout,
                    stderr,
                })
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(group) = group {
                    // A group that is already gone is not an error.
                    kill_process_group(group, Signal::KILL).unwrap_or_default();
                }
                // Reap the program so that no zombie stays behind; its result is of no use now.
                drop(status.recv_timeout(REAP_GRACE));
                Err(ToolError::TimedOut {
                    after: self.deadline,
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(ToolError::Io(io::Error::other(
                "a watcher of the program stopped",
            ))),
        }
    }
}
