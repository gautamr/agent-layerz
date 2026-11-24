//! Tests of the emitter: every golden sample byte for byte, the
//! law that what alz writes alz reads back, and the shape of the map type.

mod json;

use proptest::collection::vec;
use proptest::prelude::{Strategy, any, prop_assert_eq, prop_oneof, proptest};
use proptest::sample::select;

use crate::render::yaml::{DuplicateKey, EmitFloat, EmitMap, EmitValue, emit};
use crate::yaml::{Node, Value, YamlText, parse};

use json::Json;

/// The golden samples; see `tests/fixtures/yaml-emit/README.md`.
const SAMPLES: &str = include_str!("../../../tests/fixtures/yaml-emit/samples.json");

/// One golden sample.
struct Sample {
    name: String,
    class: String,
    input: EmitValue,
    yaml: String,
    /// Set for a sample whose `yaml` is text that alz does not write (spec §10.2): the case is
    /// tested by the tests of the rule, over the sample's input.
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
        other @ (Json::Bool(_)
        | Json::Null
        | Json::Int(_)
        | Json::Float(_)
        | Json::List(_)
        | Json::Object(_)) => {
            panic!("not text: {other:?}")
        }
    }
}

fn emit_value(json: &Json) -> EmitValue {
    match json {
        Json::Text(text) => EmitValue::Text(text.clone()),
        Json::Bool(flag) => EmitValue::Bool(*flag),
        Json::Null => EmitValue::Null,
        Json::Int(number) => EmitValue::Int(*number),
        Json::Float(number) => EmitValue::Float(EmitFloat::new(*number)),
        Json::List(items) => EmitValue::Seq(items.iter().map(emit_value).collect()),
        Json::Object(entries) => EmitValue::Map(
            EmitMap::from_entries(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), emit_value(value)))
                    .collect(),
            )
            .expect("the samples have unique keys"),
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
fn every_golden_sample_is_reproduced_byte_for_byte() {
    let all = samples();
    let failures: Vec<String> = all
        .iter()
        .filter(|sample| sample.drift.is_none())
        .filter_map(|sample| {
            let written = emit(&sample.input);
            (written != sample.yaml).then(|| {
                format!(
                    "{}\n--- expected\n{}\n--- alz\n{written}\n",
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
        Value::Null => EmitValue::Null,
        Value::Int(number) => EmitValue::Int(i64::try_from(*number).expect("a small integer")),
        Value::Float(number) => EmitValue::Float(EmitFloat::new(*number)),
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

fn written(text: &str) -> String {
    emit(&EmitValue::Seq(vec![EmitValue::Text(text.to_owned())]))
}

#[test]
fn text_that_alz_would_read_as_a_number_is_quoted() {
    for text in ["0b101", "+0b1", "-0b1", "+0o17", "-0o17", "+0x1f", "-0x1f"] {
        assert_eq!(written(text), format!("- \"{text}\"\n"));
    }
    // What the core schema does not read as a number stays plain.
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

#[test]
fn text_with_a_c1_control_character_or_a_unicode_line_separator_is_written_as_escapes() {
    // TASK-73, spec §10.2 rule 1: these are never written raw.
    for (text, escape) in [
        ("\u{7f}", "\\x7f"),
        ("\u{80}", "\\x80"),
        ("\u{85}", "\\N"),
        ("\u{9f}", "\\x9f"),
        ("\u{2028}", "\\L"),
        ("\u{2029}", "\\P"),
    ] {
        assert_eq!(written(text), format!("- \"{escape}\"\n"));
        assert_eq!(written(&format!("a{text}b")), format!("- \"a{escape}b\"\n"));
        assert_eq!(
            emit(&value_map(&format!("a{text}b"))),
            format!("k: \"a{escape}b\"\n")
        );
        let as_key =
            EmitMap::from_entries(vec![(format!("a{text}b"), EmitValue::Int(1))]).expect("one key");
        assert_eq!(
            emit(&EmitValue::Map(as_key)),
            format!("\"a{escape}b\": 1\n")
        );
    }
    // U+00A0 is not one of them: it stays as it is.
    assert_eq!(written("a\u{a0}b"), "- a\u{a0}b\n");
}

#[test]
fn escaped_characters_are_never_written_raw_for_the_inputs_of_the_samples_that_hold_them() {
    let raw = |text: &str| {
        text.chars()
            .any(|c| matches!(c, '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}'))
    };
    let all = samples();
    let affected: Vec<&Sample> = all
        .iter()
        .filter(|sample| {
            sample.drift.as_deref() == Some("escaped-characters-the-reference-writes-raw")
        })
        .collect();
    assert!(affected.len() > 40, "only {} samples", affected.len());
    for sample in affected {
        let alz = emit(&sample.input);
        assert!(!raw(&alz), "{}: alz wrote one raw: {alz:?}", sample.name);
        // The reader refuses them even escaped (spec section 7.13).
        assert!(
            load(&alz).is_none(),
            "{}: the reader accepted {alz:?}",
            sample.name
        );
    }
}

#[test]
fn a_space_between_line_feeds_is_one_escaped_space_for_the_inputs_of_the_samples_that_hold_it() {
    let all = samples();
    let affected: Vec<&Sample> = all
        .iter()
        .filter(|sample| {
            sample.drift.as_deref() == Some("space-between-line-feeds-in-long-double-quoted-text")
        })
        .collect();
    assert!(affected.len() >= 10, "only {} samples", affected.len());
    for sample in &affected {
        let alz = emit(&sample.input);
        assert!(
            !alz.contains("\\\\ "),
            "{}: alz wrote one: {alz:?}",
            sample.name
        );
        if !mentions_refused(&sample.input) {
            assert_eq!(
                load(&alz).as_ref().map(emit_value_of).as_ref(),
                Some(&sample.input),
                "{}: not read back from {alz:?}",
                sample.name
            );
        }
    }
    assert!(
        affected
            .iter()
            .any(|sample| !mentions_refused(&sample.input)),
        "no sample can be read back"
    );
}

fn value_map(text: &str) -> EmitValue {
    EmitValue::Map(
        EmitMap::from_entries(vec![("k".to_owned(), EmitValue::Text(text.to_owned()))])
            .expect("one key"),
    )
}

#[test]
fn a_space_between_two_line_feeds_in_long_double_quoted_text_reads_back_as_written() {
    // TASK-73, spec §10.2: the space is written as one escaped space, not as a backslash twice and a space,
    // which reads back as a backslash. The text ends in a blank after its last line feed, so a
    // block cannot hold it and it is double-quoted; it has no character that alz's reader
    // refuses, so it reads back.
    for tail in ["\n \nb\n ", "\n \n\nb\n ", "\n\n \n \nb\n  ", " \n \nb\n "] {
        let text = format!("{}{tail}", "a".repeat(40));
        let value = value_map(&text);
        let written = emit(&value);
        assert_eq!(
            load(&written).as_ref().map(emit_value_of),
            Some(value),
            "wrote {written:?}"
        );
    }
}

#[test]
fn a_space_between_two_line_feeds_is_written_as_one_escaped_space() {
    // The case of TASK-73 as found: the text holds U+0001, which alz's reader refuses, so only
    // the bytes are checked. alz writes `\ `, never `\\ `.
    let written = emit(&value_map(&format!("{}\n \nb\u{1}", "a".repeat(40))));
    assert_eq!(
        written,
        format!("k: \"{}\n\n  \\ \n\n  b\\x01\"\n", "a".repeat(40))
    );
}

fn refused_char(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}')
}

fn mentions_refused(value: &EmitValue) -> bool {
    match value {
        EmitValue::Text(text) => text.chars().any(refused_char),
        EmitValue::Bool(_) | EmitValue::Int(_) | EmitValue::Float(_) | EmitValue::Null => false,
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

/// Text of 30 to 50 letters, then a run that puts a space between two line feeds or not, a few
/// characters, and an ending that is blank after a line feed or not (a block cannot keep that, so
/// the text is double-quoted). This is where the folding of long double-quoted text and alz's
/// reader meet (TASK-73), and where the plain generator seldom reaches.
fn long_text() -> impl Strategy<Value = String> {
    (
        30..50_usize,
        select(vec!["", "\n \n", "\n \n\n", " \n \n", "\n\n \n \n"]),
        vec(alphabet(), 0..6),
        select(vec!["", "\n ", "\n \n ", "\n\t", "\n  "]),
    )
        .prop_map(|(letters, middle, chars, end)| {
            format!(
                "{}{middle}{}{end}",
                "a".repeat(letters),
                chars.into_iter().collect::<String>()
            )
        })
}

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        vec(alphabet(), 0..10).prop_map(|chars| chars.into_iter().collect::<String>()),
        long_text(),
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

/// The number a node holds, whichever way the reader took it.
fn number_of(node: &Node) -> Option<f64> {
    match node.value() {
        Value::Float(number) => Some(*number),
        Value::Int(number) => format!("{number}").parse().ok(),
        Value::Null | Value::Bool(_) | Value::Text(_) | Value::Seq(_) | Value::Map(_) => None,
    }
}

proptest! {
    #[test]
    fn a_finite_float_reads_back_as_the_same_number(
        number in any::<f64>().prop_filter("finite, and not a zero, which reads back as the integer 0", |number| {
            number.is_finite() && number.classify() != std::num::FpCategory::Zero
        })
    ) {
        let written = emit(&EmitValue::Seq(vec![EmitValue::Float(EmitFloat::new(number))]));
        let read = load(&written).and_then(|node| match node.value() {
            Value::Seq(items) => items.first().and_then(number_of),
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_)
            | Value::Map(_) => None,
        });
        prop_assert_eq!(
            read.map(f64::to_bits),
            Some(number.to_bits()),
            "wrote {:?}",
            written
        );
    }

    #[test]
    fn emit_then_parse_gives_back_the_value(inner in emit_values()) {
        // alz writes a list or a mapping at the top: a top-level block of text with a leading space
        // (`|1-` and the line at column 0) is one no reader gets back.
        let value = EmitValue::Seq(vec![inner]);
        let written = emit(&value);
        let read = load(&written);
        prop_assert_eq!(read.as_ref().map(emit_value_of), Some(value), "wrote {:?}", written);
    }

    #[test]
    fn long_text_reads_back_as_written_whatever_its_blanks_and_line_feeds(text in long_text()) {
        // The case of TASK-73 where the reader accepts the text: a space between line feeds in
        // long double-quoted text (it ends in blanks after a line feed, which a block cannot keep).
        let value = value_map(&text);
        let written = emit(&value);
        prop_assert_eq!(
            load(&written).as_ref().map(emit_value_of),
            Some(value),
            "wrote {:?}",
            written
        );
    }

    #[test]
    fn text_with_a_character_alz_escapes_is_written_without_it_raw(
        before in text(),
        after in text(),
        escaped in select(vec!['\u{7f}', '\u{80}', '\u{85}', '\u{9f}', '\u{2028}', '\u{2029}']),
        inner in emit_values(),
    ) {
        // The reader refuses these characters even escaped (spec §7.13), so there is no round trip
        // to check; the escape keeps them out of the bytes on disk (TASK-73).
        let text = format!("{before}{escaped}{after}");
        let entry = EmitMap::from_entries(vec![
            (text.clone(), EmitValue::Text(text.clone())),
        ]).expect("one key");
        let value = EmitValue::Seq(vec![EmitValue::Map(entry), inner]);
        let written = emit(&value);
        prop_assert_eq!(
            written.chars().any(|c| matches!(c, '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}')),
            false,
            "wrote {:?}",
            written
        );
        prop_assert_eq!(load(&written), None, "the reader accepted {:?}", written);
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
