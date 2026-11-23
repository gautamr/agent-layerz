//! The line of git's standard error that a failed fetch reports (spec §6 "A failed fetch",
//! TASK-68). Git's standard error carries what the remote and the transport wrote, so it is
//! untrusted: the line is built in the five steps of the spec and holds nothing a terminal could
//! act on.

use std::fmt;

use crate::untrusted_text::Stripped;

/// The first non-empty line of git's standard error, as the spec §6 "A failed fetch" derives it.
/// The type is the proof that the line is stripped (decision-26, class 1), trimmed and not
/// empty, so a message that ends with `: <line>` never ends with `: ` alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FetchFailureLine(Stripped);

impl FetchFailureLine {
    /// The line for `stderr`, or `None` when git wrote no line (the message then ends after the
    /// quoted source). In this order:
    /// 1. the bytes are decoded as UTF-8, with U+FFFD in place of each invalid sequence;
    /// 2. the text is split at each line feed, and only there (a carriage return stays in its
    ///    line, and step 3 removes it);
    /// 3. control and format characters are removed from each line;
    /// 4. Unicode `White_Space` is removed from both ends of each line;
    /// 5. the first line that is not empty is the result.
    #[must_use]
    pub fn from_stderr(stderr: &[u8]) -> Option<Self> {
        String::from_utf8_lossy(stderr)
            .split('\n')
            .map(Stripped::from_untrusted)
            .map(|line| Stripped::from_untrusted(line.as_str().trim()))
            .find(|line| !line.as_str().is_empty())
            .map(Self)
    }

    /// The line, with no control or format character, no line feed and no space at either end.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The line as stripped text, for a message.
    #[must_use]
    pub fn into_stripped(self) -> Stripped {
        self.0
    }
}

impl fmt::Display for FetchFailureLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::FetchFailureLine;
    use crate::untrusted_text::{Stripped, is_control, is_format};

    fn line(stderr: &[u8]) -> Option<String> {
        FetchFailureLine::from_stderr(stderr).map(|line| line.as_str().to_owned())
    }

    #[test]
    fn git_wrote_nothing() {
        assert_eq!(line(b""), None);
    }

    #[test]
    fn blank_and_white_space_only_lines_are_skipped() {
        assert_eq!(
            line(b"\n   \n\t\n\xc2\xa0\xe3\x80\x80\nfatal: no such remote\nlater\n"),
            Some("fatal: no such remote".to_owned())
        );
    }

    #[test]
    fn a_line_of_control_characters_counts_as_empty() {
        assert_eq!(
            line(b"\x1b\r\n\xe2\x80\xae\n  ssh: denied  \n"),
            Some("ssh: denied".to_owned())
        );
    }

    #[test]
    fn only_blank_lines_leave_no_line() {
        assert_eq!(line(b"\n \r\n\t\n\x1b\n"), None);
    }

    #[test]
    fn a_carriage_return_inside_a_line_is_removed_and_does_not_split_it() {
        assert_eq!(
            line(b"remote: Counting\rfatal: denied\nnext\n"),
            Some("remote: Countingfatal: denied".to_owned())
        );
    }

    #[test]
    fn a_carriage_return_before_the_line_feed_stays_off_the_line() {
        assert_eq!(
            line(b"fatal: denied\r\nhint\r\n"),
            Some("fatal: denied".to_owned())
        );
    }

    #[test]
    fn bytes_that_are_not_utf8_show_as_the_replacement_character() {
        assert_eq!(
            line(b"fatal: \xff\xfe bad\n"),
            Some("fatal: \u{fffd}\u{fffd} bad".to_owned())
        );
        assert_eq!(line(b"\xff\n"), Some("\u{fffd}".to_owned()));
    }

    #[test]
    fn white_space_is_trimmed_after_the_control_characters_are_removed() {
        assert_eq!(line(b" \x07 ab \x1b\n"), Some("ab".to_owned()));
    }

    #[test]
    fn the_unicode_line_and_paragraph_separators_do_not_split_a_line() {
        assert_eq!(
            line("a\u{2028}b\u{2029}c\nd".as_bytes()),
            Some("abc".to_owned())
        );
    }

    #[test]
    fn displays_the_line() {
        assert_eq!(
            FetchFailureLine::from_stderr(b"x\n").map(|line| line.to_string()),
            Some("x".to_owned())
        );
    }

    /// Bytes from any value, and from the characters that matter to the line rules.
    fn any_stderr() -> impl Strategy<Value = Vec<u8>> {
        proptest::prop_oneof![
            proptest::collection::vec(any::<u8>(), 0..64),
            any::<String>().prop_map(String::into_bytes),
            "[a \t\r\n\u{1b}\u{200b}\u{202e}\u{2028}\u{a0}\u{fffd}\u{e9}]{0,24}"
                .prop_map(String::into_bytes),
        ]
    }

    /// The lines that survive the steps, stated without the code under test, to say that the
    /// result is `None` exactly when none survives.
    fn survivors(stderr: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(stderr)
            .split('\n')
            .map(|raw| {
                raw.chars()
                    .filter(|c| !is_control(*c) && !is_format(*c))
                    .collect::<String>()
            })
            .map(|stripped| stripped.trim().to_owned())
            .filter(|trimmed| !trimmed.is_empty())
            .collect()
    }

    proptest! {
        #[test]
        fn the_line_holds_no_control_or_format_character_and_no_line_feed(stderr in any_stderr()) {
            if let Some(found) = line(&stderr) {
                prop_assert!(found.chars().all(|c| !is_control(c) && !is_format(c) && c != '\n'));
            }
        }

        #[test]
        fn the_line_is_trimmed_and_stays_stripped(stderr in any_stderr()) {
            if let Some(found) = line(&stderr) {
                prop_assert_eq!(found.trim(), found.as_str());
                let restripped = Stripped::from_untrusted(&found);
                prop_assert_eq!(restripped.as_str(), found.as_str());
            }
        }

        #[test]
        fn there_is_no_line_exactly_when_none_survives_the_steps(stderr in any_stderr()) {
            let survivors = survivors(&stderr);
            prop_assert_eq!(line(&stderr), survivors.first().cloned());
        }

        #[test]
        fn a_carriage_return_inside_a_line_never_splits_it(
            before in "[a-z]{1,6}",
            after in "[a-z]{1,6}",
        ) {
            let stderr = format!("{before}\r{after}\nlater");
            prop_assert_eq!(line(stderr.as_bytes()), Some(format!("{before}{after}")));
        }
    }
}
