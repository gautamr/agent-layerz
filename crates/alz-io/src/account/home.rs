//! `$HOME` for the account files (spec §7.13 "Environment"; TASK-72, owner decision 3 of R6).

use std::path::{Path, PathBuf};

use alz_core::path::AbsolutePath;

use crate::EnvSnapshot;

/// The user's home folder as the account files need it: an absolute path, or the fact that there
/// is none.
///
/// `HOME` counts as it is given: an empty or relative value is not an absolute path, and the user
/// database is not asked then. Only when `HOME` is not set at all is the home folder looked up
/// there, by the real user id (`std::env::home_dir`); a user with no entry has no home folder
/// either. Without one, both account files are ignored with one warning and no account folder is
/// set, so a named account fails closed (TASK-72).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Home {
    /// An absolute path, as given. It is not resolved: a link in it stays.
    Absolute(AbsolutePath),
    /// `HOME` is empty, relative or not text, or it is unset and the user database has no
    /// absolute path for the user.
    NotAbsolute,
}

impl Home {
    /// The home folder of this process: `HOME` of `env`, else the user database's.
    #[must_use]
    pub fn locate(env: &EnvSnapshot) -> Self {
        Self::locate_with(env, std::env::home_dir)
    }

    /// As [`locate`](Self::locate), with the user database lookup injected. `lookup` is called
    /// only when `HOME` is not set.
    #[must_use]
    pub fn locate_with(env: &EnvSnapshot, lookup: impl FnOnce() -> Option<PathBuf>) -> Self {
        env.home()
            .map(Path::new)
            .map(Path::to_path_buf)
            .or_else(lookup)
            .as_deref()
            .and_then(absolute)
            .map_or(Self::NotAbsolute, Self::Absolute)
    }

    /// The home folder, when there is one.
    #[must_use]
    pub const fn path(&self) -> Option<&AbsolutePath> {
        match self {
            Self::Absolute(path) => Some(path),
            Self::NotAbsolute => None,
        }
    }
}

fn absolute(path: &Path) -> Option<AbsolutePath> {
    path.to_str()
        .and_then(|text| AbsolutePath::try_from(text).ok())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use alz_core::env::EnvVars;

    use super::Home;
    use crate::EnvSnapshot;

    fn env(home: Option<&str>) -> EnvSnapshot {
        EnvSnapshot::from_vars(
            home.map(|value| (OsString::from("HOME"), OsString::from(value)))
                .into_iter()
                .collect::<EnvVars>(),
        )
    }

    fn entry(path: &str) -> impl FnOnce() -> Option<PathBuf> {
        let path = PathBuf::from(path);
        move || Some(path)
    }

    fn never() -> Option<PathBuf> {
        panic!("the user database is only asked when HOME is not set")
    }

    #[test]
    fn an_absolute_home_is_used_as_given_and_the_user_database_is_not_asked() {
        let home = Home::locate_with(&env(Some("/home/me/")), never);
        assert_eq!(
            home.path().map(ToString::to_string).as_deref(),
            Some("/home/me/")
        );
    }

    #[test]
    fn an_empty_or_relative_home_is_not_an_absolute_path_and_the_user_database_is_not_asked() {
        for given in ["", "home/me", "~", "./x"] {
            assert_eq!(
                Home::locate_with(&env(Some(given)), never),
                Home::NotAbsolute,
                "{given:?}"
            );
        }
    }

    #[test]
    fn an_unset_home_comes_from_the_user_database() {
        let home = Home::locate_with(&env(None), entry("/home/db"));
        assert_eq!(
            home.path().map(ToString::to_string).as_deref(),
            Some("/home/db")
        );
    }

    #[test]
    fn an_unset_home_with_no_entry_or_a_relative_one_is_not_an_absolute_path() {
        assert_eq!(Home::locate_with(&env(None), || None), Home::NotAbsolute);
        assert_eq!(
            Home::locate_with(&env(None), entry("relative")),
            Home::NotAbsolute
        );
    }
}
