//! The text of a YAML file after the checks that come before parsing (spec §7.13): at most 1 MiB,
//! strict UTF-8, and no control character except tab, line feed and carriage return.

use crate::diagnostic::Position;
use crate::yaml::error::{YamlError, YamlErrorKind};

/// The largest YAML file alz reads: 1 MiB (1048576 bytes).
pub const MAX_BYTES: usize = 1_048_576;

/// The text of one YAML file that passed the size cap, strict UTF-8 and the raw control-character
/// check. The only way to get one is [`YamlText::from_bytes`], so the parser can take no text that
/// skipped them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlText(String);

impl YamlText {
    /// Checks `bytes` in this order: the cap (1048576 bytes pass, one more fails), strict UTF-8
    /// (the error holds the line and column of the first invalid byte), then the raw control
    /// characters (decision-28 as amended on 2026-11-23): tab, line feed and carriage return are
    /// allowed; every other character of U+0000 to U+001F, U+007F to U+009F, U+2028 and U+2029 is
    /// refused.
    ///
    /// # Errors
    ///
    /// [`YamlErrorKind::TooLarge`], [`YamlErrorKind::InvalidUtf8`] or
    /// [`YamlErrorKind::ControlCharacter`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, YamlError> {
        if bytes.len() > MAX_BYTES {
            return Err(YamlError::new(YamlErrorKind::TooLarge, None));
        }
        let text = std::str::from_utf8(bytes).map_err(|failure| {
            let valid = bytes.get(..failure.valid_up_to()).unwrap_or_default();
            let prefix = std::str::from_utf8(valid).unwrap_or_default();
            YamlError::new(YamlErrorKind::InvalidUtf8, Some(end_of(prefix)))
        })?;
        match text
            .char_indices()
            .find(|&(_, character)| is_forbidden_in_raw_text(character))
        {
            Some((offset, _)) => Err(YamlError::new(
                YamlErrorKind::ControlCharacter,
                Some(locate(text, offset)),
            )),
            None => Ok(Self(text.to_owned())),
        }
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `character` is refused in a file's raw text: a control character other than tab, line
/// feed and carriage return.
#[must_use]
pub const fn is_forbidden_in_raw_text(character: char) -> bool {
    !matches!(character, '\t' | '\n' | '\r') && is_control(character)
}

/// Whether `character` is refused in a key or value once YAML has applied its escapes: a control
/// character other than tab and line feed, so a carriage return fails here and not in raw text.
#[must_use]
pub const fn is_forbidden_when_decoded(character: char) -> bool {
    !matches!(character, '\t' | '\n') && is_control(character)
}

/// What a decoded key or value may hold beyond tab and line feed. A file that holds file-system
/// paths (`allowed.yaml`, spec §7.13 "Allow state") reads the characters that alz writes escaped
/// and that a path may hold; every other file reads none of them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DecodedText {
    /// Tab and line feed only.
    #[default]
    Strict,
    /// Also U+007F to U+009F, U+2028 and U+2029, which a path can hold and the emitter escapes.
    Paths,
}

impl DecodedText {
    /// Whether a key or value may not hold `character` once YAML has applied its escapes.
    #[must_use]
    pub const fn refuses(self, character: char) -> bool {
        is_forbidden_when_decoded(character)
            && match self {
                Self::Strict => true,
                Self::Paths => {
                    !matches!(character, '\u{007F}'..='\u{009F}' | '\u{2028}' | '\u{2029}')
                }
            }
    }
}

/// The set decision-28 names: U+0000 to U+001F, U+007F to U+009F, U+2028 and U+2029.
const fn is_control(character: char) -> bool {
    matches!(
        character,
        '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}' | '\u{2028}' | '\u{2029}'
    )
}

/// The line and column, from 1 and in characters, of the character that starts at byte `offset`
/// of `text`. A line ends at a line feed, a carriage return or a carriage return and a line feed
/// together, as YAML reads them.
fn locate(text: &str, offset: usize) -> Position {
    let (line, column, _) = text
        .char_indices()
        .take_while(|&(start, _)| start < offset)
        .fold(
            (1_u32, 1_u32, false),
            |(line, column, after_cr), (_, c)| match (c, after_cr) {
                ('\r', _) => (line.saturating_add(1), 1, true),
                ('\n', true) => (line, column, false),
                ('\n', false) => (line.saturating_add(1), 1, false),
                _ => (line, column.saturating_add(1), false),
            },
        );
    Position::new(line, column).unwrap_or(Position::FIRST)
}

/// The position just after the last character of `text`.
fn end_of(text: &str) -> Position {
    locate(text, text.len().saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::{MAX_BYTES, YamlText, locate};
    use crate::diagnostic::Position;
    use crate::yaml::error::YamlErrorKind;

    fn kind(bytes: &[u8]) -> Option<YamlErrorKind> {
        YamlText::from_bytes(bytes).err().map(|e| e.kind().clone())
    }

    fn at(line: u32, column: u32) -> Option<Position> {
        Position::new(line, column)
    }

    #[test]
    fn a_file_of_one_mebibyte_passes_and_one_more_byte_fails() {
        let at_the_cap = vec![b'a'; MAX_BYTES];
        assert!(YamlText::from_bytes(&at_the_cap).is_ok());
        let past_the_cap = vec![b'a'; MAX_BYTES + 1];
        assert_eq!(kind(&past_the_cap), Some(YamlErrorKind::TooLarge));
    }

    #[test]
    fn invalid_utf8_is_refused_with_the_position_of_the_first_bad_byte() {
        let error = YamlText::from_bytes(b"a: b\nc: \xff\n").expect_err("invalid UTF-8");
        assert_eq!(error.kind(), &YamlErrorKind::InvalidUtf8);
        assert_eq!(error.position(), at(2, 4));
        let first = YamlText::from_bytes(b"\xc3").expect_err("truncated sequence");
        assert_eq!(first.position(), at(1, 1));
    }

    #[test]
    fn tab_line_feed_and_carriage_return_are_allowed_in_raw_text() {
        let text = YamlText::from_bytes(b"a:\t1\r\nb: 2\rc: 3\n").expect("allowed");
        assert_eq!(text.as_str(), "a:\t1\r\nb: 2\rc: 3\n");
    }

    #[test]
    fn every_other_control_character_is_refused_in_raw_text() {
        let refused = (0_u32..=0x9F)
            .chain([0x2028, 0x2029])
            .filter_map(char::from_u32)
            .filter(|c| !matches!(c, '\t' | '\n' | '\r') && (c.is_control() || *c >= '\u{2028}'))
            .collect::<Vec<_>>();
        // 29 controls below U+0020 (32 less tab, LF, CR), DEL and U+0080 to U+009F (33), LS and PS.
        assert_eq!(refused.len(), 29 + 33 + 2);
        for character in refused {
            let text = format!("a: x{character}y");
            assert_eq!(
                kind(text.as_bytes()),
                Some(YamlErrorKind::ControlCharacter),
                "U+{:04X}",
                u32::from(character)
            );
        }
    }

    #[test]
    fn the_next_line_character_is_refused_and_the_error_has_its_position() {
        let error = YamlText::from_bytes("a: 1\nb: \u{85}\n".as_bytes()).expect_err("NEL");
        assert_eq!(error.position(), at(2, 4));
    }

    #[test]
    fn lines_end_at_cr_lf_and_cr_lf_together() {
        assert_eq!(locate("a\r\nb\rc\nd", 7), at(4, 1).unwrap());
    }

    #[test]
    fn format_characters_are_not_a_rule_of_yaml() {
        assert!(YamlText::from_bytes("a: \u{200d}\u{202e}".as_bytes()).is_ok());
    }
}
