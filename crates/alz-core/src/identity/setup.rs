//! Setting the declared identity in the repository's own `.git/config` (spec §9.4): which keys
//! may be written, and why a write is refused. The plan adds a key only when `.git/config` holds
//! no value for it. An equal value is left alone and a different one is never replaced.

use crate::identity::slot::Field;

/// Why alz may not write `.git/config` of the repository root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    /// The root is not the top level of its work tree: a write would reach the outer repository.
    NotTopLevel,
    /// `.git` is not a real directory: a linked worktree, a submodule's gitfile or a link.
    GitNotADirectory,
    /// `.git/config` exists and is not a regular file, or is a link.
    ConfigNotRegular,
    /// `.git/config` resolves outside the repository.
    ConfigOutside,
    /// The repository could not be inspected.
    Uninspectable,
}

impl Blocker {
    /// Whether a `git config` command run by hand would write to another repository, so the
    /// report prints none.
    #[must_use]
    pub const fn would_reach_another_repository(self) -> bool {
        matches!(self, Self::NotTopLevel)
    }

    /// The reason, as the report words it.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotTopLevel => "the repository root is not the top level of its work tree",
            Self::GitNotADirectory => {
                ".git is not a plain directory (a linked worktree, submodule or symlink)"
            }
            Self::ConfigNotRegular => ".git/config is not a regular file",
            Self::ConfigOutside => ".git/config resolves outside the repository",
            Self::Uninspectable => "the repository could not be inspected",
        }
    }
}

/// What the probes found about the file a write would change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitConfigPlace {
    /// A top-level repository with a real `.git` directory and a regular or absent `.git/config`.
    Writable,
    /// Nothing may be written.
    Blocked(Blocker),
}

/// What `git config --file <repo>/.git/config --get <key>` said, with the value dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// The file has no value for the key (exit status 1).
    Missing,
    /// The value equals the declared one.
    Equal,
    /// The file holds another value.
    Different,
    /// Git failed another way: the file could not be read.
    Unreadable,
}

impl KeyState {
    /// Reads the outcome of `--get`: `code` is the exit status, `None` when git did not run.
    /// The output, trimmed, is compared with `declared` and not kept.
    #[must_use]
    pub fn from_get(code: Option<i32>, stdout: &[u8], declared: &str) -> Self {
        match code {
            Some(0) => {
                let equal = std::str::from_utf8(stdout).is_ok_and(|text| text.trim() == declared);
                if equal { Self::Equal } else { Self::Different }
            }
            Some(1) => Self::Missing,
            Some(_) | None => Self::Unreadable,
        }
    }
}

/// What to do with one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Write the declared value: the file has none.
    Add,
    /// Nothing: the file already holds the declared value.
    LeaveEqual,
    /// Nothing: the file holds another value, which is reported and never replaced.
    ReportDifferent,
    /// Nothing: the file could not be read, so the key is reported to be set by hand.
    ReportUnreadable,
}

impl From<KeyState> for KeyAction {
    fn from(state: KeyState) -> Self {
        match state {
            KeyState::Missing => Self::Add,
            KeyState::Equal => Self::LeaveEqual,
            KeyState::Different => Self::ReportDifferent,
            KeyState::Unreadable => Self::ReportUnreadable,
        }
    }
}

/// The action for `user.name` and for `user.email`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupPlan {
    name: KeyAction,
    email: KeyAction,
}

impl SetupPlan {
    /// The plan for what `.git/config` holds.
    #[must_use]
    pub fn new(name: KeyState, email: KeyState) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }

    /// The action for each field, name first.
    #[must_use]
    pub const fn actions(self) -> [(Field, KeyAction); 2] {
        [(Field::Name, self.name), (Field::Email, self.email)]
    }

    /// The fields to write: exactly those whose key `.git/config` did not hold.
    pub fn additions(self) -> impl Iterator<Item = Field> {
        self.actions()
            .into_iter()
            .filter_map(|(field, action)| matches!(action, KeyAction::Add).then_some(field))
    }

    /// The fields whose local key holds another value and stays so.
    pub fn kept(self) -> impl Iterator<Item = Field> {
        self.actions().into_iter().filter_map(|(field, action)| {
            matches!(action, KeyAction::ReportDifferent).then_some(field)
        })
    }
}

/// Whether to write, and what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupDecision {
    /// Nothing may be written; the report says why.
    Blocked(Blocker),
    /// Write the additions of the plan.
    Write(SetupPlan),
}

impl SetupDecision {
    /// Decides from the place. `states` reads the two keys and runs only for a writable place,
    /// so a blocked place costs no read.
    pub fn decide(place: GitConfigPlace, states: impl FnOnce() -> (KeyState, KeyState)) -> Self {
        match place {
            GitConfigPlace::Blocked(blocker) => Self::Blocked(blocker),
            GitConfigPlace::Writable => {
                let (name, email) = states();
                Self::Write(SetupPlan::new(name, email))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Blocker, Field, GitConfigPlace, KeyAction, KeyState, SetupDecision, SetupPlan};

    #[test]
    fn get_outcomes_become_states_and_the_value_is_dropped() {
        assert_eq!(
            KeyState::from_get(Some(0), b"Owner\n", "Owner"),
            KeyState::Equal
        );
        assert_eq!(
            KeyState::from_get(Some(0), b"Other\n", "Owner"),
            KeyState::Different
        );
        assert_eq!(
            KeyState::from_get(Some(0), &[0xff], "Owner"),
            KeyState::Different
        );
        assert_eq!(KeyState::from_get(Some(1), b"", "Owner"), KeyState::Missing);
        assert_eq!(
            KeyState::from_get(Some(128), b"", "Owner"),
            KeyState::Unreadable
        );
        assert_eq!(KeyState::from_get(None, b"", "Owner"), KeyState::Unreadable);
    }

    #[test]
    fn only_a_missing_key_is_added_and_a_different_one_is_kept() {
        let plan = SetupPlan::new(KeyState::Missing, KeyState::Different);
        assert_eq!(plan.additions().collect::<Vec<_>>(), [Field::Name]);
        assert_eq!(plan.kept().collect::<Vec<_>>(), [Field::Email]);
        let plan = SetupPlan::new(KeyState::Equal, KeyState::Unreadable);
        assert_eq!(plan.additions().count(), 0);
        assert_eq!(plan.actions()[1].1, KeyAction::ReportUnreadable);
    }

    #[test]
    fn a_blocked_place_reads_nothing() {
        let decision =
            SetupDecision::decide(GitConfigPlace::Blocked(Blocker::GitNotADirectory), || {
                unreachable!("no read for a blocked place")
            });
        assert_eq!(decision, SetupDecision::Blocked(Blocker::GitNotADirectory));
        assert!(Blocker::NotTopLevel.would_reach_another_repository());
        assert!(!Blocker::ConfigOutside.would_reach_another_repository());
    }
}
