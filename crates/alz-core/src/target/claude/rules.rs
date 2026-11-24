//! `.claude/rules/<name>.md` (spec §8.1, target-facts §9): the globs as `paths:` in the
//! frontmatter, the header and the body.

use crate::diagnostic::Diagnostics;
use crate::merge::{Action, Merged, MergedRule, Traced};
use crate::render::yaml::EmitValue;

use super::Output;
use super::frontmatter::{file, sources, text};

/// The folder of the rule files.
const RULES_DIR: &str = ".claude/rules";

/// The rules as files.
pub(super) fn render(merged: &Merged) -> Output {
    merged
        .rules()
        .iter()
        .map(|(name, traced)| one(name.as_str(), traced))
        .fold(Output::default(), Output::then)
}

fn one(name: &str, traced: &Traced<MergedRule>) -> Output {
    let contributions = traced.provenance().contributions();
    // The definition that stands is the last one that is not an extension; the extensions after
    // it are sources too, and what came before it is overridden.
    let first = contributions
        .iter()
        .rposition(|part| !matches!(part.action(), Action::ExtendRule { .. }))
        .unwrap_or(0);
    let origins = contributions
        .iter()
        .skip(first)
        .map(crate::merge::Contribution::origin);
    let paths = traced
        .value()
        .paths()
        .iter()
        .map(|glob| text(glob.as_str()))
        .collect();
    file(
        &format!("{RULES_DIR}/{name}.md"),
        sources(origins),
        vec![("paths".to_owned(), EmitValue::Seq(paths))],
        &traced.value().body(),
        Diagnostics::new(),
    )
}
