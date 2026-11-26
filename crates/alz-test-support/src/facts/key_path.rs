//! The key path of a key an adapter writes: where the key sits in a structured value.

use std::fmt;

/// One step of a key path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    /// A key of an object, a table or a frontmatter mapping.
    Key(String),
    /// Any item of an array.
    Items,
}

/// One step of a [`KeyPath`], as a pattern can match it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part<'a> {
    /// A key.
    Key(&'a str),
    /// Any item of an array.
    Items,
}

/// The path of a key in a structured value: the keys from the top down, with `[]` for the items of
/// an array. It is written `hooks.PreToolUse[].hooks[].type`: keys joined by `.`, and `[]` right
/// after the key of the array, whatever the item's index. A top-level key is the key alone.
///
/// The path is for looking a key up among the facts, not a serialisation: a key that is literally
/// `[]` or holds a `.` prints like a path of several steps, which [`KeyPath::parts`] does not
/// confuse.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct KeyPath(Vec<Step>);

impl KeyPath {
    /// The path of a top-level key.
    #[must_use]
    pub fn top(key: &str) -> Self {
        Self::default().key(key)
    }

    /// This path followed by `key`.
    #[must_use]
    pub fn key(&self, key: &str) -> Self {
        self.step(Step::Key(key.to_owned()))
    }

    /// This path followed by the items of an array.
    #[must_use]
    pub fn items(&self) -> Self {
        self.step(Step::Items)
    }

    fn step(&self, step: Step) -> Self {
        Self(self.0.iter().cloned().chain(Some(step)).collect())
    }

    /// The steps from the top down, for a subject map to match on, such as
    /// `[Part::Key("permissions"), Part::Key(key)]`.
    #[must_use]
    pub fn parts(&self) -> Vec<Part<'_>> {
        self.0
            .iter()
            .map(|step| match step {
                Step::Key(key) => Part::Key(key),
                Step::Items => Part::Items,
            })
            .collect()
    }

    /// The last key of the path, the name that the facts record, or `None` for the empty path.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.iter().rev().find_map(|step| match step {
            Step::Key(key) => Some(key.as_str()),
            Step::Items => None,
        })
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .enumerate()
            .try_for_each(|(index, step)| match step {
                Step::Key(key) if index == 0 => write!(formatter, "{key}"),
                Step::Key(key) => write!(formatter, ".{key}"),
                Step::Items => write!(formatter, "[]"),
            })
    }
}
