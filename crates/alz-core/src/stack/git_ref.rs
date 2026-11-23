//! A git tag, branch or commit named in the stack (spec §5.2).

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Printable, sealed::Sealed};
use crate::lock::ObjectId;

/// A git ref: non-empty, never starting with `-` (so git cannot read it as an option), holding no
/// `..`, and made only of ASCII letters, digits and `. _ / + -`. Its alphabet is printable ASCII,
/// so it goes into a message as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitRef(String);

/// Text that is not a git ref. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a git ref: letters, digits and . _ / + -, not starting with '-' and without '..'")]
pub struct GitRefError;

impl GitRefError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be a git ref: letters, digits and . _ / + -, not starting with '-' and without '..'"
    }
}

impl GitRef {
    /// The ref as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The ref that names `commit` by its full id: lowercase hexadecimal is within the alphabet,
    /// so no check is needed.
    #[must_use]
    pub fn of_commit(commit: &ObjectId) -> Self {
        Self(commit.to_string())
    }

    /// The first 12 characters of `commit`'s id: what `layer.lock` records as `rev` for a
    /// `git:` or `github:` layer (§5.3).
    #[must_use]
    pub fn short_of(commit: &ObjectId) -> Self {
        Self(commit.to_string().chars().take(12).collect())
    }
}

fn is_ref_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'/' | b'+' | b'-')
}

impl TryFrom<&str> for GitRef {
    type Error = GitRefError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let well_formed = !text.is_empty()
            && !text.starts_with('-')
            && !text.contains("..")
            && text.bytes().all(is_ref_character);
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(GitRefError)
        }
    }
}

impl fmt::Display for GitRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Sealed for GitRef {}

impl Printable for GitRef {
    fn printable(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{GitRef, GitRefError};

    #[test]
    fn a_ref_follows_the_alphabet_and_the_two_prohibitions() {
        for good in [
            "v1.0.0",
            "layer-v0.3.0",
            "feature/x+y_z",
            "1.10",
            "a.b",
            "0123abc",
            "a-",
        ] {
            assert_eq!(GitRef::try_from(good).expect("a ref").as_str(), good);
        }
        for bad in [
            "",
            "-x",
            "--upload-pack=touch",
            "a..b",
            "..",
            "a b",
            "a:b",
            "a@b",
            "a~1",
            "a^",
            "é",
            "a\n",
            "a\0",
            "a\\b",
        ] {
            assert_eq!(GitRef::try_from(bad), Err(GitRefError), "{bad:?}");
        }
    }
}
