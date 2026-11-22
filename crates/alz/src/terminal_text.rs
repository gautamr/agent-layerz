//! Untrusted text reaches a terminal only after its control and format characters are removed
//! (decision-26), so an argument cannot move the cursor, recolour the screen or reorder text.

/// Whether a character may be printed: line feeds and every character that is neither a control
/// nor a format character.
fn is_printable(character: char) -> bool {
    character == '\n' || !(character.is_control() || is_format(character))
}

/// The Unicode format characters (category Cf) that change how surrounding text is displayed.
const fn is_format(character: char) -> bool {
    matches!(
        character,
        '\u{ad}'
            | '\u{600}'..='\u{605}'
            | '\u{61c}'
            | '\u{6dd}'
            | '\u{70f}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0001}'
            | '\u{e0020}'..='\u{e007f}'
    )
}

/// The text with every control and format character removed, except line feeds.
pub fn printable(text: &str) -> String {
    text.chars()
        .filter(|character| is_printable(*character))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::printable;

    #[test]
    fn keeps_ordinary_text_and_line_feeds() {
        assert_eq!(
            printable("error: bad flag\nUsage: alz\n"),
            "error: bad flag\nUsage: alz\n"
        );
    }

    #[test]
    fn removes_escape_sequences_carriage_returns_and_bidirectional_controls() {
        assert_eq!(
            printable("a\u{1b}[31mb\rc\u{202e}d\u{2066}e\u{7}"),
            "a[31mbcde"
        );
    }

    #[test]
    fn keeps_letters_of_other_scripts() {
        assert_eq!(printable("Größe 日本語"), "Größe 日本語");
    }
}
