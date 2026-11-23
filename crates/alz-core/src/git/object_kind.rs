//! The kinds of object git stores, as `ls-tree` and `cat-file` print them, and the error for
//! output that does not read as expected.

use thiserror::Error;

/// A git object type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// File contents.
    Blob,
    /// A directory listing.
    Tree,
    /// A commit.
    Commit,
    /// An annotated tag.
    Tag,
}

impl ObjectKind {
    /// The kind named by git's type word (`blob`, `tree`, `commit`, `tag`), if it is one.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "blob" => Some(Self::Blob),
            "tree" => Some(Self::Tree),
            "commit" => Some(Self::Commit),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
}

/// Output of git that does not have the shape its call asks for. `what` names the part that is
/// wrong, as a fixed phrase: git's own text, which is untrusted, is never in the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("unreadable git output: {what}")]
pub struct UnreadableOutput {
    /// The part of the output that is wrong.
    pub what: &'static str,
}

impl UnreadableOutput {
    /// An error about `what`.
    #[must_use]
    pub const fn new(what: &'static str) -> Self {
        Self { what }
    }
}
