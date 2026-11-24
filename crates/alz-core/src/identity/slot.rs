//! The four parts of the identity git uses for a commit: the author's and the committer's name
//! and email (spec §9.4). Every name git or a report prints for one of them is a `&'static str`
//! chosen by an exhaustive match, so no text from outside can reach a report through it.

use std::cmp::Ordering;

/// Who of a commit: the author or the committer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// The person who wrote the change.
    Author,
    /// The person who made the commit.
    Committer,
}

impl Role {
    /// Both roles, in the order reports print them.
    pub const ALL: [Self; 2] = [Self::Author, Self::Committer];
}

/// One part of an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    /// The name.
    Name,
    /// The email.
    Email,
}

impl Field {
    /// Both fields, in the order reports print them.
    pub const ALL: [Self; 2] = [Self::Name, Self::Email];

    /// The word a report uses: `name` or `email`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
        }
    }

    /// The key `user.name` or `user.email`.
    #[must_use]
    pub const fn user_key(self) -> ConfigKey {
        ConfigKey::User(self)
    }
}

/// One of the four parts: a role and a field. Slots order by field first, then by role, which is
/// the order reports print findings in: name before email, author before committer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot {
    role: Role,
    field: Field,
}

impl Slot {
    /// The part of `role` named `field`.
    #[must_use]
    pub const fn new(role: Role, field: Field) -> Self {
        Self { role, field }
    }

    /// All four slots, in the order reports print them.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::new(Role::Author, Field::Name),
            Self::new(Role::Committer, Field::Name),
            Self::new(Role::Author, Field::Email),
            Self::new(Role::Committer, Field::Email),
        ]
    }

    /// The role.
    #[must_use]
    pub const fn role(self) -> Role {
        self.role
    }

    /// The field.
    #[must_use]
    pub const fn field(self) -> Field {
        self.field
    }

    /// The environment variable git reads first for this part, such as `GIT_AUTHOR_EMAIL`.
    #[must_use]
    pub const fn variable(self) -> &'static str {
        match (self.role, self.field) {
            (Role::Author, Field::Name) => "GIT_AUTHOR_NAME",
            (Role::Author, Field::Email) => "GIT_AUTHOR_EMAIL",
            (Role::Committer, Field::Name) => "GIT_COMMITTER_NAME",
            (Role::Committer, Field::Email) => "GIT_COMMITTER_EMAIL",
        }
    }

    /// The config key of the role, such as `author.email`.
    #[must_use]
    pub const fn role_key(self) -> ConfigKey {
        ConfigKey::Role(self)
    }
}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Slot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.field
            .cmp(&other.field)
            .then(self.role.cmp(&other.role))
    }
}

/// A config key git reads for an identity: `author.*` and `committer.*` of a slot, or `user.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConfigKey {
    /// `author.name`, `author.email`, `committer.name` or `committer.email`.
    Role(Slot),
    /// `user.name` or `user.email`.
    User(Field),
}

impl ConfigKey {
    /// The key as git spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Role(slot) => match (slot.role, slot.field) {
                (Role::Author, Field::Name) => "author.name",
                (Role::Author, Field::Email) => "author.email",
                (Role::Committer, Field::Name) => "committer.name",
                (Role::Committer, Field::Email) => "committer.email",
            },
            Self::User(Field::Name) => "user.name",
            Self::User(Field::Email) => "user.email",
        }
    }

    /// The part of an identity the key sets.
    #[must_use]
    pub const fn field(self) -> Field {
        match self {
            Self::Role(slot) => slot.field,
            Self::User(field) => field,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigKey, Field, Role, Slot};

    #[test]
    fn slots_print_name_before_email_and_author_before_committer() {
        let mut slots = [
            Slot::new(Role::Committer, Field::Email),
            Slot::new(Role::Author, Field::Email),
            Slot::new(Role::Committer, Field::Name),
            Slot::new(Role::Author, Field::Name),
        ];
        slots.sort();
        assert_eq!(slots, Slot::all());
    }

    #[test]
    fn names_are_the_ones_git_reads() {
        let variables = Slot::all().map(Slot::variable);
        assert_eq!(
            variables,
            [
                "GIT_AUTHOR_NAME",
                "GIT_COMMITTER_NAME",
                "GIT_AUTHOR_EMAIL",
                "GIT_COMMITTER_EMAIL"
            ]
        );
        let keys = Slot::all().map(|slot| slot.role_key().as_str());
        assert_eq!(
            keys,
            [
                "author.name",
                "committer.name",
                "author.email",
                "committer.email"
            ]
        );
        assert_eq!(ConfigKey::User(Field::Email).as_str(), "user.email");
        assert_eq!(Field::Name.user_key().field(), Field::Name);
    }
}
