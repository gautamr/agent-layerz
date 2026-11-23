use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

use crate::diagnostic::Position;
use crate::json::{
    JsonError, JsonErrorKind, JsonNumber, JsonObject, JsonValue, MAX_BYTES, MAX_DEPTH, parse,
};

fn read(text: &str) -> Result<JsonValue, JsonError> {
    parse(text.as_bytes())
}

fn nested(depth: usize) -> String {
    format!("{}{}", "[".repeat(depth), "]".repeat(depth))
}

fn kind_of(bytes: &[u8]) -> JsonErrorKind {
    parse(bytes)
        .expect_err("an invalid document")
        .kind()
        .clone()
}

#[test]
fn a_document_is_read_into_the_tree_in_document_order() {
    let value = read(r#"{"b": [1, -2, 1.5, true, null, "x"], "a": {}}"#).expect("valid");
    let JsonValue::Object(entries) = value else {
        panic!("an object");
    };
    assert_eq!(
        entries.iter().map(|(key, _)| key).collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(
        entries.get("b"),
        Some(&JsonValue::Array(vec![
            JsonValue::Number(JsonNumber::Int(1)),
            JsonValue::Number(JsonNumber::Int(-2)),
            JsonValue::Number(JsonNumber::Float(1.5)),
            JsonValue::Bool(true),
            JsonValue::Null,
            JsonValue::Text("x".to_owned()),
        ]))
    );
    assert_eq!(
        entries.get("a"),
        Some(&JsonValue::Object(JsonObject::default()))
    );
}

#[test]
fn an_integer_is_exact_up_to_the_range_of_u64_and_i64() {
    assert_eq!(
        read("18446744073709551615"),
        Ok(JsonValue::Number(JsonNumber::Int(i128::from(u64::MAX))))
    );
    assert_eq!(
        read("-9223372036854775808"),
        Ok(JsonValue::Number(JsonNumber::Int(i128::from(i64::MIN))))
    );
}

#[test]
fn one_mebibyte_is_read_and_one_byte_more_is_not() {
    let padded = |size: usize| {
        let mut bytes = b"[".to_vec();
        bytes.resize(size - 1, b' ');
        bytes.push(b']');
        bytes
    };
    assert!(parse(&padded(MAX_BYTES)).is_ok());
    assert_eq!(kind_of(&padded(MAX_BYTES + 1)), JsonErrorKind::TooLarge);
}

#[test]
fn text_that_is_not_utf_8_is_refused_at_the_first_bad_byte() {
    let error = parse(b"{\n  \"a\": \"\xff\"}").expect_err("invalid UTF-8");
    assert_eq!(error.kind(), &JsonErrorKind::InvalidUtf8);
    assert_eq!(error.position(), Position::new(2, 9));
}

#[test]
fn nesting_is_read_to_the_limit_of_serde_json_and_refused_below() {
    // serde_json's recursion limit is 128 and counts the root value as the first level, so 127
    // arrays or objects inside one another are read and the 128th is refused.
    assert_eq!(MAX_DEPTH, 127);
    assert!(read(&nested(127)).is_ok());
    assert_eq!(kind_of(nested(128).as_bytes()), JsonErrorKind::Syntax);
    assert_eq!(kind_of(nested(100_000).as_bytes()), JsonErrorKind::Syntax);
}

#[test]
fn text_that_is_not_one_document_is_a_syntax_error_with_a_position() {
    let cases = [
        ("", None),
        ("   ", None),
        ("{", None),
        ("[1,]", Position::new(1, 4)),
        ("{} {}", Position::new(1, 4)),
        ("{'a': 1}", Position::new(1, 2)),
        ("\u{feff}{}", Position::new(1, 1)),
        ("// c\n{}", Position::new(1, 1)),
        ("[NaN]", Position::new(1, 2)),
        ("\"a\nb\"", None),
    ];
    for (text, at) in cases {
        let error = read(text).expect_err(text);
        assert_eq!(error.kind(), &JsonErrorKind::Syntax, "{text:?}");
        if at.is_some() {
            assert_eq!(error.position(), at, "{text:?}");
        }
    }
}

#[test]
fn a_duplicate_key_is_an_error_at_any_depth_and_names_the_quoted_key() {
    for text in [
        r#"{"a":1,"a":2}"#,
        r#"{"x":{"a":1,"a":2}}"#,
        r#"[{"a":1},{"b":1,"b":1}]"#,
        r#"{"a":1,"a":2}"#,
    ] {
        let error = read(text).expect_err(text);
        let JsonErrorKind::DuplicateKey(key) = error.kind() else {
            panic!("a duplicate key: {text}");
        };
        assert!(error.position().is_some(), "{text}");
        assert!(["'a'", "'b'"].contains(&key.to_string().as_str()), "{key}");
    }
}

#[test]
fn a_lone_surrogate_escape_has_no_utf_8_form_and_is_refused() {
    assert_eq!(read(r#""😀""#), Ok(JsonValue::Text("\u{1f600}".to_owned())));
    for text in [
        r#""\ud800""#,
        r#""\udc00""#,
        r#""\ud800x""#,
        r#"{"\ud800":1}"#,
    ] {
        assert_eq!(
            read(text).expect_err(text).kind(),
            &JsonErrorKind::Syntax,
            "{text}"
        );
    }
}

#[test]
fn an_object_built_from_entries_refuses_a_repeated_key() {
    let twice = vec![
        ("a".to_owned(), JsonValue::Null),
        ("b".to_owned(), JsonValue::Null),
        ("a".to_owned(), JsonValue::Null),
    ];
    assert_eq!(
        JsonObject::from_entries(twice)
            .expect_err("a repeated key")
            .key(),
        "a"
    );
}

proptest! {
    /// Whatever bytes arrive, the answer is a value or an error: the parser never panics.
    #[test]
    fn any_bytes_give_a_value_or_an_error(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        prop_assert!(parse(&bytes).is_ok() || parse(&bytes).is_err());
    }

    #[test]
    fn text_in_a_string_reads_back_as_it_was_escaped(text in any::<String>()) {
        let quoted = crate::render::json::emit(&JsonValue::Text(text.clone()));
        prop_assert_eq!(read(&quoted), Ok(JsonValue::Text(text)));
    }
}
