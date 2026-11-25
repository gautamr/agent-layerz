use proptest::prelude::{Strategy, prop_assert_eq, proptest};

use super::{MarkdownDoc, MarkdownError};
use crate::diagnostic::Where;
use crate::render::yaml::{EmitMap, EmitValue, emit};
use crate::untrusted_text::Stripped;
use crate::yaml::{MAX_BYTES, Node, Value, YamlErrorKind};

fn parse(text: &str) -> Result<MarkdownDoc, MarkdownError> {
    MarkdownDoc::parse(text.as_bytes())
}

fn file() -> Where {
    Where::layer_file(
        Stripped::from_untrusted("personal"),
        Stripped::from_untrusted("agents/a.md"),
    )
}

fn line(error: MarkdownError) -> String {
    error.into_diagnostic(file()).to_string()
}

#[test]
fn a_file_with_a_block_has_its_mapping_and_the_text_after_the_closing_line_as_body() {
    let doc = parse("---\nname: a\ntools: [x]\n---\nbody\nmore\n").expect("valid");
    assert_eq!(doc.front().mapping().len(), 2);
    assert!(matches!(
        doc.front().get("name").map(Node::value),
        Some(Value::Text(name)) if name == "a"
    ));
    assert_eq!(doc.body().as_str(), "body\nmore\n");
}

#[test]
fn crlf_is_normalised_in_the_block_and_in_the_body() {
    let doc = parse("---\r\nname: a\r\n---\r\nbody\r\nmore\r\n").expect("valid");
    assert!(doc.front().get("name").is_some());
    assert_eq!(doc.body().as_str(), "body\nmore\n");
}

#[test]
fn a_file_without_an_opening_line_has_an_empty_mapping_and_the_whole_text_as_body() {
    for text in [
        "",
        "body\n",
        "---",
        "--- \nx\n",
        "\n---\nx: 1\n---\n",
        "----\nx: 1\n---\n",
        "# title\n---\nnot: front\n---\n",
    ] {
        let doc = parse(text).expect("valid");
        assert!(doc.front().is_empty(), "{text:?}");
        assert_eq!(doc.body().as_str(), text, "{text:?}");
    }
}

#[test]
fn the_text_of_a_file_without_a_block_has_crlf_as_lf_and_a_lone_carriage_return_kept() {
    let doc = parse("a\r\nb\rc\r\n").expect("valid");
    assert_eq!(doc.body().as_str(), "a\nb\rc\n");
}

#[test]
fn an_empty_block_and_a_block_of_comments_are_an_empty_mapping() {
    for text in [
        "---\n---\nbody",
        "---\n# only a comment\n---\nbody",
        "---\nnull\n---\nbody",
    ] {
        let doc = parse(text).expect("valid");
        assert!(doc.front().is_empty(), "{text:?}");
        assert_eq!(doc.body().as_str(), "body", "{text:?}");
    }
}

#[test]
fn the_closing_line_may_end_the_file_without_a_line_feed() {
    let doc = parse("---\na: 1\n---").expect("valid");
    assert_eq!(doc.body().as_str(), "");
}

#[test]
fn the_block_closes_at_the_first_line_that_is_exactly_three_dashes() {
    let doc = parse("---\na: |\n  ---x\n  ----\nb: 1\n---\n---\ntail\n").expect("valid");
    assert_eq!(doc.front().mapping().len(), 2);
    assert_eq!(doc.body().as_str(), "---\ntail\n");
}

#[test]
fn an_unclosed_block_is_a_frontmatter_error_naming_the_file() {
    for text in [
        "---\na: 1\n",
        "---\n",
        "---\na: 1\n--- \n",
        "---\r\na: 1\r\n----\r\n",
    ] {
        let error = parse(text).expect_err("not closed");
        assert_eq!(error, MarkdownError::NotClosed, "{text:?}");
        assert_eq!(
            line(error),
            "error [frontmatter] personal:agents/a.md: frontmatter is not closed with '---'"
        );
    }
}

#[test]
fn frontmatter_that_is_not_a_mapping_is_a_frontmatter_error_at_its_place_in_the_file() {
    for (text, place) in [
        ("---\n- a\n- b\n---\n", "2:1"),
        ("---\njust text\n---\n", "2:1"),
        ("---\n\n\n  42\n---\n", "4:3"),
        ("---\ntrue\n---\n", "2:1"),
    ] {
        let message = line(parse(text).expect_err("not a mapping"));
        assert_eq!(
            message,
            format!(
                "error [frontmatter] personal:agents/a.md:{place}: frontmatter must be a YAML mapping"
            ),
            "{text:?}"
        );
    }
}

#[test]
fn yaml_errors_in_the_block_keep_their_codes_and_count_lines_of_the_file() {
    let syntax = line(parse("---\na: 1\nb: [\n---\n").expect_err("bad yaml"));
    assert!(
        syntax.starts_with("error [yaml] personal:agents/a.md:"),
        "{syntax}"
    );
    let duplicate = line(parse("---\na: 1\nb: 2\na: 3\n---\n").expect_err("duplicate key"));
    assert_eq!(
        duplicate,
        "error [yaml] personal:agents/a.md:4:1: duplicate key 'a'"
    );
    let tag = line(parse("---\na: !custom x\n---\n").expect_err("tag"));
    assert!(
        tag.starts_with("error [yaml-tag] personal:agents/a.md:2:"),
        "{tag}"
    );
}

#[test]
fn the_yaml_budgets_apply_inside_the_block() {
    let uses: Vec<String> = (0..101).map(|n| format!("k{n}: *b\n")).collect();
    let aliases = format!("---\nbase: &b x\n{}---\n", uses.concat());
    assert!(matches!(
        parse(&aliases),
        Err(MarkdownError::Yaml(error)) if *error.kind() == YamlErrorKind::TooManyAliases
    ));
    let deep = format!("---\na: {}{}\n---\n", "[".repeat(70), "]".repeat(70));
    assert!(matches!(
        parse(&deep),
        Err(MarkdownError::Yaml(error)) if *error.kind() == YamlErrorKind::TooDeep
    ));
}

#[test]
fn the_cap_of_one_mib_counts_the_whole_file_that_holds_the_block() {
    let head = "---\na: 1\n---\n";
    let at_cap = format!("{head}{}", "x".repeat(MAX_BYTES - head.len()));
    assert_eq!(at_cap.len(), MAX_BYTES);
    assert!(parse(&at_cap).is_ok());
    let past_cap = format!("{at_cap}x");
    assert!(matches!(
        parse(&past_cap),
        Err(MarkdownError::Yaml(error)) if *error.kind() == YamlErrorKind::TooLarge
    ));
}

#[test]
fn a_file_without_a_block_has_no_one_mib_cap() {
    let text = "x".repeat(MAX_BYTES + 1);
    assert_eq!(
        parse(&text).expect("valid").body().as_str().len(),
        MAX_BYTES + 1
    );
}

#[test]
fn a_control_character_or_invalid_utf8_in_a_file_with_a_block_is_a_yaml_error() {
    assert!(matches!(
        parse("---\na: 1\n---\nbody \u{1b}\n"),
        Err(MarkdownError::Yaml(error)) if *error.kind() == YamlErrorKind::ControlCharacter
    ));
    assert!(matches!(
        MarkdownDoc::parse(b"---\na: 1\n---\n\xff"),
        Err(MarkdownError::Yaml(error)) if *error.kind() == YamlErrorKind::InvalidUtf8
    ));
}

/// §4.1 "Markdown frontmatter": a file with no block is under neither the 1 MiB cap nor the
/// control-character rule, so it may hold a control character, which a file with a block may not.
#[test]
fn a_file_without_a_block_may_hold_a_control_character() {
    for text in [
        "body \u{1b}[0m\n",
        "a\u{0}b",
        "# title\n\u{7}\n---\nx: 1\n---\n",
    ] {
        let doc = parse(text).expect("a control character is no error without a block");
        assert!(doc.front().is_empty(), "{text:?}");
        assert_eq!(doc.body().as_str(), text, "{text:?}");
    }
}

#[test]
fn a_file_without_a_block_that_is_not_utf8_is_refused() {
    assert_eq!(MarkdownDoc::parse(b"\xff"), Err(MarkdownError::NotUtf8));
    assert_eq!(
        MarkdownDoc::parse(b"\xff")
            .expect_err("refused")
            .into_diagnostic(file())
            .to_string(),
        "error [frontmatter] personal:agents/a.md: the file is not valid UTF-8"
    );
}

fn emit_value() -> impl Strategy<Value = EmitValue> {
    let text = "[ -~\n\t\u{e9}\u{65e5}]{0,12}".prop_map(EmitValue::Text);
    let leaf = proptest::prop_oneof![
        text,
        proptest::bool::ANY.prop_map(EmitValue::Bool),
        (-1_000_000_i64..1_000_000).prop_map(EmitValue::Int),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        proptest::prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(EmitValue::Seq),
            proptest::collection::btree_map("[a-z][a-z0-9_]{0,6}", inner, 0..4).prop_map(|map| {
                EmitValue::Map(EmitMap::from_entries(map.into_iter().collect()).expect("unique"))
            }),
        ]
    })
}

fn same(node: &Node, emitted: &EmitValue) -> bool {
    match (node.value(), emitted) {
        (Value::Text(read), EmitValue::Text(written)) => read == written,
        (Value::Bool(read), EmitValue::Bool(written)) => read == written,
        (Value::Int(read), EmitValue::Int(written)) => *read == i128::from(*written),
        (Value::Seq(read), EmitValue::Seq(written)) => {
            read.len() == written.len() && read.iter().zip(written).all(|(r, w)| same(r, w))
        }
        (Value::Map(read), EmitValue::Map(written)) => {
            read.len() == written.len()
                && read
                    .entries()
                    .zip(written.entries())
                    .all(|((key, value), (name, emitted))| {
                        matches!(key.value(), Value::Text(text) if text == name)
                            && same(value, emitted)
                    })
        }
        _ => false,
    }
}

proptest! {
    #[test]
    fn a_generated_document_reads_back_as_the_mapping_and_the_body_it_was_written_from(
        entries in proptest::collection::btree_map("[a-z][a-z0-9_]{0,6}", emit_value(), 0..5),
        body in "[ -~\n\u{e9}]{0,40}",
    ) {
        let map = EmitMap::from_entries(entries.into_iter().collect()).expect("unique keys");
        let block = emit(&EmitValue::Map(map.clone()));
        let text = format!("---\n{block}---\n{body}");
        let doc = parse(&text).expect("a generated document is valid");
        prop_assert_eq!(doc.body().as_str(), body.as_str());
        let read: Vec<(&Node, &Node)> = doc.front().mapping().entries().collect();
        prop_assert_eq!(read.len(), map.len());
        let all_same = read
            .iter()
            .zip(map.entries())
            .all(|((key, value), (name, emitted))| {
                matches!(key.value(), Value::Text(text) if text == name) && same(value, emitted)
            });
        prop_assert_eq!(all_same, true);
        // And the same file written with CRLF reads back alike.
        let crlf = parse(&text.replace('\n', "\r\n")).expect("valid with CRLF");
        prop_assert_eq!(crlf.body().as_str(), body.as_str());
    }

    #[test]
    fn any_bytes_give_a_document_or_an_error_of_one_line_and_never_a_panic(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..80)) {
        if let Err(error) = MarkdownDoc::parse(&bytes) {
            let shown = error.into_diagnostic(file()).to_string();
            prop_assert_eq!(shown.contains('\n'), false);
        }
    }
}
