//! The `stack` of the stack manifest (spec §5.2): one or more entries, each a source with an
//! optional pin.

use thiserror::Error;

use crate::schema::{
    Checked, Refusal, SchemaError, SchemaErrors, all, refine, require, unknown_fields, zip3,
};
use crate::stack::git_ref::GitRef;
use crate::stack::source::Source;
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};

/// What is wrong with the stack as a whole. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StackError {
    /// A stack without an entry.
    #[error("a stack needs at least one entry")]
    Empty,
}

impl StackError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Empty => "must not be empty",
        }
    }
}

/// One layer of the stack: where it comes from and, for a `path:` source, the ref that pins it
/// (§6). A `ref:` next to a `github:` or `git:` source is checked and then dropped, because the
/// ref after `@` is the pin there; so only a `path:` entry can hold a pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEntry {
    source: Source,
    pin: Option<GitRef>,
}

impl StackEntry {
    /// An entry. `pin` is kept only for a `path:` source.
    #[must_use]
    pub fn new(source: Source, pin: Option<GitRef>) -> Self {
        let pin = match source {
            Source::Path { .. } => pin,
            Source::Git { .. } | Source::GitHub { .. } => None,
        };
        Self { source, pin }
    }

    /// Reads one item of `stack`, at `path`.
    ///
    /// # Errors
    ///
    /// Every problem of the entry: not a mapping, an unknown field, a missing or refused
    /// `source`, a `ref` that is not text or not a git ref.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path)?;
        zip3(
            unknown_fields(&section, &["source", "ref"]),
            require::<Source>(&section, "source"),
            refine::<GitRef>(&section, "ref"),
        )
        .map(|((), source, pin)| Self::new(source, pin))
    }

    /// The source.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    /// The ref that pins a `path:` source, if the entry has one.
    #[must_use]
    pub const fn pin(&self) -> Option<&GitRef> {
        self.pin.as_ref()
    }
}

/// The stack as declared: at least one entry, in the order written, the repository's own layer
/// last by convention. It is the first state of the pipeline (doc-6 §3.3); the resolved stack
/// holds directories as well, so it is a type of R3's and not a marker on this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredStack {
    first: StackEntry,
    rest: Vec<StackEntry>,
}

impl DeclaredStack {
    /// The stack of `entries`, `None` when there is none.
    #[must_use]
    pub fn from_entries(entries: Vec<StackEntry>) -> Option<Self> {
        let mut all = entries.into_iter();
        all.next().map(|first| Self {
            first,
            rest: all.collect(),
        })
    }

    /// Reads the required `stack` field of the manifest.
    ///
    /// # Errors
    ///
    /// `stack` missing, not a list or empty, and every problem of every entry.
    pub fn from_section(section: &Section<'_>) -> Checked<Self> {
        let path = section.field_path("stack");
        let at = section.node("stack").map_or(section.at(), Node::at);
        section
            .items("stack")?
            .ok_or_else(|| SchemaErrors::from(SchemaError::missing(path.clone(), section.at())))
            .and_then(|items| {
                all(items
                    .into_iter()
                    .map(|(item_path, node)| StackEntry::from_node(node, item_path)))
            })
            .and_then(|entries| {
                Self::from_entries(entries).ok_or_else(|| {
                    SchemaError::refused(path, Refusal::Stack(StackError::Empty), at).into()
                })
            })
    }

    /// The entries in the order written.
    pub fn iter(&self) -> impl Iterator<Item = &StackEntry> + '_ {
        std::iter::once(&self.first).chain(&self.rest)
    }

    /// How many entries: at least one.
    #[must_use]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// Always false: the stack has at least one entry. It is here for the `len` convention.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}
