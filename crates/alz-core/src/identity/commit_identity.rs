//! The commit identity a layer declares (spec §7.12 "Validation"). The identity is untrusted
//! layer content that reaches a git subprocess, so a name and an email are values that carry the
//! proof of the rules, and a `CommitIdentity` is made only from two of them.

use thiserror::Error;

use crate::schema::{Checked, SchemaErrors, require, unknown_fields, zip};
use crate::untrusted_text::{is_control, is_format};
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};

const NAME_MAX: usize = 128;
const EMAIL_MAX: usize = 254;
/// Characters git trims from both ends of a name, or that a shell reads specially
/// (`docs/target-facts.md` §11).
const NAME_EDGE: [char; 9] = ['.', ',', ':', ';', '"', '\'', '\\', '<', '>'];

/// A committer name that passed the rules of §7.12.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentityName(String);

/// Why a text is not a committer name. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IdentityNameError {
    /// Empty, or more than 128 characters.
    #[error("a committer name has 1 to 128 characters")]
    Length,
    /// Whitespace at the start or the end.
    #[error("a committer name has no leading or trailing whitespace")]
    Whitespace,
    /// One of `. , : ; " ' \ < >` at the start or the end, which git trims.
    #[error("a committer name does not start or end with a character git trims")]
    Edge,
    /// A `-` at the start, which a command line reads as an option.
    #[error("a committer name does not start with '-'")]
    LeadingDash,
    /// A control character anywhere.
    #[error("a committer name has no control character")]
    Control,
    /// A Unicode format character anywhere.
    #[error("a committer name has no invisible format character")]
    Format,
    /// `<`, `>`, a backtick or a backslash anywhere.
    #[error("a committer name has none of '<', '>', a backtick or a backslash")]
    Forbidden,
}

impl IdentityNameError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Length => "must be 1 to 128 characters",
            Self::Whitespace => "must not start or end with whitespace",
            Self::Edge => "must not start or end with . , : ; \" ' \\ < or > (git trims them)",
            Self::LeadingDash => "must not start with '-'",
            Self::Control => "must not contain control characters",
            Self::Format => "must not contain invisible format characters",
            Self::Forbidden => "must not contain '<', '>', a backtick or a backslash",
        }
    }
}

impl IdentityName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for IdentityName {
    type Error = IdentityNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let length = text.chars().count();
        let at_edge = |test: fn(char) -> bool| {
            text.chars().next().is_some_and(test) || text.chars().next_back().is_some_and(test)
        };
        // The order is the order of the spec's list; the first rule broken is the one reported.
        let broken = [
            (!(1..=NAME_MAX).contains(&length)).then_some(IdentityNameError::Length),
            at_edge(char::is_whitespace).then_some(IdentityNameError::Whitespace),
            at_edge(|character| NAME_EDGE.contains(&character)).then_some(IdentityNameError::Edge),
            text.starts_with('-')
                .then_some(IdentityNameError::LeadingDash),
            text.chars()
                .any(is_control)
                .then_some(IdentityNameError::Control),
            text.chars()
                .any(is_format)
                .then_some(IdentityNameError::Format),
            text.contains(['<', '>', '`', '\\'])
                .then_some(IdentityNameError::Forbidden),
        ];
        broken
            .into_iter()
            .flatten()
            .next()
            .map_or_else(|| Ok(Self(text.to_owned())), Err)
    }
}

/// A committer email that passed the rules of §7.12.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EmailAddress(String);

/// Why a text is not a committer email. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EmailError {
    /// More than 254 characters.
    #[error("a committer email has at most 254 characters")]
    TooLong,
    /// A `-` at the start, which a command line reads as an option.
    #[error("a committer email does not start with '-'")]
    LeadingDash,
    /// A `.` at the start or the end.
    #[error("a committer email does not start or end with '.'")]
    EdgeDot,
    /// A Unicode format character anywhere.
    #[error("a committer email has no invisible format character")]
    Format,
    /// Not `local@domain` in plain ASCII.
    #[error("a committer email is a plain ASCII address")]
    NotPlainAscii,
}

impl EmailError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::TooLong => "must be at most 254 characters",
            Self::LeadingDash => "must not start with '-'",
            Self::EdgeDot => "must not start or end with '.'",
            Self::Format => "must not contain invisible format characters",
            Self::NotPlainAscii => "must be a plain ASCII email address",
        }
    }
}

/// `^[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$`, read without a regular expression.
fn is_plain_address(text: &str) -> bool {
    let local_character = |byte: u8| byte.is_ascii_alphanumeric() || b"._%+-".contains(&byte);
    let label_character = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'-';
    text.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && local.bytes().all(local_character)
            && domain.contains('.')
            && domain
                .split('.')
                .all(|label| !label.is_empty() && label.bytes().all(label_character))
    })
}

impl EmailAddress {
    /// The address as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for EmailAddress {
    type Error = EmailError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let broken = [
            (text.chars().count() > EMAIL_MAX).then_some(EmailError::TooLong),
            text.starts_with('-').then_some(EmailError::LeadingDash),
            (text.starts_with('.') || text.ends_with('.')).then_some(EmailError::EdgeDot),
            text.chars().any(is_format).then_some(EmailError::Format),
            (!is_plain_address(text)).then_some(EmailError::NotPlainAscii),
        ];
        broken
            .into_iter()
            .flatten()
            .next()
            .map_or_else(|| Ok(Self(text.to_owned())), Err)
    }
}

/// The identity every commit is authored and committed under: a name and an email, both valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitIdentity {
    name: IdentityName,
    email: EmailAddress,
}

impl CommitIdentity {
    /// An identity of a valid name and a valid email.
    #[must_use]
    pub const fn new(name: IdentityName, email: EmailAddress) -> Self {
        Self { name, email }
    }

    /// The committer name.
    #[must_use]
    pub const fn name(&self) -> &IdentityName {
        &self.name
    }

    /// The committer email.
    #[must_use]
    pub const fn email(&self) -> &EmailAddress {
        &self.email
    }

    /// Reads the value of the `commit_identity` field at `path`: a mapping with exactly the keys
    /// `name` and `email`, both text.
    ///
    /// # Errors
    ///
    /// Every problem of the value, with its key path: a value that is not a mapping, a key that
    /// is not `name` or `email`, a missing key, a value that is not text or that breaks a rule.
    /// No error holds the value.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path).map_err(SchemaErrors::from)?;
        zip(
            unknown_fields(&section, &["name", "email"]),
            zip(
                require::<IdentityName>(&section, "name"),
                require::<EmailAddress>(&section, "email"),
            ),
        )
        .map(|((), (name, email))| Self::new(name, email))
    }
}

#[cfg(test)]
mod tests {
    use super::{EmailAddress, EmailError, IdentityName, IdentityNameError};

    fn name(text: &str) -> Result<IdentityName, IdentityNameError> {
        IdentityName::try_from(text)
    }

    fn email(text: &str) -> Result<EmailAddress, EmailError> {
        EmailAddress::try_from(text)
    }

    #[test]
    fn a_name_is_refused_for_the_first_rule_it_breaks() {
        assert_eq!(name(""), Err(IdentityNameError::Length));
        assert_eq!(name(&"a".repeat(129)), Err(IdentityNameError::Length));
        assert_eq!(name(" Owner"), Err(IdentityNameError::Whitespace));
        assert_eq!(name("Owner\u{a0}"), Err(IdentityNameError::Whitespace));
        for edge in [".", ",", ":", ";", "\"", "'", "\\", "<", ">"] {
            assert_eq!(name(&format!("{edge}Owner")), Err(IdentityNameError::Edge));
            assert_eq!(name(&format!("Owner{edge}")), Err(IdentityNameError::Edge));
        }
        assert_eq!(name("--upload-pack=x"), Err(IdentityNameError::LeadingDash));
        assert_eq!(name("Own\u{7}er"), Err(IdentityNameError::Control));
        assert_eq!(name("Own\u{85}er"), Err(IdentityNameError::Control));
        assert_eq!(name("Own\u{2028}er"), Err(IdentityNameError::Control));
        assert_eq!(name("Own\u{200b}er"), Err(IdentityNameError::Format));
        assert_eq!(name("Own\u{202e}er"), Err(IdentityNameError::Format));
        assert_eq!(name("Own\u{ad}er"), Err(IdentityNameError::Format));
        for forbidden in ["<", ">", "`", "\\"] {
            assert_eq!(
                name(&format!("Own{forbidden}er")),
                Err(IdentityNameError::Forbidden)
            );
        }
    }

    #[test]
    fn a_name_of_128_characters_counts_code_points_and_an_inner_apostrophe_is_fine() {
        assert!(name(&"é".repeat(128)).is_ok());
        assert_eq!(name(&"é".repeat(129)), Err(IdentityNameError::Length));
        assert!(name("Owner O'Example").is_ok());
        assert!(name("Owner - Example").is_ok());
    }

    #[test]
    fn an_email_is_refused_for_the_first_rule_it_breaks() {
        assert_eq!(
            email(&format!("{}@example.com", "a".repeat(250))),
            Err(EmailError::TooLong)
        );
        assert_eq!(email("-x@example.com"), Err(EmailError::LeadingDash));
        assert_eq!(email(".x@example.com"), Err(EmailError::EdgeDot));
        assert_eq!(email("x@example.com."), Err(EmailError::EdgeDot));
        assert_eq!(email("x@exam\u{200b}ple.com"), Err(EmailError::Format));
        for text in [
            "",
            "owner.example.com",
            "owner@example",
            "@example.com",
            "owner@@example.com",
            "owner@exa mple.com",
            "owner@example..com",
            "o\u{e9}@example.com",
            "owner@example.com\n",
            "a@b@example.com",
        ] {
            assert_eq!(email(text), Err(EmailError::NotPlainAscii), "{text:?}");
        }
    }

    #[test]
    fn an_email_of_254_characters_is_the_longest() {
        let local = "a".repeat(254 - "@example.com".len());
        assert!(email(&format!("{local}@example.com")).is_ok());
        assert_eq!(
            email(&format!("{local}a@example.com")),
            Err(EmailError::TooLong)
        );
    }

    #[test]
    fn plain_addresses_are_accepted() {
        for text in [
            "owner@example.com",
            "a.b+c%d_e-f@sub.example-1.com",
            "ci@example.invalid",
        ] {
            assert!(email(text).is_ok(), "{text}");
        }
    }
}
