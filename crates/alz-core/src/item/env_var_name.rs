//! The name of an environment variable in a layer file (spec §7.9).
//!
//! A name is a plain identifier, `^[A-Za-z_][A-Za-z0-9_]*$`. Anything else (`NAME=x`, a NUL, a
//! space) is refused, because a spawn would read `CODEX_HOME=/x` as the name `CODEX_HOME` with a
//! value. Whether a name is one of the protected variables is a separate question, asked of a
//! valid name only.

use crate::account::protected_variable;

/// A plain variable name. The text is kept as written: a name compares ignoring ASCII case only
/// when it is asked whether it is protected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EnvVarName(String);

/// Text that is not a plain variable name. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvVarNameError;

impl EnvVarName {
    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The protected variable this name stands for, ignoring ASCII case, in its fixed spelling.
    #[must_use]
    pub fn protected(&self) -> Option<&'static str> {
        protected_variable(&self.0)
    }
}

impl TryFrom<&str> for EnvVarName {
    type Error = EnvVarNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let starts_well = text
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
        let plain = text
            .chars()
            .all(|next| next.is_ascii_alphanumeric() || next == '_');
        if starts_well && plain {
            Ok(Self(text.to_owned()))
        } else {
            Err(EnvVarNameError)
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{prop_assert_eq, proptest};

    use super::EnvVarName;
    use crate::account::PROTECTED_VARIABLES;

    #[test]
    fn plain_names_are_accepted() {
        for name in ["HOME", "_x", "a1", "Path_2", "_"] {
            assert!(EnvVarName::try_from(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for name in [
            "",
            "1A",
            "A B",
            "A=1",
            "CODEX_HOME=/x",
            "A\0B",
            "A-B",
            "\u{c9}t\u{e9}",
            "HOME\u{212a}",
            "A\n",
        ] {
            assert!(EnvVarName::try_from(name).is_err(), "{name:?}");
        }
    }

    proptest! {
        #[test]
        fn a_name_matches_the_grammar_exactly(text in "\\PC{0,8}") {
            let grammar = !text.is_empty()
                && !text.starts_with(|c: char| c.is_ascii_digit())
                && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            prop_assert_eq!(EnvVarName::try_from(text.as_str()).is_ok(), grammar);
        }

        #[test]
        fn a_protected_name_is_found_in_any_ascii_case(
            index in 0..PROTECTED_VARIABLES.len(),
            flips in proptest::collection::vec(proptest::bool::ANY, 16),
        ) {
            let fixed = PROTECTED_VARIABLES.get(index).copied().unwrap_or("HOME");
            let varied: String = fixed
                .chars()
                .zip(flips.iter().cycle())
                .map(|(c, flip)| if *flip { c.to_ascii_lowercase() } else { c })
                .collect();
            let name = EnvVarName::try_from(varied.as_str()).expect("a plain name");
            prop_assert_eq!(name.protected(), Some(fixed));
        }
    }
}
