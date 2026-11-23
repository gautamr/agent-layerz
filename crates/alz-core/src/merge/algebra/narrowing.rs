//! Narrowing (spec §7.2): when a tool entry grants nothing beyond another.
//!
//! [`covers`] is a preorder on [`ToolGrant`]: reflexive and transitive, with no other law. An
//! overlay may only leave an agent with entries that an entry of the agent it refines covers, so
//! by transitivity every entry of a merged agent is covered by one of its base unless an overlay
//! set `allow_widen`.
//!
//! The rule compares by what a scope means, not by its text:
//!
//! - a path scope compares whole segments, split at `/`, so `docs/**` covers `docs/adr/**` and
//!   `docs/a.md` and never `docs-private/a.md`, and `*` never crosses a `/`;
//! - a command scope may end in `*`, and the part that `*` stands for may not hold a shell
//!   operator, so `mise run *` covers `mise run test` and never `mise run a; b`.

use crate::item::{ToolGrant, ToolScope};
use crate::merge::OrderedSet;

/// The characters that join, redirect, group or substitute commands. A command that a `*` stands
/// for may hold none of them, and neither a line feed nor a carriage return.
const SHELL_OPERATORS: [char; 10] = [';', '&', '|', '<', '>', '(', ')', '`', '\n', '\r'];

/// Whether `candidate` grants nothing beyond `base`: the same tool, and either `base` has no
/// scope, the scopes are equal, or `base`'s scope covers `candidate`'s by the rule of its kind.
#[must_use]
pub fn covers(base: &ToolGrant, candidate: &ToolGrant) -> bool {
    base.tool() == candidate.tool()
        && match (base.scope(), candidate.scope()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(base), Some(candidate)) => scope_covers(base, candidate),
        }
}

fn scope_covers(base: &ToolScope, candidate: &ToolScope) -> bool {
    base == candidate
        || match (base, candidate) {
            (ToolScope::Path(base), ToolScope::Path(candidate)) => path_covers(base, candidate),
            (ToolScope::Command(base), ToolScope::Command(candidate)) => {
                command_covers(base, candidate)
            }
            // Any other wildcard covers only an equal scope, and so does every scope of a tool
            // that is neither a path nor a command tool.
            (
                ToolScope::Other(_),
                ToolScope::Other(_) | ToolScope::Path(_) | ToolScope::Command(_),
            )
            | (ToolScope::Path(_) | ToolScope::Command(_), ToolScope::Other(_))
            | (ToolScope::Path(_), ToolScope::Command(_))
            | (ToolScope::Command(_), ToolScope::Path(_)) => false,
        }
}

/// A path base whose last segment is `**` covers a candidate whose segments, compared whole and as
/// written, start with the base's other segments, when the candidate has no `.` or `..` segment.
///
/// A base whose other segments end in `**` too (`docs/**/**`) covers only an equal scope: the
/// rule as written would make it cover `docs/**`, which covers `docs/a`, which it does not cover
/// itself, and a covering relation must be transitive.
fn path_covers(base: &str, candidate: &str) -> bool {
    let base: Vec<&str> = base.split('/').collect();
    let candidate: Vec<&str> = candidate.split('/').collect();
    base.split_last().is_some_and(|(last, others)| {
        *last == "**"
            && others.last() != Some(&"**")
            && candidate
                .iter()
                .all(|segment| !matches!(*segment, "." | ".."))
            && candidate.starts_with(others)
    })
}

/// A command base that ends in `*` covers a candidate that starts with the base's text before its
/// trailing `*`s, when the rest of the candidate holds no shell operator.
fn command_covers(base: &str, candidate: &str) -> bool {
    base.ends_with('*')
        && candidate
            .strip_prefix(base.trim_end_matches('*'))
            .is_some_and(|rest| !rest.contains(SHELL_OPERATORS))
}

/// The entries of `result` that no entry of `base` covers, in order. An agent that has no list of
/// tools has every tool, so nothing widens it.
#[must_use]
pub fn widened<'a>(
    base: Option<&OrderedSet<ToolGrant>>,
    result: &'a OrderedSet<ToolGrant>,
) -> Vec<&'a ToolGrant> {
    base.map_or_else(Vec::new, |base| {
        result
            .iter()
            .filter(|entry| !base.iter().any(|granted| covers(granted, entry)))
            .collect()
    })
}

#[cfg(test)]
mod tests;
