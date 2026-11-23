//! The text rule for a `.toml` passthrough file (spec §7.9). alz has no TOML parser, so a file is
//! refused by what its text says. The rule is stricter than the checks of the same variables in
//! JSON and YAML, and a parser may relax it later.
//!
//! A file is refused when it mentions `shell_environment_policy` or one of the protected variables
//! as a whole name, ignoring ASCII case, or when it has a `\u`, `\U` or `\x` escape or a backslash
//! at the end of a line: those can spell or join a name that a text search cannot see.

use crate::account::PROTECTED_VARIABLES;

/// The key that builds the environment of Codex's shell commands.
pub(super) const POLICY_KEY: &str = "shell_environment_policy";

/// Whether the text mentions `shell_environment_policy`, in any ASCII case.
pub(super) fn mentions_policy(text: &str) -> bool {
    text.to_ascii_lowercase().contains(POLICY_KEY)
}

/// The protected variables the text mentions as a whole name: not preceded or followed by a
/// letter, digit, `_` or `-`, so `/home/user` is a mention and `homepage` is not. In the order of
/// [`PROTECTED_VARIABLES`], each once.
pub(super) fn named_variables(text: &str) -> impl Iterator<Item = &'static str> {
    let lower = text.to_ascii_lowercase();
    PROTECTED_VARIABLES
        .into_iter()
        .filter(move |name| mentions_whole_name(&lower, &name.to_ascii_lowercase()))
}

/// Whether `lower`, already in ASCII lower case, holds `name` with no word character at either side.
fn mentions_whole_name(lower: &str, name: &str) -> bool {
    let bytes = lower.as_bytes();
    lower.match_indices(name).any(|(start, found)| {
        let before = start.checked_sub(1).and_then(|index| bytes.get(index));
        let after = bytes.get(start.saturating_add(found.len()));
        !before.is_some_and(|byte| is_word_byte(*byte))
            && !after.is_some_and(|byte| is_word_byte(*byte))
    })
}

/// A character of a bare TOML key: a name inside a longer word is a different name.
const fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Whether the text has a `\u`, `\U` or `\x` escape, or a backslash that ends a line (followed by
/// spaces or tabs only), which can spell or join a name.
pub(super) fn hides_a_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\\')
        .any(|(index, _)| {
            let rest = bytes.get(index.saturating_add(1)..).unwrap_or_default();
            matches!(rest.first(), Some(b'u' | b'U' | b'x')) || ends_the_line(rest)
        })
}

/// Whether `rest` is spaces and tabs and then a line end, `\n` or `\r\n`.
fn ends_the_line(rest: &[u8]) -> bool {
    let mut after_blanks = rest
        .iter()
        .copied()
        .skip_while(|byte| matches!(byte, b' ' | b'\t'));
    match after_blanks.next() {
        Some(b'\n') => true,
        Some(b'\r') => after_blanks.next() == Some(b'\n'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{hides_a_name, mentions_policy, named_variables};

    fn named(text: &str) -> Vec<&'static str> {
        named_variables(text).collect()
    }

    #[test]
    fn a_whole_name_is_a_mention_in_any_case() {
        assert_eq!(named("HOME = 'x'"), ["HOME"]);
        assert_eq!(named("[env]\ncodex_home = 'x'"), ["CODEX_HOME"]);
        assert_eq!(named("path = \"/home/user\""), ["HOME"]);
        assert_eq!(named("a = Xdg_Cache_Home"), ["XDG_CACHE_HOME"]);
    }

    #[test]
    fn a_name_inside_a_longer_word_is_not_a_mention() {
        for text in [
            "url = \"https://example.invalid/homepage\"",
            "x = CODEX_HOME_2",
            "my-home = 1",
            "home_dir = 1",
            "ahome = 1",
            "model = 'x'",
        ] {
            assert_eq!(named(text), Vec::<&str>::new(), "{text}");
        }
    }

    #[test]
    fn a_longer_name_does_not_also_mention_the_home_inside_it() {
        assert_eq!(named("XDG_CONFIG_HOME = 1"), ["XDG_CONFIG_HOME"]);
    }

    #[test]
    fn several_names_are_each_listed_once_in_the_fixed_order() {
        assert_eq!(
            named("home home xdg_cache_home claude_config_dir"),
            ["CLAUDE_CONFIG_DIR", "HOME", "XDG_CACHE_HOME"]
        );
    }

    #[test]
    fn the_policy_key_is_found_in_any_case() {
        assert!(mentions_policy("[Shell_Environment_Policy.set]"));
        assert!(!mentions_policy("[shell_environment]"));
    }

    #[test]
    fn escapes_and_line_ending_backslashes_can_hide_a_name() {
        for text in [
            "\"\\u0048OME\" = 'x'",
            "\"\\U00000048OME\" = 'x'",
            "\"\\x48OME\" = 'x'",
            "x = \"\"\"CODEX_\\\nHOME\"\"\"",
            "x = \"\"\"CODEX_\\  \t\r\nHOME\"\"\"",
        ] {
            assert!(hides_a_name(text), "{text}");
        }
    }

    #[test]
    fn other_backslashes_hide_nothing() {
        for text in [
            "x = 'C:\\dir'",
            "x = \"a\\tb\"",
            "x = \"a\\\"b\"",
            "x = \\",
            "x = \\ y",
        ] {
            assert!(!hides_a_name(text), "{text}");
        }
    }
}
