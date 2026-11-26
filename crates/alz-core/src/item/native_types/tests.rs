use std::collections::BTreeSet;

use alz_adapter::{Field, Shape};

use crate::item::{KeyRule, Step};
use crate::target::Registry;

/// Claude's typed settings keys, as its descriptor states them.
fn claude_settings() -> &'static [Field] {
    claude_table(|vocabulary| vocabulary.settings_keys)
}

/// Claude's typed agent fields, as its descriptor states them.
fn claude_agent_fields() -> &'static [Field] {
    claude_table(|vocabulary| vocabulary.agent_fields)
}

fn claude_table(
    pick: impl FnOnce(&alz_adapter::NativeVocabulary) -> Option<alz_adapter::TypedKeys>,
) -> &'static [Field] {
    pick(
        &Registry::builtin()
            .descriptor(crate::target::claude())
            .expect("the Claude adapter is built in")
            .vocabulary,
    )
    .expect("the Claude descriptor types the keys")
    .fields
}

/// Section 14 of the target facts, where every row of the tables is recorded.
fn recorded() -> &'static str {
    let facts = include_str!("../../../../../docs/target-facts.md");
    facts
        .split("\n## 14. ")
        .nth(1)
        .expect("docs/target-facts.md has a section 14")
}

/// The rows of `fields`: the key paths, a key below an object, `*` for the value of a map and
/// `[]` for the item of a list, each with its shape.
fn rows(prefix: &str, fields: &[Field]) -> Vec<(String, Shape)> {
    fields
        .iter()
        .flat_map(|field| {
            let path = if prefix.is_empty() {
                field.name().to_owned()
            } else {
                format!("{prefix}.{}", field.name())
            };
            let below = below(&path, field.shape());
            std::iter::once((path, field.shape())).chain(below)
        })
        .collect()
}

fn paths(prefix: &str, fields: &[Field]) -> Vec<String> {
    rows(prefix, fields)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

fn below(path: &str, shape: Shape) -> Vec<(String, Shape)> {
    match shape {
        Shape::Bool
        | Shape::False
        | Shape::Text
        | Shape::Whole
        | Shape::WholeIn(_)
        | Shape::Number
        | Shape::Fraction(_)
        | Shape::McpServerName
        | Shape::Choice(_) => Vec::new(),
        Shape::List(item) | Shape::NonEmptyList(item) => below(&format!("{path}[]"), *item),
        Shape::Map(item) => below(&format!("{path}.*"), *item),
        Shape::Object(fields) | Shape::Single(fields) => rows(path, fields),
        Shape::Either(options) => options
            .iter()
            .flat_map(|option| below(path, *option))
            .collect(),
    }
}

#[test]
fn every_key_of_the_tables_is_recorded_with_its_path_in_the_target_facts() {
    let facts = recorded();
    for (table, fields) in [
        ("settings", claude_settings()),
        ("agent", claude_agent_fields()),
    ] {
        for path in paths("", fields) {
            assert!(
                facts.contains(&format!("`{path}`")),
                "{table} key `{path}` is not recorded in section 14 of docs/target-facts.md"
            );
        }
    }
}

#[test]
fn the_tables_have_the_recorded_number_of_rows() {
    assert_eq!(paths("", claude_settings()).len(), 147);
    assert_eq!(paths("", claude_agent_fields()).len(), 10);
}

#[test]
fn a_key_name_is_listed_once_in_its_mapping() {
    fn unique(fields: &[Field]) {
        let names: BTreeSet<&str> = fields.iter().map(Field::name).collect();
        assert_eq!(names.len(), fields.len(), "{fields:?}");
        for field in fields {
            nested(field.shape());
        }
    }
    fn nested(shape: Shape) {
        match shape {
            Shape::Bool
            | Shape::False
            | Shape::Text
            | Shape::Whole
            | Shape::WholeIn(_)
            | Shape::Number
            | Shape::Fraction(_)
            | Shape::McpServerName
            | Shape::Choice(_) => {}
            Shape::List(item) | Shape::NonEmptyList(item) | Shape::Map(item) => nested(*item),
            Shape::Object(fields) | Shape::Single(fields) => unique(fields),
            Shape::Either(options) => options.iter().for_each(|option| nested(*option)),
        }
    }
    unique(claude_settings());
    unique(claude_agent_fields());
}

#[test]
fn a_key_that_the_security_table_refuses_is_not_a_typed_key() {
    for path in paths("", claude_settings()) {
        if path.contains('*') || path.contains('[') {
            continue;
        }
        let names: Vec<&str> = path.split('.').collect();
        let steps: Vec<Step<'_>> = names.iter().map(|name| Step::Key(name)).collect();
        let registry = Registry::builtin();
        let row = registry
            .key_rules(crate::target::claude())
            .expect("the Claude adapter is built in")
            .native_key(&steps);
        assert!(
            !matches!(
                row.map(|row| row.rule()),
                Some(KeyRule::FirstClass(_) | KeyRule::Reserved(_))
            ),
            "{path} is refused by the security table, so it has no type"
        );
    }
}

#[test]
fn every_claude_floor_key_has_a_type() {
    let typed: BTreeSet<String> = paths("", claude_settings()).into_iter().collect();
    let registry = Registry::builtin();
    let rules = registry
        .key_rules(crate::target::claude())
        .expect("the Claude adapter is built in");
    for row in rules.rows() {
        if matches!(row.rule(), KeyRule::Floor(_)) {
            assert!(
                typed.contains(row.path()),
                "the floor key {} has no type",
                row.path()
            );
        }
    }
}

#[test]
fn an_agent_field_is_neither_a_neutral_key_nor_a_first_class_key() {
    let neutral = [
        "name",
        "description",
        "tools",
        "model",
        "permissionMode",
        "skills",
        "isolation",
        "memory",
    ];
    let registry = Registry::builtin();
    let vocabulary = &registry
        .descriptor(crate::target::claude())
        .expect("the Claude adapter is built in")
        .vocabulary;
    let first_class = &vocabulary.agent_first_class_keys;
    assert_eq!(vocabulary.agent_neutral_keys, neutral);
    for field in claude_agent_fields() {
        assert!(!neutral.contains(&field.name()), "{}", field.name());
        assert!(
            !first_class.iter().any(|row| row.path == field.name()),
            "{}",
            field.name()
        );
    }
}
