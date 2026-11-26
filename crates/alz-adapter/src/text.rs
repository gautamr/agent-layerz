//! Untrusted text for a diagnostic (spec §10.1, decision-26): the rule that removes control and
//! format characters, and the form in which a key from the input is shown. An adapter that names
//! a key of the input in a diagnostic message quotes it with [`QuotedKey`]; the core strips every
//! message again when it emits it, so the text here is the same rule, not a second one.
//!
//! [`Stripped`] removes the control and format characters of spec §10.1; the table of format
//! characters is checked against the Unicode data in its own module.

mod format_characters;

use std::cmp::Ordering;
use std::fmt;

use self::format_characters::FORMAT_CHARACTERS;

/// The most code points of a key that a diagnostic prints, counted after stripping.
const QUOTED_KEY_LIMIT: usize = 64;

/// Whether `character` is a control character: C0, DEL and C1 (category Cc), or the line or
/// paragraph separator.
#[must_use]
pub const fn is_control(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}'
    )
}

/// Whether `character` is a format character (category Cf): zero-width, bidirectional controls,
/// soft hyphen and so on.
#[must_use]
pub fn is_format(character: char) -> bool {
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
    use super::{QuotedKey, Stripped, is_control, is_format};

    #[test]
    fn stripping_removes_controls_and_format_characters_and_keeps_other_scripts() {
        assert_eq!(
            Stripped::from_untrusted("a\u{1b}[31mb\u{202e}c\nd\u{2028}e").as_str(),
            "a[31mbcde"
        );
        assert_eq!(
            Stripped::from_untrusted("Größe 日本語").as_str(),
            "Größe 日本語"
        );
        assert!(is_control('\u{7f}') && is_format('\u{200b}'));
    }

    #[test]
    fn a_quoted_key_is_stripped_quoted_and_its_single_quotes_are_doubled() {
        assert_eq!(
            QuotedKey::from_untrusted("na\u{1b}me'x").to_string(),
            "'name''x'"
        );
        assert_eq!(QuotedKey::from_untrusted("\u{1b}").to_string(), "''");
    }

    #[test]
    fn a_quoted_key_is_cut_after_64_code_points_counted_after_stripping() {
        let key = format!("{}x", "é\u{200b}".repeat(64));
        assert_eq!(
            QuotedKey::from_untrusted(&key).to_string(),
            format!("'{}…'", "é".repeat(64))
        );
        let exact = "é".repeat(64);
        assert_eq!(
            QuotedKey::from_untrusted(&exact).to_string(),
            format!("'{exact}'")
        );
    }
}
