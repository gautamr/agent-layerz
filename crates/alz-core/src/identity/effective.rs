//! The identity git would use for a commit, as `git var GIT_AUTHOR_IDENT` and
//! `git var GIT_COMMITTER_IDENT` print it (spec §9.4): `Name <email> <epoch> <zone>`.
//!
//! The output is compared with the declared identity at once and only the two verdicts are
//! kept. The reported value is dropped when the line is parsed, so nothing later can print it.

use crate::identity::{CommitIdentity, Field};

/// Whether one reported part equals the declared one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// Exactly equal.
    Same,
    /// Not equal, or not reported at all.
    Different,
}

impl Agreement {
    fn of(equal: bool) -> Self {
        if equal { Self::Same } else { Self::Different }
    }
}

/// What the effective identity of one role says about the declared one: a verdict for the name and
/// one for the email, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effective {
    name: Agreement,
    email: Agreement,
}

impl Effective {
    /// Both parts differ: git reported nothing it could be compared with.
    pub const DIFFERENT: Self = Self {
        name: Agreement::Different,
        email: Agreement::Different,
    };

    /// Compares the standard output of `git var GIT_<ROLE>_IDENT`, `None` when git failed (no
    /// identity with `user.useConfigOnly`, a broken config file), with `declared`. Total: any
    /// bytes give a value, and bytes that are not an identity line, or not UTF-8, differ in both
    /// parts. A trimmed output is parsed as `<name> <<email>> <epoch> <zone>`.
    #[must_use]
    pub fn compare(output: Option<&[u8]>, declared: &CommitIdentity) -> Self {
        output
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(split_ident)
            .map_or(Self::DIFFERENT, |(name, email)| Self {
                name: Agreement::of(name == declared.name().as_str()),
                email: Agreement::of(email == declared.email().as_str()),
            })
    }

    /// The verdict for one field.
    #[must_use]
    pub const fn of(self, field: Field) -> Agreement {
        match field {
            Field::Name => self.name,
            Field::Email => self.email,
        }
    }

    /// Whether both parts equal the declared ones.
    #[must_use]
    pub const fn matches(self) -> bool {
        matches!((self.name, self.email), (Agreement::Same, Agreement::Same))
    }
}

/// `(name, email)` of `Name <email> <epoch> <zone>`. The name is everything before the last
/// ` <`, so it may hold `<` and spaces; the email holds neither `<` nor `>`.
fn split_ident(output: &str) -> Option<(&str, &str)> {
    let (rest, zone) = output.trim().rsplit_once(' ')?;
    let mut zone_chars = zone.chars();
    let zone_ok = zone_chars
        .next()
        .is_some_and(|sign| matches!(sign, '+' | '-'))
        && zone.chars().count() == 5
        && zone_chars.all(|digit| digit.is_ascii_digit());
    let (rest, epoch) = rest.rsplit_once(' ')?;
    let epoch_ok = !epoch.is_empty() && epoch.chars().all(|digit| digit.is_ascii_digit());
    let (head, email) = rest.strip_suffix('>')?.rsplit_once('<')?;
    let name = head.strip_suffix(' ')?;
    (zone_ok && epoch_ok && !email.contains('>')).then_some((name, email))
}

#[cfg(test)]
mod tests {
    use super::{Agreement, Effective};
    use crate::identity::{CommitIdentity, EmailAddress, Field, IdentityName};

    fn declared() -> CommitIdentity {
        CommitIdentity::new(
            IdentityName::try_from("Owner Example").unwrap(),
            EmailAddress::try_from("owner@example.com").unwrap(),
        )
    }

    fn compare(output: &str) -> Effective {
        Effective::compare(Some(output.as_bytes()), &declared())
    }

    #[test]
    fn an_equal_line_matches() {
        let result = compare("Owner Example <owner@example.com> 1700000000 +0200\n");
        assert!(result.matches());
    }

    #[test]
    fn each_part_is_judged_alone() {
        let result = compare("Someone <owner@example.com> 1 -0700");
        assert_eq!(result.of(Field::Name), Agreement::Different);
        assert_eq!(result.of(Field::Email), Agreement::Same);
        let result = compare("Owner Example <other@example.com> 1 -0700");
        assert_eq!(result.of(Field::Name), Agreement::Same);
        assert_eq!(result.of(Field::Email), Agreement::Different);
    }

    #[test]
    fn a_comparison_is_exact() {
        assert!(!compare("owner example <owner@example.com> 1 +0000").matches());
        assert!(!compare("Owner Example <Owner@example.com> 1 +0000").matches());
    }

    #[test]
    fn anything_that_is_not_an_identity_line_differs_in_both_parts() {
        for text in [
            "",
            "Owner Example",
            "Owner Example <owner@example.com>",
            "Owner Example <owner@example.com> x +0000",
            "Owner Example <owner@example.com> 1 +00000",
            "Owner Example <owner@example.com> 1 0000",
            "Owner Example <owner@<example.com> 1 +0000",
            "Owner Example<owner@example.com> 1 +0000",
            "<owner@example.com> 1 +0000",
        ] {
            assert_eq!(compare(text), Effective::DIFFERENT, "{text:?}");
        }
        assert_eq!(Effective::compare(None, &declared()), Effective::DIFFERENT);
        assert_eq!(
            Effective::compare(Some(&[0xff, 0xfe]), &declared()),
            Effective::DIFFERENT
        );
    }
}
