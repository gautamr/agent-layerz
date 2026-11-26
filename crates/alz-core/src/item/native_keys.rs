//! The native keys with a security meaning (spec §7.9, §8.0 **Descriptor.**): the rules that say
//! which keys of a target's native settings alz refuses or only lets get stricter.
//!
//! The rows are data of the adapter: its descriptor lists them in [`NativeVocabulary`], and
//! [`KeyRules`] reads them for one target. The core holds the machinery that applies them, so the
//! security floor (§7.6, §7.9, §9.3) stays the core's whatever adapter states the keys. A row is a
//! [`NativeKey`]: a target, a key path and a [`KeyRule`].
//!
//! - [`KeyRule::FirstClass`]: the key duplicates a first-class item, so `settings.yaml` refuses it
//!   and names the place that sets it.
//! - [`KeyRule::Reserved`]: the key is refused for the reason given: it runs a command, widens
//!   what a project may do, or is meant for a scope that alz does not write.
//! - [`KeyRule::Floor`]: the key may be set, but a more specific layer may only move it up its
//!   [`Strictness`]: a looser value, a list that gains an entry where none may, or a change of
//!   kind is a `settings` error that names the layer, the file and the key path.
//!
//! A key path is the key names from the target's mapping down, joined by `.`. A segment `*` is any
//! one key, and `**` is any run of keys or list items, so `**.a.b` is `b` in an `a` mapping
//! wherever it stands. A path is never the text of a layer: the errors print the path of the row,
//! so no value and no profile name of the layer reaches the output.
//!
//! The first row that matches a key path applies; the rows come in the order first-class, floor,
//! reserved, and in the order of the descriptor within each. Adapters may rely on a key only once
//! `docs/target-facts.md` records it.
//!
//! [`KeyRules::loosenings`] is the check of the floors: pure, over the merged tree of the more
//! general layers and the tree of the more specific one.

use std::mem::discriminant;

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::merge::Tree;
use crate::untrusted_text::{QuotedKey, Stripped};
use alz_adapter::{FloorRule, NativeVocabulary, TargetId};

/// How a key that has a floor gets stricter: the core's reading of a [`FloorRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness<'a> {
    /// A boolean; the payload is the stricter value, and once a layer sets it, a more specific
    /// layer may not set the other.
    Flag(bool),
    /// Text in ranks, loosest rank first; a value of a rank may move to its own or a later rank.
    /// A value that is in no rank may only stay as it was.
    Ladder(&'a [Vec<String>]),
    /// A list or mapping that may not gain an entry (list item or mapping key, at any depth) that
    /// a more general layer did not set. Settings have no removal syntax, so it cannot shrink
    /// either.
    NoAdditions,
    /// A list that may only grow.
    Union,
}

impl<'a> From<&'a FloorRule> for Strictness<'a> {
    fn from(rule: &'a FloorRule) -> Self {
        match rule {
            FloorRule::TrueIsStricter => Self::Flag(true),
            FloorRule::FalseIsStricter => Self::Flag(false),
            FloorRule::Ranked(ranks) => Self::Ladder(ranks),
            FloorRule::NoAdditions => Self::NoAdditions,
            FloorRule::Union => Self::Union,
        }
    }
}

/// What a row says about a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRule<'a> {
    /// The key duplicates a first-class item, set through the place named.
    FirstClass(&'a str),
    /// The key is refused, for the reason given.
    Reserved(&'a str),
    /// The key may only get stricter.
    Floor(Strictness<'a>),
}

/// A row: a key path of a target and what is said about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeKey<'a> {
    target: &'a TargetId,
    path: &'a str,
    rule: KeyRule<'a>,
}

impl<'a> NativeKey<'a> {
    /// The target whose native settings hold the key.
    #[must_use]
    pub const fn target(&self) -> &'a TargetId {
        self.target
    }

    /// The key path, with `*` and `**` as wildcards.
    #[must_use]
    pub const fn path(&self) -> &'a str {
        self.path
    }

    /// What the row says about the key.
    #[must_use]
    pub const fn rule(&self) -> KeyRule<'a> {
        self.rule
    }

    /// The path as the errors print it: the target, then the key path.
    #[must_use]
    pub fn display_path(&self) -> String {
        format!("{}.{}", self.target, self.path)
    }

    /// Whether the key path of the row stands for `steps`.
    fn matches(&self, steps: &[Step<'_>]) -> bool {
        pattern_matches(&self.path.split('.').collect::<Vec<_>>(), steps)
    }
}

/// The key rules of one target: its descriptor's [`NativeVocabulary`] read by the core.
#[derive(Debug, Clone, Copy)]
pub struct KeyRules<'a> {
    target: &'a TargetId,
    vocabulary: &'a NativeVocabulary,
}

impl<'a> KeyRules<'a> {
    /// The rules of `target` that `vocabulary` states.
    #[must_use]
    pub const fn new(target: &'a TargetId, vocabulary: &'a NativeVocabulary) -> Self {
        Self { target, vocabulary }
    }

    /// The target the rules are for.
    #[must_use]
    pub const fn target(&self) -> &'a TargetId {
        self.target
    }

    /// The vocabulary the rules read: the descriptor's, or the registry's descriptor-only entry.
    #[must_use]
    pub const fn vocabulary(&self) -> &'a NativeVocabulary {
        self.vocabulary
    }

    /// The rows in lookup order: first-class, floor, then reserved, each in the descriptor's order.
    pub fn rows(&self) -> impl Iterator<Item = NativeKey<'a>> + use<'a> {
        let target = self.target;
        let vocabulary = self.vocabulary;
        let first_class = vocabulary
            .first_class_keys
            .iter()
            .map(move |key| NativeKey {
                target,
                path: &key.path,
                rule: KeyRule::FirstClass(&key.set_through),
            });
        let floors = vocabulary.floor_keys.iter().map(move |key| NativeKey {
            target,
            path: &key.path,
            rule: KeyRule::Floor(Strictness::from(&key.rule)),
        });
        let reserved = vocabulary.reserved_keys.iter().map(move |key| NativeKey {
            target,
            path: &key.path,
            rule: KeyRule::Reserved(&key.reason),
        });
        first_class.chain(floors).chain(reserved)
    }

    /// The row for `steps` below the mapping of the target, when there is one.
    #[must_use]
    pub fn native_key(&self, steps: &[Step<'_>]) -> Option<NativeKey<'a>> {
        self.rows().find(|key| key.matches(steps))
    }

    /// The row that refuses the key at `steps`, when one does: a first-class item or a reserved
    /// key.
    #[must_use]
    pub fn refusal(&self, steps: &[Step<'_>]) -> Option<NativeKey<'a>> {
        self.native_key(steps).filter(|key| match key.rule {
            KeyRule::FirstClass(_) | KeyRule::Reserved(_) => true,
            KeyRule::Floor(_) => false,
        })
    }

    /// The floor keys that `after`, the settings of a more specific layer, loosens against
    /// `before`, the settings merged from the more general ones. A key that `before` does not set,
    /// or sets to `null`, is free.
    #[must_use]
    pub fn loosenings(&self, before: &Tree, after: &Tree) -> Vec<Loosening<'a>> {
        self.descend(&[], Some(before), after)
    }

    fn descend(
        &self,
        steps: &[Step<'_>],
        before: Option<&Tree>,
        after: &Tree,
    ) -> Vec<Loosening<'a>> {
        let floor = self.native_key(steps).and_then(|key| match key.rule {
            KeyRule::Floor(strictness) => Some((key, strictness)),
            KeyRule::FirstClass(_) | KeyRule::Reserved(_) => None,
        });
        match (floor, before) {
            (Some((key, strictness)), Some(held)) if *held != Tree::Null => strictness
                .admits(held, after)
                .err()
                .map(|how| Loosening { key, how })
                .into_iter()
                .collect(),
            (Some(_), _) => Vec::new(),
            (None, _) => match after {
                Tree::Map(entries) => entries
                    .iter()
                    .flat_map(|(key, value)| {
                        let below = [steps, &[Step::Key(key)]].concat();
                        let held = match before {
                            Some(Tree::Map(held)) => held.get(key),
                            Some(_) | None => None,
                        };
                        self.descend(&below, held, value)
                    })
                    .collect(),
                Tree::List(items) => items
                    .iter()
                    .flat_map(|item| self.descend(&[steps, &[Step::Unnamed]].concat(), None, item))
                    .collect(),
                Tree::Null | Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) => {
                    Vec::new()
                }
            },
        }
    }
}

/// One step down a tree: a mapping key, or anything that no key path names (a list item, a key
/// that is not text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step<'a> {
    /// A mapping key.
    Key(&'a str),
    /// A list item or a key that is not text.
    Unnamed,
}

fn pattern_matches(pattern: &[&str], steps: &[Step<'_>]) -> bool {
    match (pattern.split_first(), steps.split_first()) {
        (None, None) => true,
        (Some((&"**", rest)), _) => {
            pattern_matches(rest, steps)
                || steps
                    .split_first()
                    .is_some_and(|(_, below)| pattern_matches(pattern, below))
        }
        (Some((&"*", rest)), Some((Step::Key(_), below))) => pattern_matches(rest, below),
        (Some((&name, rest)), Some((&Step::Key(key), below))) => {
            name == key && pattern_matches(rest, below)
        }
        (None, Some(_)) | (Some(_), None | Some((Step::Unnamed, _))) => false,
    }
}

/// The error for the key `key` that a row refuses, at `at`.
#[must_use]
pub fn refusal_error(key: NativeKey<'_>, at: &Where) -> Option<Diagnostic> {
    let named = Message::new(QuotedKey::from_untrusted(&key.display_path()));
    let message = match key.rule {
        KeyRule::FirstClass(owner) => named
            .then(" is set through ")
            .then(Stripped::from_untrusted(owner)),
        KeyRule::Reserved(reason) => named
            .then(" is not set in settings.yaml: ")
            .then(Stripped::from_untrusted(reason)),
        KeyRule::Floor(_) => return None,
    };
    Some(Diagnostic::error(Code::Settings, Some(at.clone()), message))
}

/// Why a value is not as strict as the one a more general layer set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Looseness {
    /// The value is looser on the order of the key.
    Looser,
    /// A list or mapping gains an entry where none may.
    Added,
    /// The value is not of the kind the more general layer set.
    Kind,
}

impl Looseness {
    const fn sentence(self) -> &'static str {
        match self {
            Self::Looser => {
                " is a floor: this value is looser than the one a more general layer set"
            }
            Self::Added => {
                " is a floor: it may not gain entries that a more general layer did not set"
            }
            Self::Kind => " is a floor: its kind differs from the one a more general layer set",
        }
    }
}

/// A floor key that a more specific layer loosens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loosening<'a> {
    key: NativeKey<'a>,
    how: Looseness,
}

impl<'a> Loosening<'a> {
    /// The row of the key.
    #[must_use]
    pub const fn key(&self) -> NativeKey<'a> {
        self.key
    }

    /// How the value is looser.
    #[must_use]
    pub const fn how(&self) -> Looseness {
        self.how
    }

    /// The `settings` error at `at`, which names the layer and the file; the key path is the one
    /// of the row.
    #[must_use]
    pub fn diagnostic(&self, at: &Where) -> Diagnostic {
        Diagnostic::error(
            Code::Settings,
            Some(at.clone()),
            Message::new(QuotedKey::from_untrusted(&self.key.display_path()))
                .then(self.how.sentence()),
        )
    }
}

impl Strictness<'_> {
    /// Whether `after`, from a more specific layer, is at least as strict as `before`.
    fn admits(self, before: &Tree, after: &Tree) -> Result<(), Looseness> {
        match (self, before, after) {
            (Self::Flag(stricter), Tree::Bool(held), Tree::Bool(next)) => {
                if *held == stricter && *next != stricter {
                    Err(Looseness::Looser)
                } else {
                    Ok(())
                }
            }
            (Self::Ladder(ranks), Tree::Text(held), Tree::Text(next)) => {
                match (rank(ranks, held), rank(ranks, next)) {
                    (Some(held), Some(next)) if next < held => Err(Looseness::Looser),
                    (Some(_), Some(_)) => Ok(()),
                    (None, _) | (_, None) if held == next => Ok(()),
                    (None, _) | (_, None) => Err(Looseness::Looser),
                }
            }
            (Self::NoAdditions, Tree::List(_), Tree::List(_))
            | (Self::NoAdditions, Tree::Map(_), Tree::Map(_)) => {
                if adds(before, after) {
                    Err(Looseness::Added)
                } else {
                    Ok(())
                }
            }
            (Self::Union, Tree::List(_), Tree::List(_)) => Ok(()),
            (
                Self::Flag(_) | Self::Ladder(_) | Self::NoAdditions | Self::Union,
                Tree::Null
                | Tree::Bool(_)
                | Tree::Int(_)
                | Tree::Float(_)
                | Tree::Text(_)
                | Tree::List(_)
                | Tree::Map(_),
                _,
            ) => Err(Looseness::Kind),
        }
    }
}

/// The rank of `value`, loosest first.
fn rank(ranks: &[Vec<String>], value: &str) -> Option<usize> {
    ranks
        .iter()
        .position(|names| names.iter().any(|name| name == value))
}

/// Whether `after` has an entry, a list item or a mapping key at any depth, that `before` has
/// not, or a value of another kind where `before` has a list or mapping.
fn adds(before: &Tree, after: &Tree) -> bool {
    match (before, after) {
        (Tree::List(held), Tree::List(next)) => next.iter().any(|item| !held.contains(item)),
        (Tree::Map(held), Tree::Map(next)) => next
            .iter()
            .any(|(key, value)| held.get(key).is_none_or(|before| adds(before, value))),
        (held, next) => discriminant(held) != discriminant(next),
    }
}

#[cfg(test)]
mod spec_lists;
#[cfg(test)]
mod tests;
