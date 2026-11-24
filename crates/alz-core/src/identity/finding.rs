//! Where one wrong part of the effective identity comes from, found in git's own order (spec
//! §9.4): the environment variable of the part, the config key of the role, `user.*`, the `EMAIL`
//! variable (for an email only), and otherwise the host. A finding has no field that could hold a
//! value: the wrong value is compared and dropped before a finding exists.

use std::collections::BTreeSet;

use crate::identity::origin::{ConfigOrigin, GitScope};
use crate::identity::slot::{ConfigKey, Field, Slot};

/// What git answered when asked for one config key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAnswer {
    /// The key has a value, set at this scope and origin.
    Set(ConfigOrigin),
    /// The key is not set anywhere (git exits 1).
    NotSet,
    /// Git failed another way, or could not run.
    Failed,
}

impl ConfigAnswer {
    /// The answer to `git config --show-scope --show-origin --get <key>`: `code` is the exit
    /// status, `None` when git did not run or was ended by a signal. Only status 1 means not set.
    #[must_use]
    pub fn from_git(code: Option<i32>, stdout: &[u8]) -> Self {
        match code {
            Some(0) => Self::Set(ConfigOrigin::parse(stdout)),
            Some(1) => Self::NotSet,
            Some(_) | None => Self::Failed,
        }
    }
}

/// Which environment variables are set, by presence only: their values never enter this crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Presence {
    identity: BTreeSet<Slot>,
    email: bool,
}

impl Presence {
    /// The `GIT_<ROLE>_<FIELD>` variables of `slots` are set; `EMAIL` is set when `email`.
    #[must_use]
    pub fn new(slots: impl IntoIterator<Item = Slot>, email: bool) -> Self {
        Self {
            identity: slots.into_iter().collect(),
            email,
        }
    }
}

/// The place an identity part comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The environment variable of the part, such as `GIT_AUTHOR_EMAIL`.
    Environment(Slot),
    /// The `EMAIL` variable.
    EmailVariable,
    /// A config key, read at this scope and origin.
    Config {
        /// The key.
        key: ConfigKey,
        /// Where git read it; its scope is never [`GitScope::Unknown`], which is
        /// [`Source::Unknown`].
        origin: ConfigOrigin,
    },
    /// Nothing sets the part: git guessed it from the host.
    Guessed(Field),
    /// Git could not say where the part comes from.
    Unknown(ConfigKey),
}

/// What a finding is known by, so two findings of one source print once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceKey {
    /// An environment variable of a part.
    Variable(Slot),
    /// The `EMAIL` variable.
    EmailVariable,
    /// A config key.
    Config(ConfigKey),
}

impl SourceKey {
    /// The key or variable as the report prints it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Variable(slot) => slot.variable(),
            Self::EmailVariable => "EMAIL",
            Self::Config(key) => key.as_str(),
        }
    }
}

impl Source {
    /// A config key read at `origin`; an origin without a known scope is not located.
    #[must_use]
    pub fn config(key: ConfigKey, origin: ConfigOrigin) -> Self {
        match origin.scope() {
            GitScope::Unknown => Self::Unknown(key),
            GitScope::System
            | GitScope::Global
            | GitScope::Local
            | GitScope::Worktree
            | GitScope::Command => Self::Config { key, origin },
        }
    }

    /// The key of the finding.
    #[must_use]
    pub const fn key(&self) -> SourceKey {
        match self {
            Self::Environment(slot) => SourceKey::Variable(*slot),
            Self::EmailVariable => SourceKey::EmailVariable,
            Self::Config { key, .. } | Self::Unknown(key) => SourceKey::Config(*key),
            Self::Guessed(field) => SourceKey::Config(field.user_key()),
        }
    }
}

/// One part of the effective identity that differs from the declared one, and where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    slot: Slot,
    source: Source,
}

impl Finding {
    /// The part and its source.
    #[must_use]
    pub const fn new(slot: Slot, source: Source) -> Self {
        Self { slot, source }
    }

    /// The part that differs.
    #[must_use]
    pub const fn slot(&self) -> Slot {
        self.slot
    }

    /// Where it comes from.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    /// Whether the part comes from the local `user.<field>` key, which `.git/config` holds.
    #[must_use]
    pub fn is_local_user_key(&self, field: Field) -> bool {
        matches!(
            &self.source,
            Source::Config { key, origin }
                if *key == field.user_key() && origin.scope() == GitScope::Local
        )
    }
}

/// Finds where `slot` comes from. `ask` is called for a config key only when no environment
/// variable of the part is set and no earlier key had an answer, so the effects behind it (one
/// `git config` each) run only as far as git's own lookup does.
pub fn locate(
    slot: Slot,
    presence: &Presence,
    mut ask: impl FnMut(ConfigKey) -> ConfigAnswer,
) -> Finding {
    let source = if presence.identity.contains(&slot) {
        Source::Environment(slot)
    } else {
        [slot.role_key(), slot.field().user_key()]
            .into_iter()
            .find_map(|key| match ask(key) {
                ConfigAnswer::NotSet => None,
                ConfigAnswer::Set(origin) => Some(Source::config(key, origin)),
                ConfigAnswer::Failed => Some(Source::Unknown(key)),
            })
            .unwrap_or_else(|| match slot.field() {
                Field::Email if presence.email => Source::EmailVariable,
                Field::Email | Field::Name => Source::Guessed(slot.field()),
            })
    };
    Finding::new(slot, source)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{ConfigAnswer, Presence, Source, SourceKey, locate};
    use crate::identity::origin::ConfigOrigin;
    use crate::identity::slot::{ConfigKey, Field, Role, Slot};

    const AUTHOR_EMAIL: Slot = Slot::new(Role::Author, Field::Email);

    fn set(scope: &str) -> ConfigAnswer {
        ConfigAnswer::Set(ConfigOrigin::parse(
            format!("{scope}\tfile:/h/.gitconfig\tv").as_bytes(),
        ))
    }

    /// The source found, and the keys asked about, in order.
    fn found(
        slot: Slot,
        presence: &Presence,
        answers: &[(ConfigKey, ConfigAnswer)],
    ) -> (Source, Vec<ConfigKey>) {
        let asked = RefCell::new(Vec::new());
        let finding = locate(slot, presence, |key| {
            asked.borrow_mut().push(key);
            answers
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map_or(ConfigAnswer::NotSet, |(_, answer)| answer.clone())
        });
        (finding.source().clone(), asked.into_inner())
    }

    #[test]
    fn the_environment_comes_first_and_git_is_not_asked() {
        let presence = Presence::new([AUTHOR_EMAIL], true);
        let (source, asked) = found(AUTHOR_EMAIL, &presence, &[]);
        assert_eq!(source, Source::Environment(AUTHOR_EMAIL));
        assert_eq!(asked.len(), 0);
    }

    #[test]
    fn the_role_key_comes_before_user_and_a_found_key_ends_the_search() {
        let role = AUTHOR_EMAIL.role_key();
        let (source, asked) = found(
            AUTHOR_EMAIL,
            &Presence::default(),
            &[
                (role, set("local")),
                (Field::Email.user_key(), set("global")),
            ],
        );
        assert_eq!(asked, [role]);
        assert_eq!(source.key(), SourceKey::Config(role));
    }

    #[test]
    fn user_comes_next_then_the_email_variable_then_the_host() {
        let user = Field::Email.user_key();
        let (source, asked) = found(AUTHOR_EMAIL, &Presence::default(), &[(user, set("global"))]);
        assert_eq!(asked, [AUTHOR_EMAIL.role_key(), user]);
        assert_eq!(source.key(), SourceKey::Config(user));

        let (source, _) = found(AUTHOR_EMAIL, &Presence::new([], true), &[]);
        assert_eq!(source, Source::EmailVariable);
        let (source, _) = found(AUTHOR_EMAIL, &Presence::default(), &[]);
        assert_eq!(source, Source::Guessed(Field::Email));
    }

    #[test]
    fn the_email_variable_is_for_emails_only() {
        let name = Slot::new(Role::Author, Field::Name);
        let (source, _) = found(name, &Presence::new([], true), &[]);
        assert_eq!(source, Source::Guessed(Field::Name));
    }

    #[test]
    fn a_failed_lookup_is_an_unknown_source_and_ends_the_search() {
        let (source, asked) = found(
            AUTHOR_EMAIL,
            &Presence::default(),
            &[(AUTHOR_EMAIL.role_key(), ConfigAnswer::Failed)],
        );
        assert_eq!(source, Source::Unknown(AUTHOR_EMAIL.role_key()));
        assert_eq!(asked.len(), 1);
    }

    #[test]
    fn a_scope_git_did_not_name_is_an_unknown_source() {
        let (source, _) = found(
            AUTHOR_EMAIL,
            &Presence::default(),
            &[(AUTHOR_EMAIL.role_key(), set("someday"))],
        );
        assert_eq!(source, Source::Unknown(AUTHOR_EMAIL.role_key()));
    }

    #[test]
    fn git_exit_codes_mean_set_not_set_or_failed() {
        assert!(matches!(
            ConfigAnswer::from_git(Some(0), b"local\tfile:x\tv"),
            ConfigAnswer::Set(_)
        ));
        assert_eq!(ConfigAnswer::from_git(Some(1), b""), ConfigAnswer::NotSet);
        assert_eq!(ConfigAnswer::from_git(Some(128), b""), ConfigAnswer::Failed);
        assert_eq!(ConfigAnswer::from_git(None, b""), ConfigAnswer::Failed);
    }
}
