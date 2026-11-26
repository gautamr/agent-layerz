//! `.claude/rules/<name>.md` (spec §8.1, target-facts §9): the globs as `paths:` in the
//! frontmatter, and the body.

use alz_adapter::{JsonValue, Model, PathRule};

use super::Made;
use super::frontmatter::{file, header_sources, text};

/// The folder of the rule files.
const RULES_DIR: &str = ".claude/rules";

/// The rules as files.
pub(super) fn render(model: &Model) -> Made {
    model
        .rules
        .iter()
        .map(one)
        .fold(Made::default(), Made::then)
}

fn one(rule: &PathRule) -> Made {
    let paths = rule.paths.iter().map(|glob| text(glob)).collect();
    file(
        &format!("{RULES_DIR}/{}.md", rule.name),
        header_sources(&rule.sources),
        vec![("paths".to_owned(), JsonValue::Array(paths))],
        &rule.body,
    )
}
