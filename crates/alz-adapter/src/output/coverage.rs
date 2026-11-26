//! The coverage report (spec §8.0 **Coverage report.**).

use crate::model::SecurityItemId;

/// How an adapter's output enforces a security item on the tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Enforcement {
    /// As the model states it.
    Expressed,
    /// At least as strictly, in another form, such as a broader deny rule. The adapter says how in
    /// a warning with one of its own codes.
    Approximated,
    /// Anything else. From TASK-127 this is the error `target` unless the layer that set the item
    /// opts the target out of it (§8.0 **Security items a target cannot enforce.**).
    NotExpressed,
}

/// One line of the report: a security item of the model and how the output enforces it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CoverageEntry {
    /// The item.
    pub item: SecurityItemId,
    /// How it is enforced.
    pub enforcement: Enforcement,
}

impl CoverageEntry {
    /// An entry.
    #[must_use]
    pub const fn new(item: SecurityItemId, enforcement: Enforcement) -> Self {
        Self { item, enforcement }
    }
}

/// The coverage report: an entry for each security item of the model. It is a list, not a map, so
/// the core can see an item that is named twice and refuse it (§8.0 **Output checks.**). The core
/// cannot check what a native key means, only that every item is accounted for. An item of a kind
/// that a later minor version of the model adds is left out of the report of an adapter built for
/// an earlier version.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Coverage(Vec<CoverageEntry>);

impl Coverage {
    /// A report of `entries`, in the order given.
    #[must_use]
    pub const fn new(entries: Vec<CoverageEntry>) -> Self {
        Self(entries)
    }

    /// A report that states one enforcement for every item of `items`, in the order given: the
    /// report of an adapter that treats all items alike, such as one that expresses every item.
    #[must_use]
    pub fn all(items: impl IntoIterator<Item = SecurityItemId>, enforcement: Enforcement) -> Self {
        Self(
            items
                .into_iter()
                .map(|item| CoverageEntry::new(item, enforcement))
                .collect(),
        )
    }

    /// The entries, in order.
    #[must_use]
    pub fn entries(&self) -> &[CoverageEntry] {
        &self.0
    }
}
