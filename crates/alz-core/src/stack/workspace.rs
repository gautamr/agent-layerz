//! The `workspace` of the stack manifest (spec §5.2): the client hub and the additional
//! directories each target may read and write. They stay relative to the repository, so the
//! generated output is the same on every machine.

use std::collections::BTreeSet;
use std::fmt;

use thiserror::Error;

use crate::schema::{Checked, refine, refine_list, unknown_fields, zip3};
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};

/// Text that is not a workspace directory. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkspaceDirError {
    /// Nothing, or only white space.
    #[error("a workspace directory is a non-empty path")]
    Empty,
    /// An absolute path, or one that starts with `~`.
    #[error("a workspace directory is relative to the repository")]
    NotRelative,
}

impl WorkspaceDirError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Empty => "must be a non-empty path",
            Self::NotRelative => {
                "must be relative to the repository: not absolute and not starting with '~'"
            }
        }
    }
}

/// A directory relative to the repository, normalised: `\` read as `/`, `.` and empty segments
/// and a trailing `/` removed, `..` resolved against the segment before it. It may start with
/// `..` (`../hub`), and is `.` when nothing is left.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceDir(String);

impl WorkspaceDir {
    /// The directory as normalised text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The segments of `text` with `.` and empty ones dropped and each `..` that can cancel the
/// segment before it doing so.
fn normalised_segments(text: &str) -> Vec<&str> {
    text.split('/')
        .filter(|segment| !matches!(*segment, "" | "."))
        .fold(Vec::new(), |mut kept, segment| {
            if segment == ".." && kept.last().is_some_and(|last| *last != "..") {
                kept.pop();
            } else {
                kept.push(segment);
            }
            kept
        })
}

impl TryFrom<&str> for WorkspaceDir {
    type Error = WorkspaceDirError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let read_as_slashes = text.replace('\\', "/");
        if text
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .is_empty()
        {
            Err(WorkspaceDirError::Empty)
        } else if read_as_slashes.starts_with('/') || text.starts_with('~') {
            Err(WorkspaceDirError::NotRelative)
        } else {
            let segments = normalised_segments(&read_as_slashes);
            Ok(Self(if segments.is_empty() {
                ".".to_owned()
            } else {
                segments.join("/")
            }))
        }
    }
}

impl fmt::Display for WorkspaceDir {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The hub and the additional directories.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    hub: Option<WorkspaceDir>,
    add_dirs: Vec<WorkspaceDir>,
}

impl Workspace {
    /// Reads the `workspace` mapping at `node`.
    ///
    /// # Errors
    ///
    /// Every problem of the mapping: an unknown field, a path that is empty, absolute or starts
    /// with `~`, a field of the wrong type.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path)?;
        zip3(
            unknown_fields(&section, &["hub", "add_dirs"]),
            refine::<WorkspaceDir>(&section, "hub"),
            refine_list::<WorkspaceDir>(&section, "add_dirs"),
        )
        .map(|((), hub, listed)| {
            let mut seen = BTreeSet::new();
            let add_dirs = hub
                .iter()
                .cloned()
                .chain(listed.unwrap_or_default())
                .filter(|dir| seen.insert(dir.clone()))
                .collect();
            Self { hub, add_dirs }
        })
    }

    /// The client hub, if the manifest names one.
    #[must_use]
    pub const fn hub(&self) -> Option<&WorkspaceDir> {
        self.hub.as_ref()
    }

    /// The additional directories: the hub first, then `add_dirs` in the order written, with no
    /// duplicate.
    #[must_use]
    pub fn add_dirs(&self) -> &[WorkspaceDir] {
        &self.add_dirs
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceDir, WorkspaceDirError};

    fn dir(text: &str) -> String {
        WorkspaceDir::try_from(text)
            .expect("a workspace directory")
            .to_string()
    }

    #[test]
    fn a_directory_is_normalised_as_the_spec_says() {
        let cases = [
            ("../hub", "../hub"),
            ("../hub/", "../hub"),
            ("..\\hub\\", "../hub"),
            ("./a//b/", "a/b"),
            ("a/../b", "b"),
            ("a/..", "."),
            (".", "."),
            ("./", "."),
            ("../../x", "../../x"),
            ("a/../../x", "../x"),
            (" x", " x"),
        ];
        for (text, expected) in cases {
            assert_eq!(dir(text), expected, "{text:?}");
        }
    }

    #[test]
    fn a_directory_that_is_empty_or_not_relative_is_refused() {
        for text in ["", "  ", "\t", "\u{feff}"] {
            assert_eq!(WorkspaceDir::try_from(text), Err(WorkspaceDirError::Empty));
        }
        for text in ["/abs", "~", "~/hub", "\\abs", "\\\\server\\share"] {
            assert_eq!(
                WorkspaceDir::try_from(text),
                Err(WorkspaceDirError::NotRelative),
                "{text:?}"
            );
        }
    }
}
