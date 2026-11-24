//! The variables no layer may set (spec §7.9) and the Codex pattern match that decides which of
//! them a `shell_environment_policy` pattern reaches. Pure: the checks of settings, MCP files and
//! passthrough files that use it are R4's.

/// The variables that choose the account folder or locate alz's own files and the layer cache.
pub const PROTECTED_VARIABLES: [&str; 6] = [
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
];

/// The protected variable that `name` stands for, ignoring ASCII case (§7.9). The result is one
/// of the fixed names, never the text that was given, so it can be printed.
#[must_use]
pub fn protected_variable(name: &str) -> Option<&'static str> {
    PROTECTED_VARIABLES
        .into_iter()
        .find(|protected| protected.eq_ignore_ascii_case(name))
}

/// Whether `name` is a protected variable, ignoring ASCII case (§7.9).
#[must_use]
pub fn is_protected_name(name: &str) -> bool {
    protected_variable(name).is_some()
}

/// The protected variables that `pattern` matches as Codex matches its policy patterns.
pub fn matched_protected_names(pattern: &str) -> impl Iterator<Item = &'static str> + '_ {
    PROTECTED_VARIABLES
        .into_iter()
        .filter(|name| codex_pattern_matches(pattern, name))
}

/// Whether `pattern` matches `name` as Codex builds its policy patterns (`docs/target-facts.md`
/// §12 item 15, wildmatch 2.6.1): `*` matches any run of characters, `?` one character, there are
/// no classes and no escapes, and two characters are equal when they are equal or their Unicode
/// lower-case forms are.
#[must_use]
pub fn codex_pattern_matches(pattern: &str, name: &str) -> bool {
    let name: Vec<char> = name.chars().collect();
    // `reach[j]` says that the pattern read so far matches the first `j` characters of `name`.
    let start: Vec<bool> = (0..=name.len()).map(|length| length == 0).collect();
    let reach = pattern.chars().fold(start, |reach, token| {
        if token == '*' {
            reach
                .iter()
                .scan(false, |seen, &reached| {
                    *seen = *seen || reached;
                    Some(*seen)
                })
                .collect()
        } else {
            let after_one = name.iter().enumerate().map(|(index, &character)| {
                reach.get(index).copied().unwrap_or(false)
                    && (token == '?' || same_character(token, character))
            });
            std::iter::once(false).chain(after_one).collect()
        }
    });
    reach.last().copied().unwrap_or(false)
}

fn same_character(left: char, right: char) -> bool {
    left == right || left.to_lowercase().eq(right.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{codex_pattern_matches, is_protected_name, matched_protected_names};

    #[test]
    fn the_codex_pattern_cases() {
        let cases = [
            ("*", "CODEX_HOME", true),
            ("?ODEX_HOME", "CODEX_HOME", true),
            ("*home*", "CODEX_HOME", true),
            ("XDG_*", "XDG_STATE_HOME", true),
            ("*_HOME", "CODEX_HOME", true),
            ("codex_home", "CODEX_HOME", true),
            ("HOME", "HOME", true),
            ("H?ME", "HOME", true),
            ("*FOO*", "CODEX_HOME", false),
            ("?HOME", "HOME", false),
            ("HOM", "HOME", false),
            ("HOMES", "HOME", false),
            ("XDG_*", "HOME", false),
            ("", "HOME", false),
            ("*", "", true),
        ];
        assert_eq!(cases.len(), 15);
        for (pattern, name, expected) in cases {
            assert_eq!(
                codex_pattern_matches(pattern, name),
                expected,
                "pattern {pattern:?} against {name:?}"
            );
        }
    }

    #[test]
    fn protected_names_compare_ignoring_ascii_case_only() {
        assert!(is_protected_name("codex_home"));
        assert!(is_protected_name("Xdg_State_Home"));
        assert!(!is_protected_name("CODEX_HOME2"));
        assert!(!is_protected_name("HOME\u{212a}"));
    }

    #[test]
    fn a_protected_name_is_given_back_in_its_fixed_spelling() {
        assert_eq!(super::protected_variable("codex_home"), Some("CODEX_HOME"));
        assert_eq!(
            super::protected_variable("Xdg_Cache_Home"),
            Some("XDG_CACHE_HOME")
        );
        assert_eq!(super::protected_variable("PATH"), None);
    }

    #[test]
    fn a_pattern_reaches_the_protected_names_it_matches() {
        let reached: Vec<&str> = matched_protected_names("XDG_*").collect();
        assert_eq!(
            reached,
            ["XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"]
        );
        assert_eq!(matched_protected_names("PATH").count(), 0);
    }
}
