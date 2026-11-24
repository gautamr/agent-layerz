//! The process environment, read once (spec §6 "Git subprocesses", TASK-63).

use std::ffi::OsStr;

use alz_core::env::EnvVars;
use alz_core::path::AbsolutePath;

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

    /// `XDG_CONFIG_HOME`.
    #[must_use]
    pub fn xdg_config_home(&self) -> Option<&OsStr> {
        self.0.get("XDG_CONFIG_HOME")
    }

    /// `XDG_STATE_HOME`.
    #[must_use]
    pub fn xdg_state_home(&self) -> Option<&OsStr> {
        self.0.get("XDG_STATE_HOME")
    }

    /// Whether mise or direnv is active: `__MISE_DIFF` or `DIRENV_DIFF` exists, even empty. Only
    /// the presence is asked, never the value: each holds every variable the manager set,
    /// secrets included (decision-17).
    #[must_use]
    pub fn mise_or_direnv_active(&self) -> bool {
        ["__MISE_DIFF", "DIRENV_DIFF"]
            .into_iter()
            .any(|name| self.0.get(name).is_some())
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

/// The working directory of the process, taken once at the start of the command together with the
/// environment and passed down, so that every part of a run agrees on where it is. `None` when it
/// cannot be found or is not a path that reads as text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkingDirectory(Option<AbsolutePath>);

impl WorkingDirectory {
    /// The working directory of this process, as it is now.
    #[must_use]
    pub fn capture() -> Self {
        Self::capture_with(std::env::current_dir)
    }

    /// The working directory `lookup` gives. A run asks once, here, and passes the value down;
    /// a test counts the calls through a `lookup` of its own.
    #[must_use]
    pub fn capture_with(lookup: impl FnOnce() -> std::io::Result<std::path::PathBuf>) -> Self {
        Self(lookup().ok().and_then(|directory| {
            directory
                .to_str()
                .and_then(|text| AbsolutePath::try_from(text).ok())
        }))
    }

    /// The directory of a caller that already holds it.
    #[must_use]
    pub const fn at(directory: AbsolutePath) -> Self {
        Self(Some(directory))
    }

    /// The directory, when it is known.
    #[must_use]
    pub const fn path(&self) -> Option<&AbsolutePath> {
        self.0.as_ref()
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
    fn the_working_directory_is_captured_as_an_absolute_path() {
        assert!(super::WorkingDirectory::capture().path().is_some());
    }

    #[test]
    fn capture_holds_the_variables_of_this_process() {
        // PATH is set under cargo and nextest; the test never mutates the environment.
        assert!(EnvSnapshot::capture().vars().get("PATH").is_some());
    }
}
