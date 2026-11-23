//! The `account` field of a layer (spec §7.13): one account for every target, or one per target.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::account::name::{AccountName, AccountNameError};
use crate::schema::{
    Checked, Refusal, SchemaError, SchemaErrors, refine_node, unknown_fields, zip,
};
use crate::target::TargetName;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{Node, Value};

/// The account a layer picks for a target: the default account, or a named one. `default` always
/// means the default account (§7.13), so a named account is never called `default`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountChoice {
    /// The default account: the target's variable is left unset.
    Default,
    /// A named account.
    Named(AccountName),
}

impl TryFrom<&str> for AccountChoice {
    type Error = AccountNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "default" => Ok(Self::Default),
            other => AccountName::try_from(other).map(Self::Named),
        }
    }
}

/// A map from targets to values with at least one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyTargetMap<T>(BTreeMap<TargetName, T>);

/// A target map without an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a per-target map needs at least one target")]
pub struct EmptyTargetMap;

impl<T> NonEmptyTargetMap<T> {
    /// The map of `entries`; a target that comes twice keeps its last value.
    ///
    /// # Errors
    ///
    /// [`EmptyTargetMap`] when there is no entry.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (TargetName, T)>,
    ) -> Result<Self, EmptyTargetMap> {
        let map: BTreeMap<TargetName, T> = entries.into_iter().collect();
        if map.is_empty() {
            Err(EmptyTargetMap)
        } else {
            Ok(Self(map))
        }
    }

    /// The value of `target`, `None` when the map leaves it out.
    #[must_use]
    pub fn get(&self, target: TargetName) -> Option<&T> {
        self.0.get(&target)
    }

    /// The entries in target order.
    pub fn iter(&self) -> impl Iterator<Item = (TargetName, &T)> + '_ {
        self.0.iter().map(|(target, value)| (*target, value))
    }
}

/// What is wrong with an `account` value as a whole (a bad name or a bad target key has its own
/// error). The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AccountShapeError {
    /// A value that is neither text nor a mapping.
    #[error("an account is a name or a mapping of target to name")]
    NotNameOrMapping,
    /// A mapping without an entry.
    #[error("an account mapping must not be empty")]
    EmptyMapping,
}

impl AccountShapeError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::NotNameOrMapping => "must be an account name or a mapping of target to name",
            Self::EmptyMapping => "must not be an empty mapping",
        }
    }
}

/// The `account` field of a layer: one choice for every target, or a choice for each target the
/// layer decides (a target left out is not decided by the layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountDeclaration {
    /// `account: client-a`: the same choice for every target.
    Every(AccountChoice),
    /// `account: {claude: client-a}`: a choice for the targets named, and at least one.
    PerTarget(NonEmptyTargetMap<AccountChoice>),
}

impl AccountDeclaration {
    /// What the declaration says for `target`: `None` when the layer leaves it undecided.
    #[must_use]
    pub fn choice_for(&self, target: TargetName) -> Option<&AccountChoice> {
        match self {
            Self::Every(choice) => Some(choice),
            Self::PerTarget(map) => map.get(target),
        }
    }

    /// Reads the value of the `account` field at `path`.
    ///
    /// # Errors
    ///
    /// Every problem of the value, with its key path: a name that breaks the rule, a target key
    /// that is not `claude` or `codex`, an empty mapping or another kind of value. No error holds
    /// the value.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let refused = |refusal: Refusal, path: KeyPath| {
            SchemaErrors::from(SchemaError::refused(path, refusal, node.at()))
        };
        match node.value() {
            Value::Text(text) => AccountChoice::try_from(text.as_str())
                .map(Self::Every)
                .map_err(|error| refused(error.into(), path)),
            Value::Map(_) => NonEmptyTargetMap::from_node(node, path).map(Self::PerTarget),
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Seq(_) => {
                Err(refused(AccountShapeError::NotNameOrMapping.into(), path))
            }
        }
    }
}

impl<T> NonEmptyTargetMap<T>
where
    T: for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    /// Reads `node`, at `path`, as a mapping of target to a validated value, with at least one
    /// target.
    ///
    /// # Errors
    ///
    /// Every problem of the mapping, with its key path: a value that is not text or is refused
    /// (`null` included), a key that is not `claude` or `codex`, an empty mapping, or a node that
    /// is not a mapping. No error holds a value.
    #[expect(
        clippy::manual_try_fold,
        reason = "the fold keeps the problems of every target, which try_fold would stop at the first"
    )]
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path.clone()).map_err(SchemaError::from)?;
        let known = TargetName::ALL.map(TargetName::as_str);
        let entries = TargetName::ALL
            .into_iter()
            .map(|target| {
                // Test 29: `{claude: null}` is refused, so each entry is read as written.
                section
                    .written_node(target.as_str())
                    .map(|entry| {
                        refine_node::<T>(path.field(target.as_str()), entry)
                            .map(|choice| (target, choice))
                    })
                    .transpose()
            })
            .fold(Ok(Vec::new()), |all, next| {
                zip(all, next).map(|(mut all, next)| {
                    all.extend(next);
                    all
                })
            });
        zip(unknown_fields(&section, &known), entries)
            .map(|((), entries)| entries)
            .and_then(|entries| {
                NonEmptyTargetMap::from_entries(entries).map_err(|EmptyTargetMap| {
                    SchemaErrors::from(SchemaError::refused(
                        path,
                        AccountShapeError::EmptyMapping.into(),
                        node.at(),
                    ))
                })
            })
    }
}
