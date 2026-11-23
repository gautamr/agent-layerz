//! Where the layer cache root is (spec §6 "The layer cache", TASK-53). This is the one function
//! that every command resolving the stack, and account resolution (§7.13), use, so they never
//! disagree about the folder.

use thiserror::Error;

use crate::path::AbsolutePath;

/// The folder under the cache base that alz owns.
const FOLDER: &str = "agent-layerz";

/// The cache root as a path: `$XDG_CACHE_HOME/agent-layerz` or `$HOME/.cache/agent-layerz`. The
/// path is as written, not resolved: a link in it is followed later, one component at a time
/// (`RealPathWalk`), and the result is checked against the account folders and the repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheLocation(AbsolutePath);

/// Neither `XDG_CACHE_HOME` nor `HOME` is an absolute path. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("cannot locate the layer cache: set HOME or XDG_CACHE_HOME to an absolute path")]
pub struct CacheLocationError;

impl CacheLocation {
    /// The cache root for the two variables, each as the environment holds it (`None` when it is
    /// unset).
    ///
    /// - An absolute `XDG_CACHE_HOME` gives `<value>/agent-layerz`. An empty or relative one counts
    ///   as unset, as the XDG Base Directory specification says.
    /// - Otherwise an absolute `home` gives `<home>/.cache/agent-layerz`. `home` is `HOME` as given
    ///   (§7.13 "Environment"): the caller passes the user database's entry for an unset `HOME`, and
    ///   `None` when there is none.
    ///
    /// # Errors
    ///
    /// [`CacheLocationError`] when neither value is an absolute path.
    pub fn from_env(
        home: Option<&str>,
        xdg_cache_home: Option<&str>,
    ) -> Result<Self, CacheLocationError> {
        let base = xdg_cache_home
            .filter(|value| is_absolute(value))
            .map(str::to_owned)
            .or_else(|| {
                home.filter(|value| is_absolute(value))
                    .map(|home| format!("{}/.cache", home.trim_end_matches('/')))
            })
            .ok_or(CacheLocationError)?;
        AbsolutePath::try_from(format!("{}/{FOLDER}", base.trim_end_matches('/')).as_str())
            .map(Self)
            .map_err(|_not_absolute| CacheLocationError)
    }

    /// The cache root.
    #[must_use]
    pub const fn path(&self) -> &AbsolutePath {
        &self.0
    }
}

fn is_absolute(text: &str) -> bool {
    AbsolutePath::try_from(text).is_ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{prop_assert_eq, proptest};

    use super::{CacheLocation, CacheLocationError};

    fn root(home: Option<&str>, xdg: Option<&str>) -> String {
        CacheLocation::from_env(home, xdg)
            .expect("a cache root")
            .path()
            .to_string()
    }

    #[test]
    fn an_unset_empty_or_relative_xdg_cache_home_gives_the_home_default() {
        for xdg in [None, Some(""), Some("cache"), Some("./cache"), Some("~/c")] {
            assert_eq!(
                root(Some("/home/u"), xdg),
                "/home/u/.cache/agent-layerz",
                "{xdg:?}"
            );
        }
    }

    #[test]
    fn an_absolute_xdg_cache_home_gives_its_own_folder() {
        assert_eq!(
            root(Some("/home/u"), Some("/var/cache")),
            "/var/cache/agent-layerz"
        );
        assert_eq!(root(None, Some("/var/cache/")), "/var/cache/agent-layerz");
        assert_eq!(root(Some("/h"), Some("/")), "/agent-layerz");
    }

    #[test]
    fn a_trailing_slash_of_home_does_not_double_the_separator() {
        assert_eq!(root(Some("/home/u/"), None), "/home/u/.cache/agent-layerz");
    }

    #[test]
    fn the_cache_cannot_be_located_without_an_absolute_variable() {
        for (home, xdg) in [
            (None, None),
            (Some(""), None),
            (Some("home/u"), Some("")),
            (Some("~"), Some("relative")),
            (None, Some("")),
        ] {
            assert_eq!(
                CacheLocation::from_env(home, xdg),
                Err(CacheLocationError),
                "{home:?} {xdg:?}"
            );
        }
        assert_eq!(
            CacheLocationError.to_string(),
            "cannot locate the layer cache: set HOME or XDG_CACHE_HOME to an absolute path"
        );
    }

    proptest! {
        /// TASK-53 AC #3: an empty or relative value is the same as an unset one, whatever
        /// `HOME` is, so lint, build and account resolution agree.
        #[test]
        fn a_value_that_is_not_absolute_is_the_same_as_an_unset_one(
            home in proptest::option::of("[/a-z.~ ]{0,12}"),
            xdg in "[a-z.~ ][/a-z.~ ]{0,12}|",
        ) {
            prop_assert_eq!(
                CacheLocation::from_env(home.as_deref(), Some(&xdg)),
                CacheLocation::from_env(home.as_deref(), None)
            );
        }
    }
}
