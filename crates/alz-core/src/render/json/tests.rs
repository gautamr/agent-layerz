//! Tests of the emitter: every golden sample byte for byte, the
//! orders the spec fixes for keys, and the law that what alz writes alz reads.

use std::collections::BTreeMap;

use proptest::collection::{btree_map, vec};
use proptest::prelude::{Strategy, any, prop_assert_eq, prop_oneof, proptest};

use crate::json::{JsonNumber, JsonObject, JsonValue, parse};
use crate::render::json::emit;

/// The golden samples, each `{ name, input, json }`; see `tests/fixtures/json-emit/README.md`.
const SAMPLES: &str = include_str!("../../../tests/fixtures/json-emit/samples.json");

fn text(value: &JsonValue, key: &str) -> String {
    let JsonValue::Object(entries) = value else {
        panic!("an object");
    };
    match entries.get(key) {
        Some(JsonValue::Text(text)) => text.clone(),
        other => panic!("{key} is not text: {other:?}"),
    }
}

#[test]
fn every_sample_is_written_byte_for_byte() {
    let JsonValue::Array(samples) = parse(SAMPLES.as_bytes()).expect("the samples are JSON") else {
        panic!("a list of samples");
    };
    assert!(samples.len() >= 30, "{} samples", samples.len());
    for sample in &samples {
        let JsonValue::Object(entries) = sample else {
            panic!("a sample is an object");
        };
        let input = entries.get("input").expect("an input");
        assert_eq!(
            emit(input),
            text(sample, "json"),
            "{}",
            text(sample, "name")
        );
    }
}

fn object(entries: &[(&str, JsonValue)]) -> JsonValue {
    JsonValue::Object(
        JsonObject::from_entries(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        )
        .expect("unique keys"),
    )
}

fn int(number: i128) -> JsonValue {
    JsonValue::Number(JsonNumber::Int(number))
}

#[test]
fn keys_are_in_code_point_order_not_utf16_order() {
    // U+1F600 is above U+FFFF and U+FF5E is below it: by code point U+FF5E comes first, by UTF-16
    // unit U+1F600 would come first, which the spec does not do (TASK-64).
    let written = emit(&object(&[
        ("\u{1f600}", int(1)),
        ("\u{ff5e}", int(2)),
        ("a", int(3)),
    ]));
    assert_eq!(
        written,
        "{\n  \"a\": 3,\n  \"\u{ff5e}\": 2,\n  \"\u{1f600}\": 1\n}\n"
    );
}

#[test]
fn a_key_that_is_an_array_index_is_a_plain_string_in_name_order() {
    // Moving `10` and `9` before every other key, in numeric order, is not what the spec says
    // (TASK-64). The spec sorts them as strings: `-x` < `10` < `9` < `a`.
    let written = emit(&object(&[
        ("a", int(1)),
        ("9", int(2)),
        ("10", int(3)),
        ("-x", int(4)),
    ]));
    assert_eq!(
        written,
        "{\n  \"-x\": 4,\n  \"10\": 3,\n  \"9\": 2,\n  \"a\": 1\n}\n"
    );
}

#[test]
fn a_path_sorts_as_one_string() {
    let written = emit(&object(&[("a/b", int(1)), ("a-b", int(2)), ("a", int(3))]));
    assert_eq!(written, "{\n  \"a\": 3,\n  \"a-b\": 2,\n  \"a/b\": 1\n}\n");
}

#[test]
fn floats_are_written_as_the_shortest_decimal_that_reads_back() {
    let cases = [
        (0.0, "0"),
        (-0.0, "0"),
        (1.0, "1"),
        (-1.5, "-1.5"),
        (0.1, "0.1"),
        (100.0, "100"),
        (1e20, "100000000000000000000"),
        (1.234_567_890_123_456_8e20, "123456789012345680000"),
        (1e21, "1e+21"),
        (1.5e21, "1.5e+21"),
        (0.000_001, "0.000001"),
        (1e-7, "1e-7"),
        (1.5e-7, "1.5e-7"),
        (123_456.789, "123456.789"),
        (f64::MAX, "1.7976931348623157e+308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
        (5e-324, "5e-324"),
        (f64::NAN, "null"),
        (f64::INFINITY, "null"),
    ];
    for (float, expected) in cases {
        assert_eq!(
            emit(&JsonValue::Number(JsonNumber::Float(float))),
            format!("{expected}\n"),
            "{float:e}"
        );
    }
}

fn any_key() -> impl Strategy<Value = String> {
    "[a-z0-9 \"\\\\\u{e9}\u{ff5e}\u{e000}\u{1f600}-]{0,4}"
}

fn any_value() -> impl Strategy<Value = JsonValue> {
    let leaf = prop_oneof![
        any::<bool>().prop_map(JsonValue::Bool),
        any::<i64>().prop_map(|number| int(i128::from(number))),
        any::<String>().prop_map(JsonValue::Text),
    ];
    leaf.prop_recursive(4, 32, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..4).prop_map(JsonValue::Array),
            btree_map(any_key(), inner, 0..4)
                .prop_map(|entries| JsonValue::Object(JsonObject::from(entries))),
        ]
    })
}

/// `value` with every object's entries in name order: what reading what `emit` wrote gives back.
fn in_name_order(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Array(items) => JsonValue::Array(items.iter().map(in_name_order).collect()),
        JsonValue::Object(entries) => JsonValue::Object(JsonObject::from(
            entries
                .iter()
                .map(|(key, item)| (key.to_owned(), in_name_order(item)))
                .collect::<BTreeMap<_, _>>(),
        )),
        leaf @ (JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)) => leaf.clone(),
    }
}

proptest! {
    #[test]
    fn what_is_emitted_reads_back_as_the_same_value_in_name_order(value in any_value()) {
        let written = emit(&value);
        prop_assert_eq!(parse(written.as_bytes()), Ok(in_name_order(&value)));
        prop_assert_eq!(written.matches('\n').count() >= 1, true);
    }

    #[test]
    fn the_keys_of_an_object_come_out_in_code_point_order(
        keys in btree_map(any_key(), any::<bool>(), 0..8),
    ) {
        let object = JsonValue::Object(JsonObject::from(
            keys.iter().rev().map(|(key, flag)| (key.clone(), JsonValue::Bool(*flag))).collect::<BTreeMap<_, _>>(),
        ));
        let read = parse(emit(&object).as_bytes()).expect("emitted JSON is JSON");
        let JsonValue::Object(entries) = read else { panic!("an object") };
        let order: Vec<&str> = entries.iter().map(|(key, _)| key).collect();
        let mut by_code_point = order.clone();
        by_code_point.sort_by(|left, right| left.chars().cmp(right.chars()));
        prop_assert_eq!(order, by_code_point);
    }

    #[test]
    fn the_output_is_deterministic(value in any_value()) {
        prop_assert_eq!(emit(&value), emit(&value.clone()));
    }
}
