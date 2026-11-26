//! Path-scoped rules (spec §7.10).

use alz_adapter::PathRule;

use crate::merge::{Action, Contribution, Merged, MergedRule, Traced};

use super::sources::of_origins;

/// The rules, by name.
pub(super) fn of(merged: &Merged) -> Vec<PathRule> {
    merged
        .rules()
        .iter()
        .map(|(name, traced)| one(name.as_str(), traced))
        .collect()
}

fn one(name: &str, traced: &Traced<MergedRule>) -> PathRule {
    let contributions = traced.provenance().contributions();
    // The definition that stands is the last one that is not an extension; the extensions after
    // it are sources too, and what came before it is overridden.
    let first = contributions
        .iter()
        .rposition(|part| !matches!(part.action(), Action::ExtendRule { .. }))
        .unwrap_or(0);
    let mut rule = PathRule::new(
        name,
        traced
            .value()
            .paths()
            .iter()
            .map(|glob| glob.as_str().to_owned())
            .collect(),
        traced.value().body(),
    );
    rule.sources = of_origins(contributions.iter().skip(first).map(Contribution::origin));
    rule
}
