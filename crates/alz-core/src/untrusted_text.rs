//! Untrusted text reaches a terminal in one of three classes (decision-26, spec §10.1). This
//! module holds the first two as types, each with one constructor that takes any text, so a
//! value of the type is the proof that the text went through the rule:
//!
//! - [`Stripped`], class 1, for diagnostics: control and format characters are removed;
//! - [`Escaped`], class 2, for text a person decides on: nothing is removed, every character that
//!   is not printable ASCII is written as an escape;
//! - [`QuotedKey`], a key name taken from the input, stripped, cut and quoted, with a single quote
//!   inside it doubled (decision-28, spec §10.1).
//!
//! Class 3, usage output, stays in the binary, which strips each argument fragment with
//! [`Stripped`].
//!
//! [`Stripped`] matches `stripControl` of the TypeScript reference (`src/core/sanitize.ts`);
//! the `parity` test compares the two over every code point.

mod format_characters;

use std::cmp::Ordering;
use std::fmt;

use self::format_characters::FORMAT_CHARACTERS;

/// The most code points of a key that a diagnostic prints, counted after stripping.
const QUOTED_KEY_LIMIT: usize = 64;

/// Control characters: C0, DEL and C1 (category Cc), and the line and paragraph separators.
pub(crate) const fn is_control(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}'
    )
}

/// Format characters (category Cf): zero-width, bidirectional controls, soft hyphen and so on.
pub(crate) fn is_format(character: char) -> bool {
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

/// Class 1: untrusted text with every control and format character removed, line feed included,
/// so the text stays on its own line and cannot move the cursor, recolour the screen or reorder
/// text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Stripped(String);

impl Stripped {
    /// Removes U+0000 to U+001F, U+007F to U+009F, U+2028, U+2029 and every character of the
    /// Unicode general category Cf from `text`. Stripping a stripped text changes nothing.
    #[must_use]
    pub fn from_untrusted(text: &str) -> Self {
        Self(
            text.chars()
                .filter(|character| !is_control(*character) && !is_format(*character))
                .collect(),
        )
    }

    /// The text without its control and format characters.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The text, handed over.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Stripped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Class 2: untrusted text in double quotes for a person to decide on. Nothing is removed:
/// printable ASCII (U+0020 to U+007E) stays, `"` and `\` get a backslash, and every other
/// character is written `\u{XXXX}`, in upper-case hexadecimal with at least four digits. Two
/// texts that differ only by an invisible or look-alike character never print alike.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Escaped(String);

impl Escaped {
    /// Quotes and escapes `text`. Different texts give different results.
    #[must_use]
    pub fn from_untrusted(text: &str) -> Self {
        let body: String = text.chars().map(escape).collect();
        Self(format!("\"{body}\""))
    }

    /// The quoted text, printable ASCII only.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Escaped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One character of class 2. A backslash is always doubled and an escape always starts with
/// `\u{`, so no two texts give the same result.
fn escape(character: char) -> String {
    match character {
        '"' | '\\' => format!("\\{character}"),
        ' '..='~' => character.to_string(),
        _ => format!("\\u{{{:04X}}}", u32::from(character)),
    }
}

/// A key name taken from the input, as a diagnostic prints it (decision-28, spec §10.1): stripped,
/// cut after 64 code points counted after stripping, with `…` added when it is cut, and shown in
/// single quotes. A single quote inside the key is written twice, as in a YAML single-quoted
/// scalar (`it's` is shown as `'it''s'`), so the key cannot close its own quotes and pass the rest
/// as message text. The cut counts the key's own code points, never the doubled quotes. It is the
/// only text from the input that a YAML, JSON or schema diagnostic may hold.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuotedKey {
    /// At most [`QUOTED_KEY_LIMIT`] code points, the first of the stripped key.
    shown: Stripped,
    cut: bool,
}

impl QuotedKey {
    /// Strips `key`, then keeps its first 64 code points.
    #[must_use]
    pub fn from_untrusted(key: &str) -> Self {
        let stripped = Stripped::from_untrusted(key);
        let cut = stripped.as_str().chars().nth(QUOTED_KEY_LIMIT).is_some();
        // A prefix of stripped text holds no control or format character either.
        let shown = Stripped(stripped.as_str().chars().take(QUOTED_KEY_LIMIT).collect());
        Self { shown, cut }
    }
}

impl fmt::Display for QuotedKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ellipsis = if self.cut { "…" } else { "" };
        let doubled = self.shown.as_str().replace('\'', "''");
        write!(formatter, "'{doubled}{ellipsis}'")
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, prop_assert_ne, proptest};

    use super::{Escaped, QuotedKey, Stripped, is_control, is_format};

    fn stripped(text: &str) -> String {
        Stripped::from_untrusted(text).into_string()
    }

    #[test]
    fn keeps_ordinary_text() {
        assert_eq!(stripped("error: bad flag"), "error: bad flag");
    }

    #[test]
    fn removes_line_feeds_and_the_line_and_paragraph_separators() {
        assert_eq!(stripped("a\nb\u{2028}c\u{2029}d"), "abcd");
    }

    #[test]
    fn removes_escape_sequences_carriage_returns_and_bidirectional_controls() {
        assert_eq!(
            stripped("a\u{1b}[31mb\rc\u{202e}d\u{2066}e\u{7}"),
            "a[31mbcde"
        );
    }

    #[test]
    fn removes_format_characters_added_after_the_first_unicode_releases() {
        assert_eq!(
            stripped("a\u{890}b\u{8e2}c\u{110bd}d\u{13430}e\u{1bca0}f\u{1d173}g"),
            "abcdefg"
        );
    }

    #[test]
    fn keeps_letters_of_other_scripts() {
        assert_eq!(stripped("Größe 日本語"), "Größe 日本語");
    }

    #[test]
    fn displays_the_stripped_text() {
        assert_eq!(Stripped::from_untrusted("a\u{1b}b").to_string(), "ab");
    }

    #[test]
    fn escaped_keeps_printable_ascii_and_escapes_quotes_and_backslashes() {
        assert_eq!(Escaped::from_untrusted("a b~").as_str(), "\"a b~\"");
        assert_eq!(Escaped::from_untrusted(r#"a"b\c"#).as_str(), r#""a\"b\\c""#);
    }

    #[test]
    fn escaped_writes_every_other_character_in_upper_case_hexadecimal_of_four_digits_or_more() {
        assert_eq!(
            Escaped::from_untrusted("\n\u{7f}é\u{202e}\u{1f600}\u{10ffff}").to_string(),
            r#""\u{000A}\u{007F}\u{00E9}\u{202E}\u{1F600}\u{10FFFF}""#
        );
    }

    #[test]
    fn escaped_tells_a_look_alike_letter_from_the_letter() {
        assert_ne!(
            Escaped::from_untrusted("a"),
            Escaped::from_untrusted("\u{430}")
        );
        assert_ne!(
            Escaped::from_untrusted("ab"),
            Escaped::from_untrusted("a\u{200b}b")
        );
    }

    #[test]
    fn a_short_key_is_stripped_and_quoted() {
        assert_eq!(
            QuotedKey::from_untrusted("name\u{1b}[31m\n\u{202e}").to_string(),
            "'name[31m'"
        );
    }

    #[test]
    fn a_key_of_64_code_points_is_not_cut() {
        let key = "é".repeat(64);
        assert_eq!(
            QuotedKey::from_untrusted(&key).to_string(),
            format!("'{key}'")
        );
    }

    #[test]
    fn a_longer_key_is_cut_after_64_code_points_counted_after_stripping() {
        let key = format!("{}{}", "é\u{200b}".repeat(64), "x");
        assert_eq!(
            QuotedKey::from_untrusted(&key).to_string(),
            format!("'{}…'", "é".repeat(64))
        );
    }

    #[test]
    fn a_single_quote_inside_a_key_is_doubled_so_the_key_cannot_close_its_quotes() {
        assert_eq!(QuotedKey::from_untrusted("a'b").to_string(), "'a''b'");
        assert_eq!(
            QuotedKey::from_untrusted("x': forged '").to_string(),
            "'x'': forged '''"
        );
        assert_eq!(QuotedKey::from_untrusted("'").to_string(), "''''");
    }

    #[test]
    fn a_key_with_a_backslash_is_shown_as_it_is() {
        assert_eq!(QuotedKey::from_untrusted(r"a\'b").to_string(), r"'a\''b'");
    }

    #[test]
    fn the_cut_counts_the_keys_own_code_points_not_the_doubled_quotes() {
        let key = "'".repeat(65);
        assert_eq!(
            QuotedKey::from_untrusted(&key).to_string(),
            format!("'{}…'", "''".repeat(64))
        );
    }

    #[test]
    fn an_empty_key_is_two_quotes() {
        assert_eq!(QuotedKey::from_untrusted("\u{1b}").to_string(), "''");
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
                stripped(&character.to_string()).is_empty() != removed
            })
            .map(|character| format!("U+{:04X}", u32::from(character)))
            .collect();
        assert!(
            differing.is_empty(),
            "the stripped text and the reference disagree on: {differing:?} (regenerate the \
             fixture with `node crates/alz/tools/untrusted-code-points.mjs`)"
        );
    }

    /// Text drawn from every code point, and from the characters that matter to the escapes.
    fn any_text() -> impl Strategy<Value = String> {
        proptest::prop_oneof![any::<String>(), "[a\\\\\"'u{}0-9A-F\u{e9}\u{200b}\n]{0,12}"]
    }

    proptest! {
        #[test]
        fn stripped_text_holds_no_control_or_format_character(text in any_text()) {
            let result = Stripped::from_untrusted(&text);
            prop_assert!(result.as_str().chars().all(|c| !is_control(c) && !is_format(c)));
        }

        #[test]
        fn stripping_is_idempotent(text in any_text()) {
            let once = Stripped::from_untrusted(&text);
            prop_assert_eq!(Stripped::from_untrusted(once.as_str()), once);
        }

        #[test]
        fn escaped_text_is_printable_ascii(text in any_text()) {
            prop_assert!(Escaped::from_untrusted(&text).as_str().chars().all(|c| matches!(c, ' '..='~')));
        }

        #[test]
        fn two_different_texts_never_escape_alike(first in any_text(), second in any_text()) {
            if first != second {
                prop_assert_ne!(Escaped::from_untrusted(&first), Escaped::from_untrusted(&second));
            }
        }

        #[test]
        fn a_quoted_key_holds_at_most_64_code_points_between_its_quotes(key in any_text()) {
            let printed = QuotedKey::from_untrusted(&key).to_string();
            let inner = printed.strip_prefix('\'').and_then(|rest| rest.strip_suffix('\''));
            prop_assert!(inner.is_some());
            let inner = inner.unwrap_or_default();
            // Every single quote inside is one of a pair, so the key cannot close its quotes.
            let undoubled = inner.replace("''", "'");
            prop_assert!(!inner.replace("''", "").contains('\''));
            let inner = undoubled.as_str();
            let stripped_length = stripped(&key).chars().count();
            let cut = stripped_length > 64;
            prop_assert_eq!(inner.chars().count(), stripped_length.min(64) + usize::from(cut));
            prop_assert_eq!(inner.ends_with('…') && cut, cut);
            let kept = inner.strip_suffix('…').filter(|_| cut).unwrap_or(inner);
            prop_assert!(kept.chars().all(|c| !is_control(c) && !is_format(c)));
        }
    }
}
