//! Where git read a config value, from `git config --show-scope --show-origin --get <key>`
//! (spec §9.4). The line is `<scope>\t<origin>\t<value>`; the value is dropped as the line is
//! parsed, so no code that holds a [`ConfigOrigin`] can print what the config says.

use crate::untrusted_text::Stripped;

/// The scope git reports for a config value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitScope {
    /// The system config file.
    System,
    /// The user's config file.
    Global,
    /// The repository's `.git/config`.
    Local,
    /// The work tree's config file.
    Worktree,
    /// `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS` or `git -c`.
    Command,
    /// A scope this version of alz does not know, or no scope at all.
    Unknown,
}

impl GitScope {
    /// The word a report uses.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Global => "global",
            Self::Local => "local",
            Self::Worktree => "worktree",
            Self::Command => "command",
            Self::Unknown => "unknown",
        }
    }

    fn parse(word: &str) -> Self {
        match word {
            "system" => Self::System,
            "global" => Self::Global,
            "local" => Self::Local,
            "worktree" => Self::Worktree,
            "command" => Self::Command,
            _ => Self::Unknown,
        }
    }
}

/// The scope and the source of one config value, without the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigOrigin {
    scope: GitScope,
    origin: Stripped,
}

impl ConfigOrigin {
    /// Parses the standard output of `git config --show-scope --show-origin --get <key>`. Total:
    /// any bytes give a value, invalid UTF-8 is replaced, only the first line and its first two
    /// tab-separated fields are read, and the third, the value, is never looked at. A missing
    /// scope is [`GitScope::Unknown`]. The origin is stripped of control and format characters
    /// and loses a leading `file:`.
    #[must_use]
    pub fn parse(output: &[u8]) -> Self {
        let text = String::from_utf8_lossy(output);
        let mut fields = text.lines().next().unwrap_or_default().split('\t');
        let scope = GitScope::parse(fields.next().unwrap_or_default());
        let origin = Stripped::from_untrusted(fields.next().unwrap_or_default());
        let origin = origin
            .as_str()
            .strip_prefix("file:")
            .map_or(origin.clone(), Stripped::from_untrusted);
        Self { scope, origin }
    }

    /// The scope.
    #[must_use]
    pub const fn scope(&self) -> GitScope {
        self.scope
    }

    /// The origin: a file path, `command line:` and the like.
    #[must_use]
    pub const fn origin(&self) -> &Stripped {
        &self.origin
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert, proptest};

    use super::{ConfigOrigin, GitScope};

    #[test]
    fn reads_scope_and_origin_and_drops_the_value() {
        let origin = ConfigOrigin::parse(b"global\tfile:/home/a/.gitconfig\tSecret Name\n");
        assert_eq!(origin.scope(), GitScope::Global);
        assert_eq!(origin.origin().as_str(), "/home/a/.gitconfig");
    }

    #[test]
    fn knows_every_scope_git_reports() {
        for (word, scope) in [
            ("system", GitScope::System),
            ("global", GitScope::Global),
            ("local", GitScope::Local),
            ("worktree", GitScope::Worktree),
            ("command", GitScope::Command),
            ("future", GitScope::Unknown),
            ("", GitScope::Unknown),
        ] {
            let line = format!("{word}\tfile:x\tv");
            assert_eq!(ConfigOrigin::parse(line.as_bytes()).scope(), scope);
        }
    }

    #[test]
    fn the_origin_of_the_command_scope_is_kept_as_git_prints_it() {
        let origin = ConfigOrigin::parse(b"command\tcommand line:\tv");
        assert_eq!(origin.origin().as_str(), "command line:");
    }

    #[test]
    fn control_characters_are_removed_from_the_origin() {
        let origin = ConfigOrigin::parse(b"local\tfile:/r/\x1b[31m.git/config\xe2\x80\xae\tv");
        assert_eq!(origin.origin().as_str(), "/r/[31m.git/config");
    }

    proptest! {
        #[test]
        fn parsing_never_panics_and_never_keeps_the_third_field(bytes in any::<Vec<u8>>()) {
            let origin = ConfigOrigin::parse(&bytes);
            prop_assert!(origin.origin().as_str().chars().all(|c| !c.is_control()));
        }

        #[test]
        fn the_value_never_appears_in_the_parsed_origin(value in "[0-9]{12}") {
            let line = format!("local\tfile:/r/.git/config\t{value}");
            let origin = ConfigOrigin::parse(line.as_bytes());
            let shown = format!("{origin:?}");
            prop_assert!(!shown.contains(&value));
        }
    }
}
