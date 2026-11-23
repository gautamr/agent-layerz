//! An absolute path as the account files write it (spec §7.13 "User config", "Allow state"): one
//! string that starts with `/`. Whether it is its own real path is a file-system question that
//! belongs to the capability layer, so the type holds only what the text itself shows.

use std::fmt;

use thiserror::Error;

/// A path that starts with `/` and holds no NUL. It is not normalised: `/a/../b` is absolute and
/// stays as written, so a `root` that is not its own real path can still be told apart later.
///
/// `Ord` is `str`'s: the byte order of the UTF-8 text, which is the code point order of the
/// spec's name order (§6).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AbsolutePath(String);

/// Text that is not an absolute path. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not an absolute path: it must start with '/'")]
pub struct AbsolutePathError;

impl AbsolutePathError {
    /// The rule, as the end of a sentence about a value.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be an absolute path"
    }
}

impl AbsolutePath {
    /// The root, `/`.
    #[must_use]
    pub fn root() -> Self {
        Self("/".to_owned())
    }

    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `/`-separated parts of the path, as written: empty parts (from `//` or a trailing `/`)
    /// are left out, and `.` and `..` are returned as they are.
    pub fn components(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.split('/').filter(|part| !part.is_empty())
    }

    /// A path from text that the caller has already shown to start with `/` and to hold no NUL.
    pub(super) const fn from_checked(text: String) -> Self {
        Self(text)
    }
}

impl TryFrom<&str> for AbsolutePath {
    type Error = AbsolutePathError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.starts_with('/') && !text.contains('\0') {
            Ok(Self(text.to_owned()))
        } else {
            Err(AbsolutePathError)
        }
    }
}

impl fmt::Display for AbsolutePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{AbsolutePath, AbsolutePathError};

    #[test]
    fn a_path_is_absolute_when_it_starts_with_a_slash_and_holds_no_nul() {
        for good in ["/", "/a", "/a/../b", "//a", "/a b/é"] {
            assert_eq!(
                AbsolutePath::try_from(good).expect("absolute").as_str(),
                good
            );
        }
        for bad in ["", "a", "./a", "~/a", " /a", "/a\0b", "\\a"] {
            assert_eq!(
                AbsolutePath::try_from(bad),
                Err(AbsolutePathError),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn absolute_paths_sort_in_name_order() {
        let path = |text| AbsolutePath::try_from(text).expect("absolute");
        assert!(path("/a-b") < path("/a/b"));
        assert!(path("/a") < path("/a/b"));
        assert!(path("/z") < path("/\u{e9}"));
    }
}
