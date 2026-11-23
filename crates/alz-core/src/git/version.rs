//! The oldest git alz runs, and what alz says about a git it refuses (spec §6 "Git subprocesses",
//! owner answer 4 of TASK-55.4).

use thiserror::Error;

use crate::{Code, Diagnostic, Message, Stripped};

/// The words `git version` prints before the version.
const PREFIX: &str = "git version ";

/// A git version as far as alz compares it: major and minor. Patch levels and vendor suffixes
/// (`2.50.1 (Apple Git-155)`, `2.40.0.windows.1`) do not matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitVersion {
    major: u32,
    minor: u32,
}

/// Why alz refuses the git it found. Each case is the message the spec gives for the error `git`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VersionRefusal {
    /// A git older than [`GitVersion::MINIMUM`]. It holds the version git printed, stripped.
    #[error("git is older than the minimum version")]
    TooOld {
        /// The text after `git version `, as git printed it.
        found: Stripped,
    },
    /// `git version` printed something that does not read as `git version <major>.<minor>…`.
    #[error("git printed an unreadable version")]
    Unreadable,
    /// No git could be started.
    #[error("git could not be started")]
    NotStarted,
}

impl GitVersion {
    /// The first version whose `rev-parse` accepts `--end-of-options`, and the oldest alz runs.
    pub const MINIMUM: Self = Self {
        major: 2,
        minor: 30,
    };

    /// The version in the output of `git version`, which is accepted when it is at least
    /// [`GitVersion::MINIMUM`].
    ///
    /// # Errors
    ///
    /// [`VersionRefusal::Unreadable`] when the first line is not `git version <major>.<minor>…`,
    /// and [`VersionRefusal::TooOld`] when the version is below the minimum.
    pub fn accept(output: &[u8]) -> Result<Self, VersionRefusal> {
        let text = String::from_utf8_lossy(output);
        let printed = text
            .lines()
            .next()
            .and_then(|line| line.strip_prefix(PREFIX))
            .ok_or(VersionRefusal::Unreadable)?;
        let version = Self::from_printed(printed).ok_or(VersionRefusal::Unreadable)?;
        if version >= Self::MINIMUM {
            Ok(version)
        } else {
            Err(VersionRefusal::TooOld {
                found: Stripped::from_untrusted(printed.trim()),
            })
        }
    }

    /// `<major>.<minor>` at the start of `printed`, each of one or more ASCII digits.
    fn from_printed(printed: &str) -> Option<Self> {
        let (major, rest) = printed.split_once('.')?;
        let minor_end = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());
        let minor = rest.get(..minor_end)?;
        let number = |digits: &str| {
            digits
                .bytes()
                .all(|byte| byte.is_ascii_digit())
                .then(|| digits.parse::<u32>().ok())
                .flatten()
        };
        Some(Self {
            major: number(major)?,
            minor: number(minor)?,
        })
    }
}

impl VersionRefusal {
    /// The error `git`, with no location (spec §6 "Git subprocesses").
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let message = Message::new("alz needs git 2.30 or later; found ");
        Diagnostic::error(
            Code::Git,
            None,
            match self {
                Self::TooOld { found } => message.then(found.clone()),
                Self::Unreadable => message.then("an unreadable version"),
                Self::NotStarted => message.then("none"),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{GitVersion, VersionRefusal};

    fn message(refusal: &VersionRefusal) -> String {
        refusal.diagnostic().to_string()
    }

    #[test]
    fn the_minimum_and_newer_versions_are_accepted() {
        for output in [
            "git version 2.30.0\n",
            "git version 2.50.1 (Apple Git-155)\n",
            "git version 3.0.0\n",
            "git version 2.40.0.windows.1\n",
            "git version 2.100\n",
        ] {
            assert!(GitVersion::accept(output.as_bytes()).is_ok(), "{output}");
        }
    }

    #[test]
    fn an_older_git_is_refused_with_the_version_it_printed() {
        let refusal = GitVersion::accept(b"git version 2.29.9 (Test Git-1)\n").unwrap_err();
        assert_eq!(
            message(&refusal),
            "error [git] alz needs git 2.30 or later; found 2.29.9 (Test Git-1)"
        );
    }

    #[test]
    fn the_version_found_is_stripped_of_control_characters() {
        let refusal = GitVersion::accept(b"git version 1.9\x1b[31m\n").unwrap_err();
        assert_eq!(
            message(&refusal),
            "error [git] alz needs git 2.30 or later; found 1.9[31m"
        );
    }

    #[test]
    fn output_that_is_not_a_version_is_unreadable() {
        for output in [
            &b""[..],
            b"\n",
            b"hello\n",
            b"git version\n",
            b"git version x.y\n",
            b"git version 2\n",
            b"git version 2.\n",
            b"git version .30\n",
            b"git version -2.30\n",
            b"\xff\xfe",
        ] {
            assert_eq!(
                GitVersion::accept(output),
                Err(VersionRefusal::Unreadable),
                "{output:?}"
            );
        }
        assert_eq!(
            message(&VersionRefusal::Unreadable),
            "error [git] alz needs git 2.30 or later; found an unreadable version"
        );
    }

    #[test]
    fn a_git_that_cannot_be_started_is_found_none() {
        assert_eq!(
            message(&VersionRefusal::NotStarted),
            "error [git] alz needs git 2.30 or later; found none"
        );
    }
}
