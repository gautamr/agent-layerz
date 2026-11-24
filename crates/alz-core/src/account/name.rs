//! Account names (spec §7.13 "Account names"). The name becomes part of a folder path and of the
//! shell hook's output, so nothing but `^[a-z0-9][a-z0-9-]*$`, at most 64 characters, is a name.

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Printable, sealed::Sealed};

/// The most characters of an account name.
const MAX_LENGTH: usize = 64;

/// An account name. Its alphabet is printable ASCII, so it goes into a message as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountName(String);

/// Text that is not an account name. The error holds no input: the value may be a path or a
/// secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "not an account name: lower-case letters, digits and '-', starting with a letter or digit, at most 64 characters"
)]
pub struct AccountNameError;

impl AccountNameError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be an account name: lower-case letters, digits and '-', starting with a letter or digit, at most 64 characters"
    }
}

impl AccountName {
    /// The name `default`, which `allowed.yaml` records for a target whose layer chooses the
    /// default account.
    #[must_use]
    pub fn default_account() -> Self {
        Self("default".to_owned())
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AccountName {
    type Error = AccountNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let mut bytes = text.bytes();
        let starts_well = bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
        let rest_is_well =
            bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if starts_well && rest_is_well && text.len() <= MAX_LENGTH {
            Ok(Self(text.to_owned()))
        } else {
            Err(AccountNameError)
        }
    }
}

impl fmt::Display for AccountName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Sealed for AccountName {}

impl Printable for AccountName {
    fn printable(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::AccountName;

    fn accepts(text: &str) -> bool {
        AccountName::try_from(text).is_ok()
    }

    #[test]
    fn accepts_the_names_the_spec_gives() {
        for text in ["client-a", "default", "2client", "a", "client-a-api", "a-"] {
            assert!(accepts(text), "{text}");
        }
        assert!(accepts(&"a".repeat(64)));
    }

    #[test]
    fn refuses_what_test_29_refuses() {
        for text in [
            "Client-A",
            "-leading",
            "~/secret-dir",
            "/etc/secret-dir",
            "../secret-dir",
            "a/secret-dir",
            "AKIAIOSFODNN7EXAMPLE",
            "client a",
            "",
            "a.b",
            "a_b",
            "é",
            "a\n",
        ] {
            assert!(!accepts(text), "{text:?}");
        }
        assert!(!accepts(&"a".repeat(65)));
    }

    proptest! {
        #[test]
        fn a_name_that_is_made_reads_back_as_itself(text in "[a-z0-9][a-z0-9-]{0,63}") {
            let name = AccountName::try_from(text.as_str()).expect("the generator makes names");
            prop_assert_eq!(name.as_str(), text.as_str());
            prop_assert_eq!(AccountName::try_from(name.as_str()), Ok(name));
        }

        #[test]
        fn text_that_breaks_the_rule_is_never_a_name(
            text in any::<String>().prop_filter(
                "outside the rule",
                |text| {
                    let mut chars = text.chars();
                    let starts = chars
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
                    !(starts
                        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                        && text.len() <= 64)
                },
            )
        ) {
            prop_assert!(AccountName::try_from(text.as_str()).is_err());
        }

        #[test]
        fn a_name_one_character_too_long_is_never_a_name(text in "[a-z0-9][a-z0-9-]{64,80}") {
            prop_assert!(AccountName::try_from(text.as_str()).is_err());
        }
    }
}
