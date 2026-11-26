//! The id of a target (spec §5.2 **Target ids**): 1 to 32 characters, a lower-case ASCII letter
//! and then lower-case ASCII letters, digits and `-`, that is `^[a-z][a-z0-9-]{0,31}$`.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// The longest target id, in characters.
const MAX_LENGTH: usize = 32;

/// A target id: the name of an agent tool that alz may write configuration for, such as
/// `claude`, `codex` or `antigravity`. It is parsed once, by [`TryFrom`] or [`FromStr`], and a
/// value is always well formed. Its alphabet is printable ASCII, so it goes into a message as it
/// is, and its [`Ord`] is the code point order of §6, which the registry uses for `<ids>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(String);

/// Text that is not a target id. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "not a target id: 1 to 32 characters, a lower-case letter and then lower-case letters, digits and '-'"
)]
pub struct TargetIdError;

impl TargetId {
    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A character after the first: a lower-case ASCII letter, a digit or `-`.
const fn is_tail(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
}

impl TryFrom<&str> for TargetId {
    type Error = TargetIdError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        // Bytes decide: every byte of a non-ASCII character is at least 0x80, so it is refused.
        let mut bytes = text.bytes();
        let well_formed = text.len() <= MAX_LENGTH
            && bytes
                .next()
                .is_some_and(|head| head.is_ascii_lowercase() && bytes.all(is_tail));
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(TargetIdError)
        }
    }
}

impl FromStr for TargetId {
    type Err = TargetIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text)
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::{TargetId, TargetIdError};

    fn accepts(text: &str) -> bool {
        TargetId::try_from(text).is_ok()
    }

    /// The grammar of §5.2, written a second way: no length check first, a regular expression
    /// read left to right.
    fn grammar(text: &str) -> bool {
        let mut characters = text.chars();
        let head_ok = characters.next().is_some_and(|c| c.is_ascii_lowercase());
        let tail: Vec<char> = characters.collect();
        head_ok
            && tail.len() <= 31
            && tail
                .iter()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
    }

    #[test]
    fn the_spec_examples_are_target_ids() {
        for text in [
            "claude",
            "codex",
            "antigravity",
            "a",
            "a-b",
            "x9",
            "a--b",
            "a-",
        ] {
            assert!(accepts(text), "{text}");
        }
    }

    #[test]
    fn the_spec_counter_examples_are_not() {
        for text in ["Claude", "9x", "my_tool", ""] {
            assert!(!accepts(text), "{text:?}");
        }
    }

    #[test]
    fn refuses_what_the_pattern_refuses() {
        for text in [
            "-a",
            "a b",
            "a.b",
            "a/b",
            "é",
            "caf\u{e9}",
            "a\n",
            "\na",
            "a\u{200b}",
            "A",
            "aB",
            "../x",
            " a",
        ] {
            assert!(!accepts(text), "{text:?}");
        }
    }

    #[test]
    fn thirty_two_characters_is_the_longest_id() {
        let longest = format!("a{}", "b".repeat(31));
        assert_eq!(longest.len(), 32);
        assert!(accepts(&longest));
        assert!(!accepts(&format!("{longest}b")));
    }

    #[test]
    fn the_error_holds_no_input_and_names_the_rule() {
        let error = TargetId::try_from("my_tool").expect_err("not an id");
        assert_eq!(error, TargetIdError);
        let message = error.to_string();
        assert!(!message.contains("my_tool"));
        assert!(message.contains("lower-case"));
    }

    #[test]
    fn an_id_prints_as_it_was_written_and_parses_through_from_str() {
        let id: TargetId = "antigravity".parse().expect("an id");
        assert_eq!(id.to_string(), "antigravity");
        assert_eq!(id.as_str(), "antigravity");
    }

    #[test]
    fn ids_order_by_code_point() {
        let mut ids: Vec<TargetId> = ["codex", "claude", "a1", "a-"]
            .into_iter()
            .map(|text| TargetId::try_from(text).expect("an id"))
            .collect();
        ids.sort();
        let sorted: Vec<&str> = ids.iter().map(TargetId::as_str).collect();
        assert_eq!(sorted, ["a-", "a1", "claude", "codex"]);
    }

    proptest! {
        #[test]
        fn an_id_that_is_made_reads_back_as_itself(text in "[a-z][a-z0-9-]{0,31}") {
            let id = TargetId::try_from(text.as_str()).expect("the generator makes ids");
            prop_assert_eq!(id.as_str(), text.as_str());
            prop_assert_eq!(id.to_string(), text.clone());
            prop_assert_eq!(TargetId::try_from(id.as_str()), Ok(id));
        }

        #[test]
        fn any_text_is_an_id_exactly_when_the_grammar_says_so(
            text in "[a-zA-Z0-9_ ./\\n\\u{e9}\\u{200b}-]{0,40}"
        ) {
            prop_assert_eq!(accepts(&text), grammar(&text), "{:?}", text);
        }

        #[test]
        fn text_with_a_character_outside_the_alphabet_is_never_an_id(
            head in "[a-z][a-z0-9-]{0,10}",
            stray in any::<char>().prop_filter(
                "outside the alphabet",
                |c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'),
            ),
            tail in "[a-z0-9-]{0,10}",
        ) {
            let text = format!("{head}{stray}{tail}");
            prop_assert!(!accepts(&text), "{:?}", text);
        }

        #[test]
        fn a_digit_or_dash_first_or_text_over_32_characters_is_never_an_id(
            first in "[0-9-]",
            rest in "[a-z0-9-]{0,10}",
            long in "[a-z][a-z0-9-]{32,40}",
        ) {
            let bad_head = format!("{first}{rest}");
            prop_assert!(!accepts(&bad_head), "{:?}", bad_head);
            prop_assert!(!accepts(&long), "{:?}", long);
        }
    }
}
