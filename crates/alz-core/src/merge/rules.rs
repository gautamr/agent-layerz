//! The merge of path-scoped rules (spec §7.10).
//!
//! A rule's identity is its file name. The most specific layer that defines a rule wins, with the
//! whole file. A file with `mode: extend` in a more specific layer appends its body under the
//! layer's heading and adds its globs to the rule's, each glob once.
//!
//! The merge of two layers' say on one rule is [`RuleChain::merge`], and it is associative: a
//! defining file discards everything before it, and an extending file adds to what is there. That
//! is why the stack can be folded in any grouping and give the same rules.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::item::{Glob, RuleBase, RuleEdit, RuleExtension, RuleFile, RuleItems, RuleName, place};
use crate::layer::{LayerHeading, LayerMeta};
use crate::merge::action::Action;
use crate::merge::algebra::OrderedSet;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::stack::ResolvedLayer;
use crate::untrusted_text::Stripped;

/// What one extending file adds to a rule: its text and globs, under the heading of its layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSection {
    heading: LayerHeading,
    extension: RuleExtension,
}

impl RuleSection {
    /// The heading of the layer that extended the rule.
    #[must_use]
    pub const fn heading(&self) -> &LayerHeading {
        &self.heading
    }

    /// The globs the layer added, and its text.
    #[must_use]
    pub const fn extension(&self) -> &RuleExtension {
        &self.extension
    }
}

/// What the layers of a stack say about one rule, as a value that combines. It holds the
/// defining file, if a layer so far defined one, and the extensions after it in stack order.
///
/// `RuleChain::default()` is the identity of [`RuleChain::merge`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleChain {
    base: Option<RuleBase>,
    extensions: Vec<RuleSection>,
}

impl RuleChain {
    /// What one file of the layer `meta` describes says about its rule.
    #[must_use]
    pub fn of(edit: &RuleEdit, meta: &LayerMeta) -> Self {
        match edit {
            RuleEdit::Replace(base) => Self {
                base: Some(base.clone()),
                extensions: Vec::new(),
            },
            RuleEdit::Extend(extension) => Self {
                base: None,
                extensions: vec![RuleSection {
                    heading: LayerHeading::of(meta),
                    extension: extension.clone(),
                }],
            },
        }
    }

    /// The chain with what a more specific layer says: its definition replaces everything before
    /// it, and its extensions come after those that are there.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        match later.base {
            Some(_) => later,
            None => Self {
                base: self.base,
                extensions: self
                    .extensions
                    .into_iter()
                    .chain(later.extensions)
                    .collect(),
            },
        }
    }

    /// Whether a layer defined the rule, so that an extension has something to extend.
    #[must_use]
    pub const fn is_defined(&self) -> bool {
        self.base.is_some()
    }

    /// The rule, when a layer defined it.
    #[must_use]
    pub fn into_rule(self) -> Option<MergedRule> {
        self.base.map(|base| MergedRule {
            base,
            extensions: self.extensions,
        })
    }
}

/// A rule after the merge: the defining file and the extensions that came after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedRule {
    base: RuleBase,
    extensions: Vec<RuleSection>,
}

impl MergedRule {
    /// The globs of the rule: those of the defining file, then those each extension added, each
    /// glob once.
    #[must_use]
    pub fn paths(&self) -> OrderedSet<Glob> {
        self.extensions
            .iter()
            .fold(self.base.paths().clone(), |paths, section| {
                paths.merge(section.extension.paths().clone())
            })
    }

    /// The text of the defining file, then each extension's text under its layer's heading,
    /// separated by one empty line. An extension without text adds no section.
    #[must_use]
    pub fn body(&self) -> String {
        std::iter::once(self.base.body().to_owned())
            .chain(self.extensions.iter().map(|section| {
                if section.extension.body().is_empty() {
                    String::new()
                } else {
                    format!("{}\n\n{}", section.heading, section.extension.body())
                }
            }))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The defining file.
    #[must_use]
    pub const fn base(&self) -> &RuleBase {
        &self.base
    }

    /// The extensions, in stack order.
    #[must_use]
    pub fn extensions(&self) -> &[RuleSection] {
        &self.extensions
    }
}

/// The state of the fold for rules.
#[derive(Debug, Default)]
pub(super) struct RuleState(BTreeMap<RuleName, Traced<RuleChain>>);

impl RuleState {
    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let (items, read) = RuleItems::read(context.layer());
        let (rules, found) = items
            .files()
            .iter()
            .fold((self.0, read), |(rules, found), file| {
                let (rules, problem) = apply(rules, context.layer(), file);
                (rules, found.merge(problem.into_iter().collect()))
            });
        (Self(rules), found)
    }

    /// The rules that a layer defined.
    pub(super) fn finish(self) -> BTreeMap<RuleName, Traced<MergedRule>> {
        self.0
            .into_iter()
            .filter_map(|(name, chain)| {
                chain
                    .filter_map(RuleChain::into_rule)
                    .map(|rule| (name, rule))
            })
            .collect()
    }
}

/// The rules after one file of the layer `meta` describes, and the problem when an extension has
/// no rule to extend.
fn apply(
    rules: BTreeMap<RuleName, Traced<RuleChain>>,
    layer: &ResolvedLayer,
    file: &RuleFile,
) -> (BTreeMap<RuleName, Traced<RuleChain>>, Option<Diagnostic>) {
    let meta = layer.meta();
    let origin = Origin::new(layer, file.path().clone());
    let chain = RuleChain::of(file.edit(), meta);
    let mut rules = rules;
    let (revised, problem) = match (rules.remove(file.name()), file.edit()) {
        (None, RuleEdit::Replace(_)) => (Some(Traced::new(chain, origin, Action::Define)), None),
        (None, RuleEdit::Extend(_)) => (None, Some(unresolved(meta, file))),
        (Some(previous), RuleEdit::Replace(_)) => {
            let merged = previous.value().clone().merge(chain);
            (
                Some(previous.revised(merged, origin, Action::Replace)),
                None,
            )
        }
        (Some(previous), RuleEdit::Extend(extension)) => {
            let merged = previous.value().clone().merge(chain);
            (
                Some(previous.revised(merged, origin, Action::extend_rule(extension))),
                None,
            )
        }
    };
    if let Some(rule) = revised {
        rules.insert(file.name().clone(), rule);
    }
    (rules, problem)
}

fn unresolved(meta: &LayerMeta, file: &RuleFile) -> Diagnostic {
    Diagnostic::error(
        Code::Unresolved,
        Some(place(meta.name(), file.path())),
        Message::new("mode: extend, but no more general layer has ")
            .then(Stripped::from_untrusted(file.path().as_str())),
    )
}

#[cfg(test)]
mod tests;
