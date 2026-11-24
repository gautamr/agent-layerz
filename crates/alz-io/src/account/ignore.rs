//! Why an account file is ignored (spec §7.13 "Both files"). Every reason is a fixed phrase; the
//! only text a message can hold besides is a path alz derived itself (the file's place, a
//! directory above it, the `.git` that was found), stripped of control characters, and never any
//! text of the file.

use alz_core::{Message, Stripped};

/// What is wrong with a directory above an account file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryProblem {
    /// It could not be looked at.
    CannotBeChecked,
    /// Group or others can write it.
    GroupOrOthersCanWrite,
    /// The user does not own it (root is accepted for a directory outside `$HOME`).
    NotOwned,
}

/// The first check of "Both files" that an account file failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoreReason {
    /// `$HOME` is not an absolute path.
    HomeNotAbsolute,
    /// The user id cannot be read.
    NoUserId,
    /// The path as located names an account folder, so no file system call is made on it.
    NamesAccountFolder,
    /// The `XDG_*` variable points away from its default while mise or direnv is active.
    XdgMoved(&'static str),
    /// The path cannot be resolved to a real one.
    PathUnresolvable,
    /// The real path lies inside an account folder.
    InsideAccountFolder,
    /// The real path lies inside the current repository.
    InsideRepository,
    /// `allowed.yaml` lies inside a git work tree; the `.git` that was found.
    InsideWorkTree(Stripped),
    /// Not a regular file of at most 1 MiB that can be read.
    Unreadable,
    /// The user does not own the file.
    NotOwned,
    /// Group or others can write the file.
    GroupOrOthersCanWrite,
    /// The file was read but is not valid YAML, or is past a YAML limit.
    NotValidYaml,
    /// A directory above the file fails its check.
    Directory {
        /// What is wrong.
        problem: DirectoryProblem,
        /// The directory, as shown.
        directory: Stripped,
    },
}

impl IgnoreReason {
    /// `head` followed by the reason.
    #[must_use]
    pub fn after(&self, head: Message) -> Message {
        match self {
            Self::HomeNotAbsolute => head.then("HOME is not an absolute path"),
            Self::NoUserId => head.then("the owner of the file cannot be checked on this platform"),
            Self::XdgMoved(variable) => head
                .then(*variable)
                .then(" points away from its default and mise or direnv is active here"),
            Self::PathUnresolvable => head.then("its path cannot be resolved"),
            Self::NamesAccountFolder | Self::InsideAccountFolder => {
                head.then("it lies inside an account folder")
            }
            Self::InsideRepository => head.then("it lies inside the current repository"),
            Self::InsideWorkTree(marker) => head
                .then("it lies inside a git work tree (")
                .then(marker.clone())
                .then(")"),
            Self::Unreadable => {
                head.then("it is not a regular file of at most 1 MiB that can be read")
            }
            Self::NotOwned => head.then("it is not owned by you"),
            Self::GroupOrOthersCanWrite => head.then("group or others can write it"),
            Self::NotValidYaml => head.then("it is not valid YAML"),
            Self::Directory { problem, directory } => {
                let lead = head.then("the directory ").then(directory.clone());
                match problem {
                    DirectoryProblem::CannotBeChecked => lead.then(" cannot be checked"),
                    DirectoryProblem::GroupOrOthersCanWrite => {
                        lead.then(" can be written by group or others")
                    }
                    DirectoryProblem::NotOwned => lead.then(" is not owned by you"),
                }
            }
        }
    }
}
