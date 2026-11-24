//! The identity check of spec §9.4: is the identity git would use for a commit the declared one,
//! and if not, where does each wrong part come from. It is a decision over values the edge
//! gathered: the repository's state, the verdicts of `git var` and the answers of `git config`.

use crate::diagnostic::Level;
use crate::identity::effective::Effective;
use crate::identity::finding::Finding;
use crate::identity::slot::{Role, Slot};

/// What `git rev-parse --git-dir` said about the repository root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoState {
    /// A git repository.
    Repository,
    /// Git itself says this is not a repository.
    NotARepository,
    /// Git is missing, or failed another way (a broken config file): the identity cannot be
    /// checked.
    Unavailable,
}

impl RepoState {
    /// Classifies the run of `git rev-parse --git-dir` made with `LC_ALL=C`: `code` is the exit
    /// status, `None` when git did not run. Only exit status 128 with git's own "not a git
    /// repository" is [`RepoState::NotARepository`].
    #[must_use]
    pub fn classify(code: Option<i32>, stderr: &[u8]) -> Self {
        match code {
            Some(0) => Self::Repository,
            Some(128) if String::from_utf8_lossy(stderr).contains("not a git repository") => {
                Self::NotARepository
            }
            Some(_) | None => Self::Unavailable,
        }
    }
}

/// The verdicts for both roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdicts {
    /// What `git var GIT_AUTHOR_IDENT` says.
    pub author: Effective,
    /// What `git var GIT_COMMITTER_IDENT` says.
    pub committer: Effective,
}

impl Verdicts {
    const fn of(self, role: Role) -> Effective {
        match role {
            Role::Author => self.author,
            Role::Committer => self.committer,
        }
    }

    /// The parts that differ, author first, name before email within a role.
    fn differing(self) -> impl Iterator<Item = Slot> {
        Role::ALL.into_iter().flat_map(move |role| {
            crate::identity::slot::Field::ALL
                .into_iter()
                .map(move |field| Slot::new(role, field))
                .filter(move |slot| {
                    !matches!(
                        self.of(role).of(slot.field()),
                        crate::identity::effective::Agreement::Same
                    )
                })
        })
    }
}

/// The parts that differ from the declared identity, at least one, in the order they were found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings(Vec<Finding>);

impl Findings {
    /// The findings, `None` for none: a mismatch has at least one.
    #[must_use]
    pub fn new(findings: Vec<Finding>) -> Option<Self> {
        (!findings.is_empty()).then_some(Self(findings))
    }

    /// The findings, as found.
    #[must_use]
    pub fn as_slice(&self) -> &[Finding] {
        &self.0
    }

    /// The findings that remain after `keep` drops some; `None` when none remain.
    #[must_use]
    pub fn retain(self, keep: impl Fn(&Finding) -> bool) -> Option<Self> {
        let kept: Vec<Finding> = self.0.into_iter().filter(|finding| keep(finding)).collect();
        (!kept.is_empty()).then_some(Self(kept))
    }
}

/// The outcome of the check of one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityCheck {
    /// Author and committer equal the declared identity.
    Matches,
    /// At least one part differs.
    Differs(Findings),
    /// Not a git repository: nothing was read, a note is printed.
    NotARepository,
    /// Git cannot be used here, so the identity cannot be checked.
    Unavailable,
}

/// The command that reports the check, which sets how serious a problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `alz init`: a warning.
    Init,
    /// `alz build`, with or without `--adopt`: a warning.
    Build,
    /// `alz check`: an error, unless `--skip-identity` skips the whole step.
    Check,
}

impl Command {
    /// How serious a mismatch, or an identity that cannot be checked, is for this command.
    #[must_use]
    pub const fn level(self) -> Level {
        match self {
            Self::Init | Self::Build => Level::Warning,
            Self::Check => Level::Error,
        }
    }
}

impl IdentityCheck {
    /// Decides the check. `verdicts` runs only in a repository, since `git var` needs one, and
    /// `locate` runs once per differing part, author first.
    pub fn decide(
        state: RepoState,
        verdicts: impl FnOnce() -> Verdicts,
        locate: impl FnMut(Slot) -> Finding,
    ) -> Self {
        match state {
            RepoState::NotARepository => Self::NotARepository,
            RepoState::Unavailable => Self::Unavailable,
            RepoState::Repository => Findings::new(verdicts().differing().map(locate).collect())
                .map_or(Self::Matches, Self::Differs),
        }
    }

    /// How serious the outcome is for `command`; `None` when nothing is wrong. A directory that is
    /// not a repository is only a note, whatever the command.
    #[must_use]
    pub const fn level(&self, command: Command) -> Option<Level> {
        match self {
            Self::Matches => None,
            Self::NotARepository => Some(Level::Note),
            Self::Differs(_) | Self::Unavailable => Some(command.level()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, IdentityCheck, RepoState, Verdicts};
    use crate::diagnostic::Level;
    use crate::identity::effective::Effective;
    use crate::identity::finding::{Finding, Source};
    use crate::identity::slot::{Field, Role, Slot};
    use crate::identity::{CommitIdentity, EmailAddress, IdentityName};

    fn declared() -> CommitIdentity {
        CommitIdentity::new(
            IdentityName::try_from("Owner").unwrap(),
            EmailAddress::try_from("o@example.com").unwrap(),
        )
    }

    fn same() -> Effective {
        Effective::compare(Some(b"Owner <o@example.com> 1 +0000"), &declared())
    }

    fn guessed(slot: Slot) -> Finding {
        Finding::new(slot, Source::Guessed(slot.field()))
    }

    #[test]
    fn rev_parse_decides_the_state() {
        let line = b"fatal: not a git repository (or any of the parent directories): .git";
        assert_eq!(RepoState::classify(Some(0), b""), RepoState::Repository);
        assert_eq!(
            RepoState::classify(Some(128), line),
            RepoState::NotARepository
        );
        assert_eq!(
            RepoState::classify(Some(128), b"fatal: bad config"),
            RepoState::Unavailable
        );
        assert_eq!(RepoState::classify(Some(1), line), RepoState::Unavailable);
        assert_eq!(RepoState::classify(None, b""), RepoState::Unavailable);
    }

    #[test]
    fn a_matching_identity_has_no_findings_and_no_level() {
        let check = IdentityCheck::decide(
            RepoState::Repository,
            || Verdicts {
                author: same(),
                committer: same(),
            },
            guessed,
        );
        assert_eq!(check, IdentityCheck::Matches);
        assert_eq!(check.level(Command::Check), None);
    }

    #[test]
    fn findings_come_author_first_and_only_for_differing_parts() {
        let wrong = Effective::compare(Some(b"Other <o@example.com> 1 +0000"), &declared());
        let check = IdentityCheck::decide(
            RepoState::Repository,
            || Verdicts {
                author: wrong,
                committer: wrong,
            },
            guessed,
        );
        let IdentityCheck::Differs(findings) = check else {
            panic!("expected findings");
        };
        let slots: Vec<Slot> = findings.as_slice().iter().map(Finding::slot).collect();
        assert_eq!(
            slots,
            [
                Slot::new(Role::Author, Field::Name),
                Slot::new(Role::Committer, Field::Name)
            ]
        );
    }

    #[test]
    fn git_is_not_asked_for_an_identity_outside_a_repository() {
        for state in [RepoState::NotARepository, RepoState::Unavailable] {
            let _ = IdentityCheck::decide(state, || unreachable!("no git var"), |_| unreachable!());
        }
    }

    #[test]
    fn the_level_follows_the_command_and_a_missing_repository_is_a_note() {
        assert_eq!(
            IdentityCheck::Unavailable.level(Command::Check),
            Some(Level::Error)
        );
        assert_eq!(
            IdentityCheck::Unavailable.level(Command::Build),
            Some(Level::Warning)
        );
        assert_eq!(
            IdentityCheck::Unavailable.level(Command::Init),
            Some(Level::Warning)
        );
        assert_eq!(
            IdentityCheck::NotARepository.level(Command::Check),
            Some(Level::Note)
        );
    }
}
