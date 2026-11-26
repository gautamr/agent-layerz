//! The name of a layer (spec §5.1): lower-case ASCII letters and digits in groups joined by a
//! single `-` or `.`, that is `^[a-z0-9]+(?:[-.][a-z0-9]+)*$`.

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Printable, sealed::Sealed};

/// A layer name. Its alphabet is printable ASCII, so it goes into a message as it is, and its
/// `Ord` is the code point order of §6. It holds the contract's name, so the core gives an adapter
/// its layers' names without a second check ([`LayerName::to_contract`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerName(alz_adapter::LayerName);

/// Text that is not a layer name. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a layer name: lower-case letters and digits in groups joined by a single '-' or '.'")]
pub struct LayerNameError;

impl LayerNameError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be lower-case letters and digits in groups joined by a single '-' or '.'"
    }
}

impl LayerName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The name as the adapter contract holds it, which is the same text.
    #[must_use]
    pub fn to_contract(&self) -> alz_adapter::LayerName {
        self.0.clone()
    }
}

impl TryFrom<&str> for LayerName {
    type Error = LayerNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        alz_adapter::LayerName::try_from(text)
            .map(Self)
            .map_err(|alz_adapter::LayerNameError| LayerNameError)
    }
}

impl fmt::Display for LayerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.as_str())
    }
}

impl Sealed for LayerName {}

impl Printable for LayerName {
    fn printable(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::LayerName;

    fn accepts(text: &str) -> bool {
        LayerName::try_from(text).is_ok()
    }

    #[test]
    fn a_name_goes_into_a_message_as_it_is() {
        use crate::diagnostic::{Message, MessagePart};
        let name = LayerName::try_from("client.team-1").expect("a name");
        let message = Message::new("layer ").then(MessagePart::printable(&name));
        assert_eq!(message.to_string(), "layer client.team-1");
    }

    #[test]
    fn accepts_names_in_groups_joined_by_one_separator() {
        for text in ["personal", "a", "client-a", "org.team", "a1-b2.c3", "0"] {
            assert!(accepts(text), "{text}");
        }
    }

    #[test]
    fn refuses_what_the_pattern_refuses() {
        for text in [
            "",
            "-",
            ".",
            "-a",
            "a-",
            ".a",
            "a.",
            "a--b",
            "a..b",
            "a-.b",
            "A",
            "aB",
            "a b",
            "a_b",
            "a/b",
            "../x",
            "é",
            "a\n",
            "a\u{200b}",
        ] {
            assert!(!accepts(text), "{text:?}");
        }
    }

    proptest! {
        #[test]
        fn a_name_that_is_made_reads_back_as_itself(
            text in "[a-z0-9]{1,6}([-.][a-z0-9]{1,6}){0,4}"
        ) {
            let name = LayerName::try_from(text.as_str()).expect("the generator makes names");
            prop_assert_eq!(name.as_str(), text.as_str());
            prop_assert_eq!(LayerName::try_from(name.as_str()), Ok(name));
        }

        #[test]
        fn text_with_a_character_outside_the_alphabet_is_never_a_name(
            head in "[a-z0-9.-]{0,8}",
            stray in any::<char>().prop_filter(
                "outside the alphabet",
                |c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '.'),
            ),
            tail in "[a-z0-9.-]{0,8}",
        ) {
            let text = format!("{head}{stray}{tail}");
            prop_assert!(LayerName::try_from(text.as_str()).is_err());
        }

        #[test]
        fn a_separator_at_an_edge_or_doubled_is_never_a_name(
            inner in "[a-z0-9]{1,4}",
            separator in "[-.]",
        ) {
            let leading = format!("{separator}{inner}");
            let trailing = format!("{inner}{separator}");
            let doubled = format!("{inner}{separator}{separator}{inner}");
            for text in [leading, trailing, doubled] {
                prop_assert!(LayerName::try_from(text.as_str()).is_err(), "{}", text);
            }
        }
    }
}
