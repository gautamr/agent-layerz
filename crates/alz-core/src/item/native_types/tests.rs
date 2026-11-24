use std::collections::BTreeSet;

use crate::item::{
    AGENT_KEYS, CLAUDE_AGENT_FIELDS, CLAUDE_SETTINGS, KEYS, KeyRule, Step, native_key,
};
use crate::target::TargetName;

use crate::item::NativeKey;

use super::{Field, Shape};

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
        ("settings", CLAUDE_SETTINGS),
        ("agent", CLAUDE_AGENT_FIELDS),
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
    assert_eq!(paths("", CLAUDE_SETTINGS).len(), 147);
    assert_eq!(paths("", CLAUDE_AGENT_FIELDS).len(), 10);
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
    unique(CLAUDE_SETTINGS);
    unique(CLAUDE_AGENT_FIELDS);
}

#[test]
fn a_key_that_the_security_table_refuses_is_not_a_typed_key() {
    for path in paths("", CLAUDE_SETTINGS) {
        if path.contains('*') || path.contains('[') {
            continue;
        }
        let names: Vec<&str> = path.split('.').collect();
        let steps: Vec<Step<'_>> = names.iter().map(|name| Step::Key(name)).collect();
        let row = native_key(TargetName::Claude, &steps);
        assert!(
            !matches!(
                row.map(NativeKey::rule),
                Some(KeyRule::FirstClass(_) | KeyRule::Reserved(_))
            ),
            "{path} is refused by the security table, so it has no type"
        );
    }
}

#[test]
fn every_claude_floor_key_has_a_type() {
    let typed: BTreeSet<String> = paths("", CLAUDE_SETTINGS).into_iter().collect();
    for row in KEYS {
        if row.target() == TargetName::Claude && matches!(row.rule(), KeyRule::Floor(_)) {
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
    for field in CLAUDE_AGENT_FIELDS {
        assert!(!neutral.contains(&field.name()), "{}", field.name());
        assert!(
            !AGENT_KEYS
                .iter()
                .any(|row| row.target() == TargetName::Claude && row.key() == field.name()),
            "{}",
            field.name()
        );
    }
}
