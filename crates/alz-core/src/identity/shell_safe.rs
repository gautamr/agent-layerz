//! A value that a pasted `git config` command may carry in any common shell (spec §9.4): POSIX
//! shells, fish and PowerShell read it the same way, inside single quotes or not. It is the only
//! way to a printed fix command: [`FixCommand`] takes a [`ShellSafe`] and nothing else.

use unicode_normalization::char::is_combining_mark;

use crate::identity::slot::ConfigKey;

/// Text made only of Unicode letters, marks and digits, the space, and `.`, `-`, `_`, `@`, `+`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSafe(String);

/// Why a text is not shell-safe. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the value holds a character a shell may read specially")]
pub struct NotShellSafe;

/// Symbols (category So) that the Alphabetic property includes: circled and parenthesised
/// letters. They are no letters, so they are not safe.
const ALPHABETIC_SYMBOLS: [std::ops::RangeInclusive<char>; 4] = [
    '\u{24B6}'..='\u{24E9}',
    '\u{1F130}'..='\u{1F149}',
    '\u{1F150}'..='\u{1F169}',
    '\u{1F170}'..='\u{1F189}',
];

fn is_safe(character: char) -> bool {
    let letter = character.is_alphabetic()
        && !ALPHABETIC_SYMBOLS
            .iter()
            .any(|range| range.contains(&character));
    letter
        || is_combining_mark(character)
        || character.is_numeric()
        || matches!(character, ' ' | '.' | '-' | '_' | '@' | '+')
}

impl TryFrom<&str> for ShellSafe {
    type Error = NotShellSafe;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        (!text.is_empty() && text.chars().all(is_safe))
            .then(|| Self(text.to_owned()))
            .ok_or(NotShellSafe)
    }
}

impl ShellSafe {
    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which config file a fix command sets the key in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixScope {
    /// `--local`: the repository's `.git/config`.
    Local,
    /// `--worktree`: the work tree's config file.
    Worktree,
}

/// `git config --local user.email 'owner@example.com'`: built from a key alz names and a
/// [`ShellSafe`] value, so it means the same in every shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCommand {
    scope: FixScope,
    key: ConfigKey,
    value: ShellSafe,
}

impl FixCommand {
    /// A command that sets `key` to `value` in `scope`.
    #[must_use]
    pub const fn new(scope: FixScope, key: ConfigKey, value: ShellSafe) -> Self {
        Self { scope, key, value }
    }

    /// The command line.
    #[must_use]
    pub fn text(&self) -> String {
        let scope = match self.scope {
            FixScope::Local => "--local",
            FixScope::Worktree => "--worktree",
        };
        format!(
            "git config {scope} {} '{}'",
            self.key.as_str(),
            self.value.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

    use super::{FixCommand, FixScope, ShellSafe};
    use crate::identity::slot::Field;

    /// Characters a POSIX shell, fish or PowerShell reads specially, or that end a command.
    const HOSTILE: &str = "'\"`$\\!&|;<>(){}[]*?#~%^=,:/\t\n\r\u{0}\u{7f}";

    #[test]
    fn plain_names_and_emails_are_safe() {
        for text in [
            "Owner Example",
            "owner@client.example",
            "a-b_c+d",
            "Zoë Ångström",
            "李 小龙",
            "e\u{301}",
        ] {
            assert!(ShellSafe::try_from(text).is_ok(), "{text:?}");
        }
    }

    #[test]
    fn what_a_shell_reads_specially_is_not_safe() {
        for text in [
            "",
            "O'Example",
            "a b!",
            "$(x)",
            "a;b",
            "a\nb",
            "a`b",
            "a\\b",
            "a\"b",
            "x/y",
            "ⓐ",
            "a\u{200b}b",
            "a\u{202e}b",
        ] {
            assert!(ShellSafe::try_from(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_fix_command_quotes_the_value() {
        let value = ShellSafe::try_from("owner@example.com").unwrap();
        let command = FixCommand::new(FixScope::Local, Field::Email.user_key(), value.clone());
        assert_eq!(
            command.text(),
            "git config --local user.email 'owner@example.com'"
        );
        let command = FixCommand::new(FixScope::Worktree, Field::Email.user_key(), value);
        assert_eq!(
            command.text(),
            "git config --worktree user.email 'owner@example.com'"
        );
    }

    proptest! {
        #[test]
        fn a_safe_value_never_holds_a_character_a_shell_would_read(text in any::<String>()) {
            if let Ok(safe) = ShellSafe::try_from(text.as_str()) {
                prop_assert!(safe.as_str().chars().all(|c| !HOSTILE.contains(c) && !c.is_control()));
                prop_assert_eq!(safe.as_str(), text.as_str());
            }
        }

        #[test]
        fn one_hostile_character_anywhere_refuses_the_value(
            head in "[a-z ]{0,6}",
            hostile in proptest::sample::select(HOSTILE.chars().collect::<Vec<_>>()),
            tail in "[a-z ]{0,6}",
        ) {
            let text = format!("{head}{hostile}{tail}");
            prop_assert!(ShellSafe::try_from(text.as_str()).is_err());
        }
    }
}
