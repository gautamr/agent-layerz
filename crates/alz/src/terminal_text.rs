//! Class 3 of the untrusted text classes (decision-26): clap's rendered output, the only output
//! that keeps its own line feeds. Classes 1 and 2 are types of `alz-core`; every fragment of an
//! argument that clap echoes is stripped with [`Stripped`], so no line feed, carriage return,
//! escape or format character from an argument reaches the terminal.

use alz_core::Stripped;

/// Text from an untrusted source, safe to print: no control, line-separator or format character,
/// line feeds included, so the text stays on its own line.
pub fn strip_untrusted(text: &str) -> String {
    Stripped::from_untrusted(text).into_string()
}

/// Text that holds trusted line structure, with every other untrusted character removed. Only
/// for text whose untrusted fragments have already gone through [`strip_untrusted`]: this is the
/// last guard, not the sanitiser.
pub fn strip_untrusted_keeping_line_feeds(text: &str) -> String {
    text.split('\n')
        .map(strip_untrusted)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Text of a file that is shown as it is laid out: line feeds and tabs stay, because neither can
/// hide text or move the cursor off its line; a carriage return, an escape, a bidirectional
/// control, every other control character and every format character go. Like
/// [`strip_untrusted_keeping_line_feeds`], this is the last guard.
pub fn strip_untrusted_keeping_layout(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            line.split('\t')
                .map(strip_untrusted)
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        strip_untrusted, strip_untrusted_keeping_layout, strip_untrusted_keeping_line_feeds,
    };

    #[test]
    fn file_text_keeps_line_feeds_and_tabs_and_loses_what_moves_the_cursor_or_hides_text() {
        assert_eq!(
            strip_untrusted_keeping_layout("a\tb\r\n\u{1b}[2Jc\t\u{202e}d\u{200b}e\u{2028}f\u{7}"),
            "a\tb\n[2Jc\tdef"
        );
    }

    #[test]
    fn strips_an_argument_fragment_to_one_line() {
        assert_eq!(strip_untrusted("a\nb\u{1b}c\u{202e}d"), "abcd");
    }

    #[test]
    fn the_last_guard_keeps_line_feeds_and_removes_everything_else_untrusted() {
        assert_eq!(
            strip_untrusted_keeping_line_feeds("a\nb\u{1b}c\u{2028}d\u{202e}e"),
            "a\nbcde"
        );
    }
}
