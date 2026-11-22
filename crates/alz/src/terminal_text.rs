//! Untrusted text reaches a terminal only after its control, line-separator and format
//! characters are removed (decision-26), so an argument cannot move the cursor, start a new
//! line that looks like a diagnostic, recolour the screen or reorder text.
//!
//! This matches `src/core/sanitize.ts`, the reference: control is category Cc (line feed
//! included) plus U+2028 and U+2029, and format is category Cf. The `parity` test compares the
//! two over every code point.

mod format_characters;

use std::cmp::Ordering;

use self::format_characters::FORMAT_CHARACTERS;

/// Control characters: C0, DEL, C1 (category Cc), and the Unicode line and paragraph separators.
fn is_control(character: char) -> bool {
    character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
}

/// Format characters (category Cf): zero-width, bidirectional controls, soft hyphen and so on.
fn is_format(character: char) -> bool {
    FORMAT_CHARACTERS
        .binary_search_by(|&(first, last)| {
            if last < character {
                Ordering::Less
            } else if first > character {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// Whether a character from an untrusted source must not reach a terminal.
fn is_untrusted(character: char) -> bool {
    is_control(character) || is_format(character)
}

/// Text from an untrusted source, safe to print: no control, line-separator or format character,
/// line feeds included, so the text stays on its own line.
pub fn strip_untrusted(text: &str) -> String {
    text.chars()
        .filter(|character| !is_untrusted(*character))
        .collect()
}

/// Text that holds trusted line structure, with every other untrusted character removed. Only
/// for text whose untrusted fragments have already gone through [`strip_untrusted`]: this is the
/// last guard, not the sanitiser.
pub fn strip_untrusted_keeping_line_feeds(text: &str) -> String {
    text.chars()
        .filter(|character| *character == '\n' || !is_untrusted(*character))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{is_untrusted, strip_untrusted, strip_untrusted_keeping_line_feeds};

    #[test]
    fn keeps_ordinary_text() {
        assert_eq!(strip_untrusted("error: bad flag"), "error: bad flag");
    }

    #[test]
    fn removes_line_feeds_and_the_line_and_paragraph_separators() {
        assert_eq!(strip_untrusted("a\nb\u{2028}c\u{2029}d"), "abcd");
    }

    #[test]
    fn removes_escape_sequences_carriage_returns_and_bidirectional_controls() {
        assert_eq!(
            strip_untrusted("a\u{1b}[31mb\rc\u{202e}d\u{2066}e\u{7}"),
            "a[31mbcde"
        );
    }

    #[test]
    fn removes_format_characters_added_after_the_first_unicode_releases() {
        assert_eq!(
            strip_untrusted("a\u{890}b\u{8e2}c\u{110bd}d\u{13430}e\u{1bca0}f\u{1d173}g"),
            "abcdefg"
        );
    }

    #[test]
    fn keeps_letters_of_other_scripts() {
        assert_eq!(strip_untrusted("Größe 日本語"), "Größe 日本語");
    }

    #[test]
    fn the_last_guard_keeps_line_feeds_and_removes_everything_else_untrusted() {
        assert_eq!(
            strip_untrusted_keeping_line_feeds("a\nb\u{1b}c\u{2028}d\u{202e}e"),
            "a\nbcde"
        );
    }

    /// The inclusive ranges of code points that the reference removes, from the fixture generated
    /// from `src/core/sanitize.ts`.
    fn removed_by_the_reference() -> Vec<(u32, u32)> {
        include_str!("../tests/fixtures/untrusted-code-points.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_once(' '))
            .map(|(first, last)| {
                (
                    u32::from_str_radix(first, 16).expect("a hexadecimal code point"),
                    u32::from_str_radix(last, 16).expect("a hexadecimal code point"),
                )
            })
            .collect()
    }

    #[test]
    fn parity_with_the_typescript_reference_over_every_code_point() {
        let reference = removed_by_the_reference();
        let differing: Vec<String> = (0..=0x0010_FFFF)
            .filter_map(char::from_u32)
            .filter(|character| {
                let code = u32::from(*character);
                let removed = reference
                    .iter()
                    .any(|(first, last)| (*first..=*last).contains(&code));
                is_untrusted(*character) != removed
            })
            .map(|character| format!("U+{:04X}", u32::from(character)))
            .collect();
        assert!(
            differing.is_empty(),
            "the sanitiser and the reference disagree on: {differing:?} (regenerate with \
             `node crates/alz/tools/untrusted-code-points.mjs`)"
        );
    }
}
