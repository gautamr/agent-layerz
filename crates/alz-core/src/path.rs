//! A path inside a layer or a repository, as one `/`-separated string (spec §5.3 "File names",
//! §6 "Name order"). The type is the proof that the text has no way out of its directory.

use std::fmt;

use thiserror::Error;

mod absolute;

pub use absolute::{AbsolutePath, AbsolutePathError};

/// A relative path that stays where it is: `/`-separated, with no empty, `.`, `..` or `.git`
/// segment, no backslash and no NUL. Its text is valid UTF-8 because a `str` is.
///
/// A path is one string, not a list of components, so `Ord` is `str`'s: the byte order of the
/// UTF-8 text, which is the code point order of the spec's name order (§6). `a-b` sorts before
/// `a/b`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SafeRelPath(String);

/// Text that is not a safe relative path. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "not a safe relative path: '/'-separated, with no empty, '.', '..' or '.git' segment, no backslash and no NUL"
)]
pub struct SafeRelPathError;

impl SafeRelPathError {
    /// The rule, as the end of a sentence about a path.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be a '/'-separated relative path with no empty, '.', '..' or '.git' segment, no backslash and no NUL"
    }
}

impl SafeRelPath {
    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The segments of the path, in order.
    pub fn segments(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.split('/')
    }
}

fn is_safe_segment(segment: &str) -> bool {
    !matches!(segment, "" | "." | ".." | ".git") && !segment.contains(['\\', '\0'])
}

impl TryFrom<&str> for SafeRelPath {
    type Error = SafeRelPathError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.split('/').all(is_safe_segment) {
            Ok(Self(text.to_owned()))
        } else {
            Err(SafeRelPathError)
        }
    }
}

impl fmt::Display for SafeRelPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, prop_assert_eq, proptest};

    use super::{SafeRelPath, SafeRelPathError};

    fn path(text: &str) -> SafeRelPath {
        SafeRelPath::try_from(text).expect("a safe path")
    }

    #[test]
    fn a_path_is_safe_only_when_every_segment_is() {
        for good in [
            "a",
            "a/b",
            ".github/x",
            "..a",
            "a..",
            ".gitignore",
            "a b/ü",
            "a/.git2",
        ] {
            assert_eq!(path(good).as_str(), good);
        }
        let bad = [
            "",
            "/",
            "/a",
            "a/",
            "a//b",
            ".",
            "..",
            "a/./b",
            "a/../b",
            "../a",
            ".git",
            "a/.git",
            ".git/config",
            "a\\b",
            "a\0b",
            "\\",
            "a/\0",
        ];
        for text in bad {
            assert_eq!(
                SafeRelPath::try_from(text),
                Err(SafeRelPathError),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_dash_sorts_before_a_slash_because_a_path_is_one_string() {
        assert!(path("a-b") < path("a/b"));
        assert!(path("a") < path("a/b"));
        assert!(path("10") < path("9"));
    }

    fn any_path() -> impl Strategy<Value = SafeRelPath> {
        proptest::collection::vec("[a-zA-Z0-9 _.\u{e9}\u{4e2d}\u{1f600}-]{1,6}", 1..4)
            .prop_filter_map("a segment that is not safe", |segments| {
                SafeRelPath::try_from(segments.join("/").as_str()).ok()
            })
    }

    proptest! {
        /// Spec §6: name order is the code point order, which is also the UTF-8 byte order.
        #[test]
        fn name_order_is_code_point_order_and_utf8_byte_order(
            left in any_path(),
            right in any_path(),
        ) {
            let by_code_point = left.as_str().chars().cmp(right.as_str().chars());
            let by_bytes = left.as_str().as_bytes().cmp(right.as_str().as_bytes());
            prop_assert_eq!(left.cmp(&right), by_code_point);
            prop_assert_eq!(left.cmp(&right), by_bytes);
        }

        #[test]
        fn a_path_prints_back_as_it_was_read(left in any_path()) {
            prop_assert_eq!(SafeRelPath::try_from(left.to_string().as_str()), Ok(left));
        }
    }
}
