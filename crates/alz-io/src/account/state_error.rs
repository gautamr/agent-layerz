//! Why `allowed.yaml` was not written (spec §7.13 "Both files"). Every refusal is a value: the
//! command that asks renders it once. No variant holds text of the file or of the repository; the
//! only text is a path alz derived itself, stripped of control characters.

use std::io;
use std::num::NonZeroUsize;

use alz_core::{Message, Stripped};
use rustix::io::Errno;
use thiserror::Error;

use super::ignore::IgnoreReason;

/// A state of `allowed.yaml` that alz refuses to change, whatever the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateProblem {
    /// The reader ignores the file, or its place.
    Ignored(IgnoreReason),
    /// The top level is not a list, or the file is not YAML.
    NotAList,
    /// The file holds entries the reader ignores, by position, counted from 1.
    IgnoredEntries(Vec<NonZeroUsize>),
}

/// Why a change to `allowed.yaml` was refused or failed. Nothing is repaired: after a refusal the
/// file, its folder and the lock file are as they were.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// The place of the file is one the reader ignores.
    #[error("allowed.yaml cannot be written: {}", phrase(.0))]
    Place(IgnoreReason),
    /// The file is a link that does not resolve.
    #[error("allowed.yaml cannot be written: it is a link that does not resolve")]
    DanglingLink,
    /// The folder of the file, or a parent of it, cannot be found or is not a folder.
    #[error(
        "allowed.yaml cannot be written: its folder cannot be found or a parent of it is not a directory"
    )]
    FolderUnusable,
    /// A missing folder could not be created.
    #[error("allowed.yaml cannot be written: its folder {0} cannot be created")]
    FolderNotCreated(Stripped),
    /// The file is a link to a file alz did not write.
    #[error(
        "allowed.yaml is a link to a file that is not empty, [] or written by alz account allow; alz does not overwrite it"
    )]
    LinkedElsewhere,
    /// The file is in a state that the reader ignores or that alz does not repair.
    #[error("{}", problem_text(.0))]
    State(StateProblem),
    /// The lock file exists and stayed until the wait ended.
    #[error(
        "{0} exists, so another alz account allow may be running; if none is, delete that file and run this again"
    )]
    LockHeld(Stripped),
    /// The lock file cannot be created.
    #[error("the lock file {0} cannot be created ({1})")]
    LockUnusable(Stripped, Errno),
    /// The file or its place changed while it was being read.
    #[error("allowed.yaml changed while it was being read")]
    Changed,
    /// The result would be larger than the reader accepts.
    #[error("the result would be larger than 1 MiB, which alz ignores")]
    TooLarge,
    /// The result would not read back to the same entries.
    #[error("the result would not read back to the same entries")]
    WouldNotReadBack,
    /// The file was written but the reader does not read it back as written.
    #[error("allowed.yaml was written but does not read back")]
    WrittenButUnreadable,
    /// The target was swapped between the read and the rename; the old file is whole.
    #[error("allowed.yaml was replaced while it was being written; nothing was changed")]
    TargetChanged,
    /// A system call failed.
    #[error("allowed.yaml cannot be written ({0})")]
    Os(#[from] Errno),
    /// An I/O call failed.
    #[error("allowed.yaml cannot be written ({0})")]
    Io(#[from] io::Error),
}

impl StateError {
    /// Whether the file was replaced before the failure. Only a result that does not read back
    /// is; every other error leaves the file as it was.
    #[must_use]
    pub const fn file_was_replaced(&self) -> bool {
        matches!(self, Self::WrittenButUnreadable)
    }
}

fn phrase(reason: &IgnoreReason) -> String {
    reason.after(Message::new("")).to_string()
}

fn problem_text(problem: &StateProblem) -> String {
    match problem {
        StateProblem::Ignored(reason) => format!(
            "allowed.yaml is ignored ({}); fix it or delete it, alz does not repair it",
            phrase(reason)
        ),
        StateProblem::NotAList => {
            "allowed.yaml is not a list; fix it or delete it, alz does not repair it".to_owned()
        }
        StateProblem::IgnoredEntries(numbers) => {
            let listed = numbers
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let noun = if numbers.len() == 1 {
                "entry"
            } else {
                "entries"
            };
            format!(
                "allowed.yaml has {noun} {listed} that alz ignores; fix or delete the file, alz does not repair it"
            )
        }
    }
}
