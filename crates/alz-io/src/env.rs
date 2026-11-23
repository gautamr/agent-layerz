//! The process environment, read once (spec §6 "Git subprocesses", TASK-63).

use std::ffi::OsStr;

use alz_core::env::EnvVars;

/// The environment of the process, taken once at the start of the command and passed down.
/// This is the only reader of the process environment, and nothing in alz changes it.
///
/// The queries return the raw value: an empty or relative value is for the caller's rule to
/// judge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot(EnvVars);

impl EnvSnapshot {
    /// The environment of this process, as it is now.
    #[must_use]
    #[expect(
        clippy::disallowed_methods,
        reason = "the one place that reads the process environment (TASK-63)"
    )]
    pub fn capture() -> Self {
        Self(std::env::vars_os().collect())
    }

    /// A snapshot of the given variables, for a caller that holds an environment as a value.
    #[must_use]
    pub const fn from_vars(vars: EnvVars) -> Self {
        Self(vars)
    }

    /// `HOME`.
    #[must_use]
    pub fn home(&self) -> Option<&OsStr> {
        self.0.get("HOME")
    }

    /// `XDG_CACHE_HOME`.
    #[must_use]
    pub fn xdg_cache_home(&self) -> Option<&OsStr> {
        self.0.get("XDG_CACHE_HOME")
    }

    /// `CLAUDE_CONFIG_DIR`.
    #[must_use]
    pub fn claude_config_dir(&self) -> Option<&OsStr> {
        self.0.get("CLAUDE_CONFIG_DIR")
    }

    /// `CODEX_HOME`.
    #[must_use]
    pub fn codex_home(&self) -> Option<&OsStr> {
        self.0.get("CODEX_HOME")
    }

    /// Every variable, to be filtered into the environment of a child process.
    #[must_use]
    pub const fn vars(&self) -> &EnvVars {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use alz_core::env::EnvVars;

    use super::EnvSnapshot;

    fn snapshot(pairs: &[(&str, &str)]) -> EnvSnapshot {
        EnvSnapshot::from_vars(
            pairs
                .iter()
                .map(|(name, value)| (OsString::from(name), OsString::from(value)))
                .collect::<EnvVars>(),
        )
    }

    #[test]
    fn the_typed_queries_read_their_variables() {
        let env = snapshot(&[
            ("HOME", "/home/me"),
            ("XDG_CACHE_HOME", ""),
            ("CLAUDE_CONFIG_DIR", "/c"),
            ("CODEX_HOME", "/x"),
        ]);
        assert_eq!(env.home(), Some(OsStr::new("/home/me")));
        assert_eq!(env.xdg_cache_home(), Some(OsStr::new("")));
        assert_eq!(env.claude_config_dir(), Some(OsStr::new("/c")));
        assert_eq!(env.codex_home(), Some(OsStr::new("/x")));
    }

    #[test]
    fn a_variable_that_is_not_set_is_none() {
        let env = snapshot(&[]);
        assert_eq!(env.home(), None);
        assert_eq!(env.codex_home(), None);
    }

    #[test]
    fn capture_holds_the_variables_of_this_process() {
        // PATH is set under cargo and nextest; the test never mutates the environment.
        assert!(EnvSnapshot::capture().vars().get("PATH").is_some());
    }
}
