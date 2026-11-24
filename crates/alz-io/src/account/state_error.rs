//! Why `allowed.yaml` was not written (spec §7.13 "Both files"). Every refusal is a value: the
//! command that asks renders it once. No variant holds text of the file or of the repository; the
//! only text is a path alz derived itself, stripped of control characters.

use std::fmt;
use std::io;

use alz_core::account::{AllowCommand, AllowFileProblem, Refusal, unwritable};
use alz_core::{Message, Stripped};
use rustix::io::Errno;
use thiserror::Error;

use super::files::{FileStatus, Loaded};
use super::ignore::IgnoreReason;
use super::resolved::ResolvedAllowState;

/// Why `allowed.yaml` cannot be used as a whole, as the reader found it: ignored, not a list, or
/// holding ignored entries. `None` when it is absent or whole.
#[must_use]
pub fn allow_file_problem(loaded: &Loaded<ResolvedAllowState>) -> Option<AllowFileProblem> {
    match loaded.status() {
        FileStatus::NotFound => None,
        FileStatus::Ignored(reason) => Some(ignored(reason)),
        FileStatus::Read if !loaded.value().is_list() => Some(AllowFileProblem::NotAList),
        FileStatus::Read => Some(loaded.value().ignored().to_vec())
            .filter(|entries| !entries.is_empty())
            .map(AllowFileProblem::Entries),
    }
}

/// The problem of a file the reader ignores, with the reader's own reason.
pub(super) fn ignored(reason: &IgnoreReason) -> AllowFileProblem {
    AllowFileProblem::Ignored(phrase(reason))
}

/// Why a change to `allowed.yaml` was refused or failed. Nothing is repaired: after a refusal the
/// file, its folder and the lock file are as they were.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// The place of the file is one the reader ignores.
    #[error("{}", self.words())]
    Place(IgnoreReason),
    /// The file is a link that does not resolve.
    #[error("{}", self.words())]
    DanglingLink,
    /// The folder of the file, or a parent of it, cannot be found or is not a folder.
    #[error("{}", self.words())]
    FolderUnusable,
    /// A missing folder could not be created.
    #[error("{}", self.words())]
    FolderNotCreated(Stripped),
    /// The file is a link to a file alz did not write.
    #[error("{}", self.words())]
    LinkedElsewhere,
    /// The file is in a state that the reader ignores or that alz does not repair.
    #[error("{}", self.words())]
    State(AllowFileProblem),
    /// The lock file exists and stayed until the wait ended.
    #[error("{}", self.words())]
    LockHeld(Stripped),
    /// The lock file cannot be created.
    #[error("{}", self.words())]
    LockUnusable(Stripped, Errno),
    /// The file or its place changed while it was being read.
    #[error("{}", self.words())]
    Changed,
    /// The result would be larger than the reader accepts.
    #[error("{}", self.words())]
    TooLarge,
    /// The result would not read back to the same entries.
    #[error("{}", self.words())]
    WouldNotReadBack,
    /// The file was written but the reader does not read it back as written.
    #[error("{}", self.words())]
    WrittenButUnreadable,
    /// The target was swapped between the read and the rename; the old file is whole.
    #[error("{}", self.words())]
    TargetChanged,
    /// A system call failed.
    #[error("{}", self.words())]
    Os(#[from] Errno),
    /// An I/O call failed.
    #[error("{}", self.words())]
    Io(#[from] io::Error),
}

/// How a failure is worded: by the state of the file, by what the writer found at its place, or
/// in the writer's own words.
enum Words<'a> {
    State(&'a AllowFileProblem),
    Unwritable(Message),
    Plain(Message),
}

impl StateError {
    /// Whether the file was replaced before the failure. Only a result that does not read back
    /// is; every other error leaves the file as it was.
    #[must_use]
    pub const fn file_was_replaced(&self) -> bool {
        matches!(self, Self::WrittenButUnreadable)
    }

    /// The failure as `command` refuses with it (§10.3), to be worded by the refusal's own text.
    #[must_use]
    pub fn refusal(&self, command: AllowCommand) -> Refusal<'_> {
        match self.words() {
            Words::State(problem) => Refusal::State(problem),
            Words::Unwritable(problem) => Refusal::Unwritable { command, problem },
            Words::Plain(text) => Refusal::Writer(text),
        }
    }

    fn words(&self) -> Words<'_> {
        match self {
            Self::State(problem) => Words::State(problem),
            Self::Place(reason) => Words::Unwritable(phrase(reason)),
            Self::DanglingLink => Words::Unwritable(Message::new("it is a link that does not resolve")),
            Self::FolderUnusable => Words::Unwritable(Message::new(
                "its folder cannot be found or a parent of it is not a directory",
            )),
            Self::FolderNotCreated(folder) => Words::Unwritable(
                Message::new("its folder ")
                    .then(folder.clone())
                    .then(" cannot be created"),
            ),
            Self::Os(errno) => Words::Unwritable(system_text(&errno.to_string())),
            Self::Io(error) => Words::Unwritable(system_text(&error.to_string())),
            Self::LinkedElsewhere => Words::Plain(Message::new(
                "allowed.yaml is a link to a file that is not empty, [] or written by alz account allow; alz does not overwrite it",
            )),
            Self::LockHeld(lock) => Words::Plain(Message::new(lock.clone()).then(
                " exists, so another alz account allow may be running; if none is, delete that file and run this again",
            )),
            Self::LockUnusable(lock, errno) => Words::Plain(
                Message::new("the lock file ")
                    .then(lock.clone())
                    .then(" cannot be created (")
                    .then(Stripped::from_untrusted(&errno.to_string()))
                    .then(")"),
            ),
            Self::Changed => Words::Plain(Message::new("allowed.yaml changed while it was being read")),
            Self::TooLarge => Words::Plain(Message::new(
                "the result would be larger than 1 MiB, which alz ignores",
            )),
            Self::WouldNotReadBack => Words::Plain(Message::new(
                "the result would not read back to the same entries",
            )),
            Self::WrittenButUnreadable => {
                Words::Plain(Message::new("allowed.yaml was written but does not read back"))
            }
            Self::TargetChanged => Words::Plain(Message::new(
                "allowed.yaml was replaced while it was being written; nothing was changed",
            )),
        }
    }
}

/// The text of a failure of the system, stripped of control characters.
fn system_text(text: &str) -> Message {
    Message::new(Stripped::from_untrusted(text))
}

/// The reader's reason, as a sentence part.
fn phrase(reason: &IgnoreReason) -> Message {
    reason.after(Message::new(""))
}

/// The failure as the tool prints it when it is not refused as a command: the writer's words with
/// `allowed.yaml cannot be written: ` before the ones about its place.
impl fmt::Display for Words<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(problem) => problem.state().fmt(formatter),
            Self::Unwritable(problem) => unwritable(AllowCommand::Allow, problem).fmt(formatter),
            Self::Plain(text) => text.fmt(formatter),
        }
    }
}
