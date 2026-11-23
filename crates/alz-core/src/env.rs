//! The environment variables of a process, as a value (spec §6 "Git subprocesses").
//!
//! Only `alz-io` reads the process environment; everything else receives it as an [`EnvVars`].

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;

/// An ordered map from variable name to value. Names and values are not text, so they are
/// `OsString`s and pass through byte for byte. Iteration is in name order.
///
/// The `Debug` text lists the names only: a value may hold a token.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct EnvVars(BTreeMap<OsString, OsString>);

impl EnvVars {
    /// The value of `name`, if the variable is set. An empty value is a value.
    #[must_use]
    pub fn get(&self, name: impl AsRef<OsStr>) -> Option<&OsStr> {
        self.0.get(name.as_ref()).map(OsString::as_os_str)
    }

    /// The variables in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> + '_ {
        self.0
            .iter()
            .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
    }

    /// The number of variables.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no variable is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// These variables with `name` set to `value`, replacing any earlier value.
    #[must_use]
    pub fn with(self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.into_iter()
            .chain([(name.into(), value.into())])
            .collect()
    }

    /// These variables without those whose name does not satisfy `keep`.
    #[must_use]
    pub fn keeping(self, keep: impl Fn(&OsStr) -> bool) -> Self {
        self.into_iter().filter(|(name, _)| keep(name)).collect()
    }
}

impl FromIterator<(OsString, OsString)> for EnvVars {
    fn from_iter<I: IntoIterator<Item = (OsString, OsString)>>(pairs: I) -> Self {
        Self(pairs.into_iter().collect())
    }
}

impl IntoIterator for EnvVars {
    type Item = (OsString, OsString);
    type IntoIter = std::collections::btree_map::IntoIter<OsString, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Debug for EnvVars {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_set().entries(self.0.keys()).finish()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use super::EnvVars;

    fn vars(pairs: &[(&str, &str)]) -> EnvVars {
        pairs
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect()
    }

    #[test]
    fn with_replaces_an_earlier_value() {
        let result = vars(&[("A", "1")]).with("A", "2").with("B", "3");
        assert_eq!(result, vars(&[("A", "2"), ("B", "3")]));
    }

    #[test]
    fn an_empty_value_is_set() {
        let result = vars(&[("A", "")]);
        assert_eq!(result.get("A").map(OsStr::is_empty), Some(true));
        assert_eq!(result.get("B"), None);
    }

    #[test]
    fn keeping_drops_the_names_that_fail() {
        let result = vars(&[("A", "1"), ("B", "2")]).keeping(|name| name == "A");
        assert_eq!(result, vars(&[("A", "1")]));
    }

    #[test]
    fn the_debug_text_holds_names_and_never_values() {
        let text = format!("{:?}", vars(&[("TOKEN", "hunter2")]));
        assert!(text.contains("TOKEN"));
        assert!(!text.contains("hunter2"));
    }
}
