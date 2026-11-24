//! Reading the person's answer to the confirmation prompt (spec §7.13 "Trust and `alz account
//! allow`"): the person confirms by typing the account name, not `y`, and anything else is no.

use std::fmt;

use super::AccountName;

/// What the prompt got: one line the person typed, or the end of input.
///
/// The line is what arrived, without its line ending, and may hold anything, an interrupt
/// character included: Ctrl-C ends the line during the read and so arrives as a line that is not
/// the name (spec §7.13). `Debug` shows no text, because the answer is never printed.
#[derive(Clone, PartialEq, Eq)]
pub enum Answer {
    /// A line.
    Line(String),
    /// The end of input.
    Ended,
}

impl fmt::Debug for Answer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line(_) => formatter.write_str("Answer::Line(..)"),
            Self::Ended => formatter.write_str("Answer::Ended"),
        }
    }
}

/// Whether the person confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// The answer, without its surrounding spaces and tabs, is the name.
    Confirmed,
    /// Anything else, the end of input included.
    NotConfirmed,
}

impl Confirmation {
    /// Judges `answer` against the name that was asked for. Only the spaces and tabs around the
    /// answer are ignored; case, other white space and other text are not.
    #[must_use]
    pub fn of(expected: &AccountName, answer: &Answer) -> Self {
        match answer {
            Answer::Line(line) if line.trim_matches([' ', '\t']) == expected.as_str() => {
                Self::Confirmed
            }
            Answer::Line(_) | Answer::Ended => Self::NotConfirmed,
        }
    }

    /// Whether this is [`Confirmation::Confirmed`].
    #[must_use]
    pub const fn is_confirmed(self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn name() -> AccountName {
        AccountName::try_from("client-a").expect("a valid name")
    }

    fn line(text: &str) -> Answer {
        Answer::Line(text.to_owned())
    }

    #[test]
    fn the_name_confirms() {
        assert!(Confirmation::of(&name(), &line("client-a")).is_confirmed());
    }

    #[test]
    fn surrounding_spaces_and_tabs_are_ignored() {
        for text in [" client-a\t", "\t\tclient-a  ", "  client-a"] {
            assert!(
                Confirmation::of(&name(), &line(text)).is_confirmed(),
                "{text:?}"
            );
        }
    }

    #[test]
    fn the_wrong_answers_of_test_40_are_no() {
        for text in [
            "y",
            "yes",
            "",
            "client-b",
            "CLIENT-A",
            "client-a now",
            "client-a-api",
            "client",
            "client a",
            "client-a\x03",
            "\x03",
            "cli\x03ent-a",
            "client-a\r",
            "\u{a0}client-a",
            "client-a\u{3000}",
        ] {
            assert_eq!(
                Confirmation::of(&name(), &line(text)),
                Confirmation::NotConfirmed,
                "{text:?}"
            );
        }
    }

    #[test]
    fn the_end_of_input_is_no() {
        assert_eq!(
            Confirmation::of(&name(), &Answer::Ended),
            Confirmation::NotConfirmed
        );
    }

    #[test]
    fn an_answer_shows_no_text_in_debug() {
        assert_eq!(format!("{:?}", line("secret")), "Answer::Line(..)");
        assert_eq!(format!("{:?}", Answer::Ended), "Answer::Ended");
    }

    proptest! {
        #[test]
        fn a_name_padded_with_spaces_and_tabs_is_confirmed(
            before in "[ \t]{0,5}",
            after in "[ \t]{0,5}",
        ) {
            let answer = line(&format!("{before}client-a{after}"));
            prop_assert_eq!(Confirmation::of(&name(), &answer), Confirmation::Confirmed);
        }

        #[test]
        fn only_the_trimmed_name_confirms(text in "\\PC{0,20}") {
            let confirmed = Confirmation::of(&name(), &line(&text)).is_confirmed();
            prop_assert_eq!(confirmed, text.trim_matches([' ', '\t']) == "client-a");
        }
    }
}
