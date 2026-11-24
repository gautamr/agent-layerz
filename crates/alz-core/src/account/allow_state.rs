//! The allow state `allowed.yaml` as account values (spec §7.13 "Allow state"): the shape only,
//! and the text `alz account allow` writes. What needs the file system (the trust checks of "Both
//! files", a `root` that is not its own real path, a `root` inside an account folder, two entries
//! with one real root, the lock and the atomic write) is the capability layer's (R6, TASK-55.7).
//!
//! The top level must be a list, or the whole file is ignored. An entry that does not fit is
//! ignored with one warning that holds its position only (its number, counted from 1, and the
//! line and column); no warning holds a value or a key.

use std::num::NonZeroUsize;

use thiserror::Error;

use crate::account::declaration::NonEmptyTargetMap;
use crate::account::name::AccountName;
use crate::account::reading::{Reading, tree, unreadable, warning};
use crate::diagnostic::{Code, Message, Where};
use crate::path::AbsolutePath;
use crate::render::yaml::{EmitMap, EmitValue, emit};
use crate::yaml::fields::KeyPath;
use crate::yaml::{Node, Value};

/// The first line of every file `alz account allow` writes.
const HEADER: &str = "# written by alz account allow; deleting the file withdraws every approval\n";

/// The `source` of an approval: the real path of a local layer, or a URL without its ref. It is
/// text that is not empty; what it must match is for the resolver.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AllowedSource(String);

/// An empty `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a source must not be empty")]
pub struct AllowedSourceError;

impl AllowedSource {
    /// The source as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AllowedSource {
    type Error = AllowedSourceError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.is_empty() {
            Err(AllowedSourceError)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// One approval: the repository root, the account accepted for each target, and the layer source
/// that declared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowEntry {
    root: AbsolutePath,
    account: NonEmptyTargetMap<AccountName>,
    source: AllowedSource,
}

impl AllowEntry {
    /// An approval.
    #[must_use]
    pub const fn new(
        root: AbsolutePath,
        account: NonEmptyTargetMap<AccountName>,
        source: AllowedSource,
    ) -> Self {
        Self {
            root,
            account,
            source,
        }
    }

    /// The repository root, as written.
    #[must_use]
    pub const fn root(&self) -> &AbsolutePath {
        &self.root
    }

    /// The accepted account per target.
    #[must_use]
    pub const fn account(&self) -> &NonEmptyTargetMap<AccountName> {
        &self.account
    }

    /// The layer source the approval was given for.
    #[must_use]
    pub const fn source(&self) -> &AllowedSource {
        &self.source
    }

    /// The entry as the file writes it: `root`, `account` (target order), `source`.
    fn emit_value(&self) -> EmitValue {
        let account = self
            .account
            .iter()
            .map(|(target, name)| (target.to_string(), EmitValue::Text(name.to_string())))
            .collect();
        EmitValue::Map(map_of(vec![
            ("root".to_owned(), EmitValue::Text(self.root.to_string())),
            ("account".to_owned(), EmitValue::Map(map_of(account))),
            (
                "source".to_owned(),
                EmitValue::Text(self.source.as_str().to_owned()),
            ),
        ]))
    }
}

/// A mapping of keys that are distinct by construction here (`root`, `account`, `source`, and one
/// key per target); a repeat would be a bug of this module and yields an empty mapping, which the
/// round-trip tests would show.
fn map_of(entries: Vec<(String, EmitValue)>) -> EmitMap {
    EmitMap::from_entries(entries).unwrap_or_default()
}

/// The approvals of `allowed.yaml`, sorted by root in name order (§6). Two entries may share a
/// root as text while the file is being read; they keep the order of the file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowState(Vec<AllowEntry>);

impl AllowState {
    /// The state of `entries`, sorted by root.
    #[must_use]
    pub fn from_entries(mut entries: Vec<AllowEntry>) -> Self {
        entries.sort_by(|left, right| left.root.cmp(&right.root));
        Self(entries)
    }

    /// The entries, sorted by root.
    #[must_use]
    pub fn entries(&self) -> &[AllowEntry] {
        &self.0
    }

    /// The text of the file: the header, then the entries sorted by root, each with `root`,
    /// `account` in target order and `source`. No entry is `[]`. The same state gives the same
    /// bytes. The size limit and the atomic write are the writer's.
    #[must_use]
    pub fn to_yaml(&self) -> String {
        let entries = self.0.iter().map(AllowEntry::emit_value).collect();
        format!("{HEADER}{}", emit(&EmitValue::Seq(entries)))
    }
}

/// What reading `allowed.yaml` gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowFile {
    /// The whole file is ignored: it is not valid YAML, or its top level is not a list.
    Ignored,
    /// A list (an empty file is an empty list): the entries that fit, and the number, counted from
    /// 1, of each entry that did not.
    List {
        /// The entries that fit their shape.
        state: AllowState,
        /// The entries that were ignored, in file order.
        ignored: Vec<NonZeroUsize>,
        /// The number, counted from 1, that each entry of `state` has in the file, in the order
        /// of `state.entries()`. A caller that has to name an entry in a warning uses it.
        numbers: Vec<NonZeroUsize>,
    },
}

impl AllowFile {
    /// Reads the bytes of `allowed.yaml`, shown as `file` in warnings. A file past a YAML limit or
    /// that is not valid YAML is ignored with one warning.
    #[must_use]
    pub fn from_bytes(bytes: &[u8], file: &Where) -> Reading<Self> {
        tree(bytes).map_or_else(
            |error| Reading::new(Self::Ignored).warned(unreadable(&error, file)),
            |node| Self::from_node(&node, file),
        )
    }

    /// Reads a parsed document, shown as `file` in warnings.
    #[must_use]
    pub fn from_node(node: &Node, file: &Where) -> Reading<Self> {
        match node.value() {
            Value::Null => Reading::new(Self::empty_list()),
            Value::Seq(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| entry(number(index), item, file))
                .fold(Reading::new(Vec::new()), |read, next| {
                    read.then(next, |mut read, next| {
                        read.push(next);
                        read
                    })
                })
                .map(|read| {
                    let ignored = read
                        .iter()
                        .filter(|(_, outcome)| outcome.is_none())
                        .map(|(number, _)| *number)
                        .collect();
                    let mut kept: Vec<(NonZeroUsize, AllowEntry)> = read
                        .into_iter()
                        .filter_map(|(number, outcome)| outcome.map(|entry| (number, entry)))
                        .collect();
                    kept.sort_by(|(_, left), (_, right)| left.root.cmp(&right.root));
                    let (numbers, entries): (Vec<_>, Vec<_>) = kept.into_iter().unzip();
                    Self::List {
                        state: AllowState(entries),
                        ignored,
                        numbers,
                    }
                }),
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Map(_) => {
                Reading::new(Self::Ignored).warned(warning(
                    Code::AccountAllowedShape,
                    file,
                    node.at(),
                    Message::new("the file is ignored: its top level must be a list"),
                ))
            }
        }
    }

    const fn empty_list() -> Self {
        Self::List {
            state: AllowState(Vec::new()),
            ignored: Vec::new(),
            numbers: Vec::new(),
        }
    }
}

/// The number of the item at `index`, counted from 1.
const fn number(index: usize) -> NonZeroUsize {
    NonZeroUsize::MIN.saturating_add(index)
}

/// Why an entry is ignored. No variant holds a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Misfit {
    Keys,
    Root,
    Source,
    Account,
}

impl Misfit {
    const fn reason(self) -> &'static str {
        match self {
            Self::Keys => "it needs exactly root, account and source",
            Self::Root => "its root is not an absolute path",
            Self::Source => "its source is not a non-empty string",
            Self::Account => "its account is not a mapping of target to a valid account name",
        }
    }
}

type Outcome = (NonZeroUsize, Option<AllowEntry>);

fn entry(number: NonZeroUsize, item: &Node, file: &Where) -> Reading<Outcome> {
    read_entry(item).map_or_else(
        |misfit| {
            Reading::new((number, None)).warned(warning(
                Code::AccountAllowedEntry,
                file,
                item.at(),
                Message::new("entry ")
                    .then(number.get())
                    .then(" is ignored (")
                    .then(misfit.reason())
                    .then(")"),
            ))
        },
        |entry| Reading::new((number, Some(entry))),
    )
}

/// An entry is a mapping with exactly `root`, `account` and `source`, each of its own shape. Keys
/// of a mapping are unique, so three keys that include the three names are exactly them.
fn read_entry(item: &Node) -> Result<AllowEntry, Misfit> {
    let mapping = match item.value() {
        Value::Map(mapping) => mapping,
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_) => return Err(Misfit::Keys),
    };
    let (Some(root), Some(account), Some(source), 3) = (
        mapping.get("root"),
        mapping.get("account"),
        mapping.get("source"),
        mapping.len(),
    ) else {
        return Err(Misfit::Keys);
    };
    Ok(AllowEntry::new(
        text(root)
            .and_then(|text| AbsolutePath::try_from(text).ok())
            .ok_or(Misfit::Root)?,
        NonEmptyTargetMap::from_node(account, KeyPath::root().field("account"))
            .map_err(|_errors| Misfit::Account)?,
        text(source)
            .and_then(|text| AllowedSource::try_from(text).ok())
            .ok_or(Misfit::Source)?,
    ))
}

fn text(node: &Node) -> Option<&str> {
    match node.value() {
        Value::Text(text) => Some(text),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => None,
    }
}

#[cfg(test)]
mod tests;
