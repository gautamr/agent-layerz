//! Tests of the emitter: every golden sample from the TypeScript reference byte for byte, the
//! law that what alz writes alz reads back, and the shape of the map type.

mod json;

use proptest::collection::vec;
use proptest::prelude::{Strategy, any, prop_assert_eq, prop_oneof, proptest};
use proptest::sample::select;

use crate::render::yaml::{DuplicateKey, EmitMap, EmitValue, emit};
use crate::yaml::{Node, Value, YamlText, parse};

use json::Json;

/// What `crates/alz-core/tools/yaml-emit-samples.mjs` wrote from the reference.
const SAMPLES: &str = include_str!("../../../tests/fixtures/yaml-emit/samples.json");

/// One golden sample.
struct Sample {
    name: String,
    class: String,
    input: EmitValue,
    yaml: String,
    /// Set for a sample whose reference output alz deliberately does not write.
    drift: Option<String>,
}

fn field<'a>(entries: &'a [(String, Json)], name: &str) -> &'a Json {
    &entries
        .iter()
        .find(|(key, _)| key == name)
        .unwrap_or_else(|| panic!("a sample has no {name}"))
        .1
}

fn text_of(json: &Json) -> String {
    match json {
        Json::Text(text) => text.clone(),
        other @ (Json::Bool(_) | Json::Int(_) | Json::List(_) | Json::Object(_)) => {
            panic!("not text: {other:?}")
        }
    }
}

fn emit_value(json: &Json) -> EmitValue {
    match json {
        Json::Text(text) => EmitValue::Text(text.clone()),
        Json::Bool(flag) => EmitValue::Bool(*flag),
        Json::Int(number) => EmitValue::Int(*number),
        Json::List(items) => EmitValue::Seq(items.iter().map(emit_value).collect()),
        Json::Object(entries) => EmitValue::Map(
            EmitMap::from_entries(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), emit_value(value)))
                    .collect(),
            )
            .expect("the reference wrote unique keys"),
        ),
    }
}

fn samples() -> Vec<Sample> {
    let Json::List(items) = json::parse(SAMPLES).expect("the sample file is JSON") else {
        panic!("the sample file is a list")
    };
    items
        .iter()
        .map(|item| {
            let Json::Object(entries) = item else {
                panic!("a sample is an object")
            };
            Sample {
                name: text_of(field(entries, "name")),
                class: text_of(field(entries, "class")),
                input: emit_value(field(entries, "input")),
                yaml: text_of(field(entries, "yaml")),
                drift: entries
                    .iter()
                    .find(|(key, _)| key == "drift")
                    .map(|(_, reason)| text_of(reason)),
            }
        })
        .collect()
}

#[test]
fn every_golden_sample_of_the_reference_is_reproduced_byte_for_byte() {
    let all = samples();
    let failures: Vec<String> = all
        .iter()
        .filter(|sample| sample.drift.is_none())
        .filter_map(|sample| {
            let written = emit(&sample.input);
            (written != sample.yaml).then(|| {
                format!(
                    "{}\n--- reference\n{}\n--- alz\n{written}\n",
                    sample.name, sample.yaml
                )
            })
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} samples differ; the first three:\n{}",
        failures.len(),
        all.len(),
        failures.iter().take(3).cloned().collect::<String>()
    );
}

#[test]
fn the_golden_samples_cover_every_scalar_class_and_the_structures() {
    let all = samples();
    let classes: std::collections::BTreeSet<&str> =
        all.iter().map(|sample| sample.class.as_str()).collect();
    for wanted in [
        "plain-words",
        "yaml-1-1-words",
        "core-schema-scalars",
        "indicators",
        "quotes",
        "multi-line",
        "document-markers",
        "control-characters",
        "unicode",
        "double-quoted-line-breaks",
        "keys",
        "indentation",
        "structure",
        "top-level",
        "reference-reads-differently",
    ] {
        assert!(classes.contains(wanted), "no sample of class {wanted}");
    }
    assert!(all.len() >= 700, "only {} samples", all.len());
}

// ---- reading back what is written ----

fn load(text: &str) -> Option<Node> {
    YamlText::from_bytes(text.as_bytes())
        .and_then(|text| parse(&text))
        .ok()
}

fn emit_value_of(node: &Node) -> EmitValue {
    match node.value() {
        Value::Text(text) => EmitValue::Text(text.clone()),
        Value::Bool(flag) => EmitValue::Bool(*flag),
        Value::Int(number) => EmitValue::Int(i64::try_from(*number).expect("a small integer")),
        Value::Seq(items) => EmitValue::Seq(items.iter().map(emit_value_of).collect()),
        Value::Map(mapping) => EmitValue::Map(
            EmitMap::from_entries(
                mapping
                    .entries()
                    .map(|(key, value)| match key.value() {
                        Value::Text(text) => (text.clone(), emit_value_of(value)),
                        other @ (Value::Null
                        | Value::Bool(_)
                        | Value::Int(_)
                        | Value::Float(_)
                        | Value::Seq(_)
                        | Value::Map(_)) => panic!("a key that is not text: {other:?}"),
                    })
                    .collect(),
            )
            .expect("the parser refuses duplicate keys"),
        ),
        other @ (Value::Null | Value::Float(_)) => panic!("alz never writes {other:?}"),
    }
}

#[test]
fn what_alz_writes_for_every_golden_input_reads_back_as_the_same_value() {
    // The reader refuses a decoded control character, U+2028 and U+2029 by design (spec §7.13), so
    // an input that holds one is not read back. Every other golden input is.
    let all = samples();
    let (refused, readable): (Vec<&Sample>, Vec<&Sample>) = all
        .iter()
        .partition(|sample| mentions_refused(&sample.input));
    let differing: Vec<&str> = readable
        .iter()
        .filter(|sample| {
            load(&emit(&sample.input))
                .map(|node| emit_value_of(&node))
                .as_ref()
                != Some(&sample.input)
        })
        .map(|sample| sample.name.as_str())
        .collect();
    assert!(
        differing.is_empty(),
        "not read back as written: {differing:#?}"
    );
    assert!(readable.len() > 500 && !refused.is_empty());
}

#[test]
fn a_sample_marked_as_drift_is_written_differently_from_the_reference() {
    let all = samples();
    let drifts: Vec<&Sample> = all.iter().filter(|sample| sample.drift.is_some()).collect();
    let same: Vec<&str> = drifts
        .iter()
        .filter(|sample| emit(&sample.input) == sample.yaml)
        .map(|sample| sample.name.as_str())
        .collect();
    assert!(
        same.is_empty(),
        "marked as drift but written as the reference does: {same:#?}"
    );
    assert!(drifts.len() > 30);
}

fn written(text: &str) -> String {
    emit(&EmitValue::Seq(vec![EmitValue::Text(text.to_owned())]))
}

#[test]
fn text_the_reference_leaves_plain_and_alz_would_read_as_a_number_is_quoted() {
    for text in ["0b101", "+0b1", "-0b1", "+0o17", "-0o17", "+0x1f", "-0x1f"] {
        assert_eq!(written(text), format!("- \"{text}\"\n"));
    }
    // What the core schema does not read as a number stays plain, as in the reference.
    for text in ["0B1", "0b", "0b12", "1_000", "0x", "017x"] {
        assert_eq!(written(text), format!("- {text}\n"));
    }
}

#[test]
fn text_that_starts_with_a_byte_order_mark_is_quoted() {
    assert_eq!(written("\u{feff}"), "- \"\u{feff}\"\n");
    assert_eq!(written("a\u{feff}"), "- a\u{feff}\n");
}

#[test]
fn text_of_only_spaces_and_line_feeds_is_quoted() {
    assert_eq!(written(" \n"), "- \"\\ \\n\"\n");
    assert_eq!(written("\n"), "- |+\n  \n");
}

fn refused_char(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}')
}

fn mentions_refused(value: &EmitValue) -> bool {
    match value {
        EmitValue::Text(text) => text.chars().any(refused_char),
        EmitValue::Bool(_) | EmitValue::Int(_) => false,
        EmitValue::Seq(items) => items.iter().any(mentions_refused),
        EmitValue::Map(map) => map
            .entries()
            .any(|(key, value)| key.chars().any(refused_char) || mentions_refused(value)),
    }
}

fn alphabet() -> impl Strategy<Value = char> {
    select(vec![
        'a', 'b', 'Z', '0', '1', '9', ' ', ' ', '\n', '\t', ':', '#', '-', '?', '"', '\'', '\\',
        '[', ']', '{', '}', ',', '.', 'e', 'x', 'o', '~', '%', '|', '>', '&', '*', '!', '@', '`',
        '+', '_', '=', 'é', '日', '👍', '\u{a0}', '\u{feff}',
    ])
}

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        vec(alphabet(), 0..10).prop_map(|chars| chars.into_iter().collect::<String>()),
        select(vec![
            "true",
            "false",
            "null",
            "~",
            "",
            "yes",
            "no",
            "123",
            "-1",
            "1.5",
            "1e3",
            ".inf",
            ".nan",
            "0x10",
            "0o17",
            "0b101",
            "017",
            "1_000",
            "12:30",
            "2024-01-01",
            "---",
            "...",
            "a\nb",
            "a\nb\n",
            "a\n\nb\n\n",
            " a\nb",
            "- a",
            "a: b",
            "a #b",
            "x\\y",
            "it's",
            "say \"hi\"",
            "<<",
        ])
        .prop_map(str::to_owned),
    ]
}

fn emit_values() -> impl Strategy<Value = EmitValue> {
    let leaf = prop_oneof![
        text().prop_map(EmitValue::Text),
        any::<bool>().prop_map(EmitValue::Bool),
        any::<i32>().prop_map(|number| EmitValue::Int(i64::from(number))),
    ];
    leaf.prop_recursive(4, 32, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..4).prop_map(EmitValue::Seq),
            vec((text(), inner), 0..4).prop_map(|entries| {
                let unique = entries.into_iter().fold(Vec::new(), |mut kept, entry| {
                    if kept
                        .iter()
                        .all(|(key, _): &(String, EmitValue)| *key != entry.0)
                    {
                        kept.push(entry);
                    }
                    kept
                });
                EmitValue::Map(EmitMap::from_entries(unique).expect("keys were made unique"))
            }),
        ]
    })
}

proptest! {
    #[test]
    fn emit_then_parse_gives_back_the_value(inner in emit_values()) {
        // alz writes a list or a mapping at the top. The reference writes a top-level block of
        // text with a leading space (`|1-` and the line at column 0) in a way no reader gets back.
        let value = EmitValue::Seq(vec![inner]);
        let written = emit(&value);
        let read = load(&written);
        prop_assert_eq!(read.as_ref().map(emit_value_of), Some(value), "wrote {:?}", written);
    }

    #[test]
    fn emit_is_deterministic_and_ends_in_one_line_feed(value in emit_values()) {
        let first = emit(&value);
        prop_assert_eq!(&first, &emit(&value.clone()));
        prop_assert_eq!(first.ends_with('\n'), true);
    }
}

#[test]
fn a_mapping_refuses_a_key_that_comes_twice_without_quoting_it_in_the_message() {
    let twice = EmitMap::from_entries(vec![
        ("a".to_owned(), EmitValue::Int(1)),
        ("b\u{1b}[31m".to_owned(), EmitValue::Int(2)),
        ("b\u{1b}[31m".to_owned(), EmitValue::Int(3)),
    ])
    .expect_err("a key comes twice");
    assert_eq!(twice, DuplicateKey("b\u{1b}[31m".to_owned()));
    assert_eq!(twice.key(), "b\u{1b}[31m");
    assert_eq!(twice.to_string(), "a mapping has the same key twice");
}

#[test]
fn a_mapping_keeps_the_order_it_is_given() {
    let map = EmitMap::from_entries(vec![
        ("b".to_owned(), EmitValue::Bool(true)),
        ("a".to_owned(), EmitValue::Text("x".to_owned())),
    ])
    .expect("unique keys");
    assert_eq!(emit(&EmitValue::Map(map)), "b: true\na: x\n");
    assert_eq!(emit(&EmitValue::Seq(Vec::new())), "[]\n");
}
