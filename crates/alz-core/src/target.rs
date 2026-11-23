//! The agent tools alz writes configuration for (spec §8). The adapter trait that goes with each
//! target is R5's; this module holds the closed set of names that layer files may use.

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Printable, sealed::Sealed};

/// A target: `claude` or `codex` (spec §8). Each variant is named in `layer.yaml` and in the
/// stack manifest by its lower-case word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetName {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
}

impl TargetName {
    /// Every target, in name order.
    pub const ALL: [Self; 2] = [Self::Claude, Self::Codex];

    /// The target's name as the spec writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

/// Text that is not the name of a target. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a target: claude or codex")]
pub struct TargetError;

impl TargetError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be a target: claude or codex"
    }
}

impl TryFrom<&str> for TargetName {
    type Error = TargetError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|target| target.as_str() == text)
            .ok_or(TargetError)
    }
}

impl fmt::Display for TargetName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Sealed for TargetName {}

impl Printable for TargetName {
    fn printable(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::{TargetError, TargetName};

    #[test]
    fn the_targets_are_listed_in_name_order() {
        let names: Vec<&str> = TargetName::ALL
            .iter()
            .map(|target| target.as_str())
            .collect();
        assert_eq!(names, ["claude", "codex"]);
        assert!(TargetName::Claude < TargetName::Codex);
    }

    #[test]
    fn a_target_is_read_from_its_exact_lower_case_name() {
        assert_eq!(TargetName::try_from("codex"), Ok(TargetName::Codex));
        assert_eq!(TargetName::try_from("Claude"), Err(TargetError));
        assert_eq!(TargetName::try_from(""), Err(TargetError));
    }
}
