//! `.claude/rules/<name>.md` (spec §8.1, target-facts §9): the globs as `paths:` in the
//! frontmatter, the header and the body.

use alz_adapter::{Model, PathRule};

use crate::diagnostic::Diagnostics;
use crate::render::yaml::EmitValue;

use super::Output;
use super::frontmatter::{file, sources, text};

/// The folder of the rule files.
const RULES_DIR: &str = ".claude/rules";

/// The rules as files.
pub(super) fn render(model: &Model) -> Output {
    model
        .rules
        .iter()
        .map(one)
        .fold(Output::default(), Output::then)
}

fn one(rule: &PathRule) -> Output {
    let paths = rule.paths.iter().map(|glob| text(glob)).collect();
    file(
        &format!("{RULES_DIR}/{}.md", rule.name),
        sources(&rule.sources),
        vec![("paths".to_owned(), EmitValue::Seq(paths))],
        &rule.body,
        Diagnostics::new(),
    )
}
