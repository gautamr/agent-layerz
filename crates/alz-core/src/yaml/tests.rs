//! Tests of the YAML entry point: each budget at its limit and one past it, every tag decision-28
//! names, decoded control characters, documents, aliases, numbers and the property that a
//! diagnostic holds no input text except one quoted key.

use proptest::prelude::{Strategy, any, prop_assert, proptest};
use proptest::sample::select;

use crate::diagnostic::{Code, Position, Where};
use crate::untrusted_text::Stripped;
use crate::yaml::fields::{FieldProblem, KeyPath, Section};
use crate::yaml::{Node, Value, YamlError, YamlErrorKind, YamlText, parse};

fn load(text: &str) -> Result<Node, YamlError> {
    YamlText::from_bytes(text.as_bytes()).and_then(|text| parse(&text))
}

fn kind_of(text: &str) -> Option<YamlErrorKind> {
    load(text).err().map(|error| error.kind().clone())
}

fn ok(text: &str) -> Node {
    load(text).unwrap_or_else(|error| panic!("{text:?} should parse, got {error}"))
}

fn entry<'a>(node: &'a Node, key: &str) -> &'a Value {
    let Value::Map(mapping) = node.value() else {
        panic!("not a mapping: {:?}", node.value())
    };
    mapping.get(key).expect("the key is there").value()
}

fn at(line: u32, column: u32) -> Position {
    Position::new(line, column).expect("a position counted from 1")
}

fn aliases_of(anchor: &str, count: usize) -> String {
    vec![format!("*{anchor}"); count].join(", ")
}

// ---- the four budgets and the cap, each at the limit and one past ----

#[test]
fn a_document_with_100_aliases_parses_and_101_is_refused() {
    let document = |count| format!("a: &x 1\nl: [{}]\n", aliases_of("x", count));
    assert!(load(&document(100)).is_ok());
    assert_eq!(kind_of(&document(101)), Some(YamlErrorKind::TooManyAliases));
}

#[test]
fn nesting_of_64_levels_parses_and_65_is_refused() {
    let nested = |levels: usize| format!("{}{}", "[".repeat(levels), "]".repeat(levels));
    assert!(load(&nested(64)).is_ok());
    assert_eq!(kind_of(&nested(65)), Some(YamlErrorKind::TooDeep));
}

#[test]
fn a_mapping_key_and_a_flow_sequence_count_as_two_levels() {
    // `a: [x]` is two levels, so 62 more mappings make 64 and one more makes 65.
    let block = |extra: usize| {
        let lines = (0..extra)
            .map(|level| format!("{}a:", " ".repeat(level)))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{lines}\n{}a: [x]\n", " ".repeat(extra))
    };
    assert!(load(&block(62)).is_ok());
    assert_eq!(kind_of(&block(63)), Some(YamlErrorKind::TooDeep));
}

#[test]
fn a_document_that_is_one_scalar_has_no_levels() {
    assert_eq!(*ok("x").value(), Value::Text("x".to_owned()));
}

#[test]
fn an_alias_counts_the_levels_of_its_anchor_where_it_is_used() {
    // The anchor is 33 levels deep; used at depth 32 it makes 65.
    let anchor = format!("{}{}", "[".repeat(33), "]".repeat(33));
    let used = |depth: usize| {
        format!(
            "a: &x {anchor}\nb: {}*x{}\n",
            "[".repeat(depth - 1),
            "]".repeat(depth - 1)
        )
    };
    assert!(load(&used(31)).is_ok());
    assert_eq!(kind_of(&used(33)), Some(YamlErrorKind::TooDeep));
}

#[test]
fn a_document_of_20000_nodes_parses_and_20001_is_refused() {
    let sequence = |scalars: usize| format!("[{}]", vec!["0"; scalars].join(","));
    assert!(
        load(&sequence(19_999)).is_ok(),
        "1 sequence and 19999 scalars"
    );
    assert_eq!(
        kind_of(&sequence(20_000)),
        Some(YamlErrorKind::TooManyNodes)
    );
}

#[test]
fn aliases_count_as_nodes_as_written() {
    let document = |aliases: usize| format!("a: &x 1\nl: [{}]\n", aliases_of("x", aliases));
    // 1 mapping, key, anchored scalar, key, sequence and the aliases: 5 + aliases nodes.
    assert_eq!(parse_nodes(&document(95)), 100);
}

fn parse_nodes(text: &str) -> usize {
    fn count(node: &Node) -> usize {
        1 + if let Value::Seq(items) = node.value() {
            items.iter().map(count).sum()
        } else if let Value::Map(mapping) = node.value() {
            mapping.entries().map(|(k, v)| count(k) + count(v)).sum()
        } else {
            0
        }
    }
    count(&ok(text))
}

fn anchored_sequence(name: &str, scalars: usize) -> String {
    format!("{name}: &{name} [{}]\n", vec!["0"; scalars].join(","))
}

#[test]
fn replayed_events_of_10000_parse_and_10001_are_refused() {
    // A sequence of 98 scalars is 100 events; 100 aliases of it replay 10000.
    let at_the_limit = format!(
        "{}l: [{}]\n",
        anchored_sequence("a", 98),
        aliases_of("a", 100)
    );
    assert!(load(&at_the_limit).is_ok());
    // 99 aliases of it replay 9900, and one alias of a 101-event sequence makes 10001.
    let past_the_limit = format!(
        "{}{}l: [{}, *b]\n",
        anchored_sequence("a", 98),
        anchored_sequence("b", 99),
        aliases_of("a", 99)
    );
    assert_eq!(
        kind_of(&past_the_limit),
        Some(YamlErrorKind::TooManyReplayedEvents)
    );
}

#[test]
fn a_document_at_every_limit_at_once_parses_through_serde_saphyrs_own_budgets() {
    // Written nodes: the root mapping, 4 keys, the anchored sequence and its 98 scalars, the
    // sequence of 100 aliases, 60 nested sequences, and the filler sequence with its scalars.
    // The 100 aliases replay 10000 events, so serde-saphyr sees 30000 nodes.
    let document = |filler: usize| {
        format!(
            "{}l: [{}]\nd: {}{}\nf: [{}]\n",
            anchored_sequence("a", 98),
            aliases_of("a", 100),
            "[".repeat(60),
            "]".repeat(60),
            vec!["0"; filler].join(",")
        )
    };
    let fixed = 1 + 4 + (1 + 98) + (1 + 100) + 60 + 1;
    assert!(load(&document(20_000 - fixed)).is_ok());
    assert_eq!(
        kind_of(&document(20_000 - fixed + 1)),
        Some(YamlErrorKind::TooManyNodes)
    );
}

#[test]
fn an_alias_bomb_that_nests_aliases_is_refused_by_what_it_expands_to() {
    let bomb = "a: &a [x, x]\nb: &b [*a, *a]\nc: &c [*b, *b]\nd: &d [*c, *c]\ne: &e [*d, *d]\n\
                f: &f [*e, *e]\ng: &g [*f, *f]\nh: &h [*g, *g]\ni: &i [*h, *h]\n\
                j: &j [*i, *i]\nk: &k [*j, *j]\nl: &l [*k, *k]\nm: &m [*l, *l]\nn: &n [*m, *m]\n\
                o: &o [*n, *n]\n";
    assert_eq!(kind_of(bomb), Some(YamlErrorKind::TooManyReplayedEvents));
}

#[test]
fn the_aliases_of_a_megabyte_are_refused_without_expanding_them() {
    let text = format!("a: &a x\nl: [{}]\n", aliases_of("a", 150_000));
    assert!(text.len() <= 1_048_576, "{}", text.len());
    assert_eq!(kind_of(&text), Some(YamlErrorKind::TooManyAliases));
}

#[test]
fn a_scalar_of_one_mebibyte_parses_through_the_whole_entry_point() {
    let text = format!("a: {}", "x".repeat(1_048_576 - 3));
    assert_eq!(text.len(), 1_048_576);
    assert!(load(&text).is_ok());
    assert_eq!(kind_of(&format!("{text}x")), Some(YamlErrorKind::TooLarge));
}

// ---- tags ----

#[test]
fn every_tag_outside_the_core_set_is_refused_with_yaml_tag() {
    for tag in [
        "!!binary aGk=",
        "!!timestamp 2020-01-01",
        "!!set {a}",
        "!!omap [a: 1]",
        "!!merge <<",
        "!!value x",
        "!foo x",
        "!!python/object x",
        "! x",
        "!<tag:example.com,2000:x> x",
    ] {
        let error = load(&format!("a: {tag}\n")).expect_err(tag);
        assert_eq!(error.kind(), &YamlErrorKind::Tag, "{tag}");
        assert_eq!(error.kind().code(), Code::YamlTag, "{tag}");
    }
}

#[test]
fn a_tag_on_a_key_or_a_collection_is_refused_too() {
    assert_eq!(kind_of("!foo a: 1\n"), Some(YamlErrorKind::Tag));
    assert_eq!(kind_of("a: !foo [1]\n"), Some(YamlErrorKind::Tag));
    assert_eq!(kind_of("a: !foo {b: 1}\n"), Some(YamlErrorKind::Tag));
}

#[test]
fn the_core_tags_are_accepted() {
    let node = ok(
        "s: !!str 1\ni: !!int 1\nf: !!float 1\nb: !!bool true\nn: !!null ~\nm: !!map {a: 1}\nq: !!seq [1]\n",
    );
    assert_eq!(*entry(&node, "s"), Value::Text("1".to_owned()));
    assert_eq!(*entry(&node, "i"), Value::Int(1));
    assert_eq!(*entry(&node, "b"), Value::Bool(true));
    assert_eq!(*entry(&node, "n"), Value::Null);
}

// ---- decoded text ----

#[test]
fn an_escape_that_decodes_to_a_forbidden_control_character_is_refused() {
    for escape in [
        r#""\x07""#,
        r#""\e""#,
        r#""\u2028""#,
        r#""\u2029""#,
        r#""\N""#,
        r#""\_ \r""#,
        r#""\0""#,
        r#""\x7f""#,
    ] {
        assert_eq!(
            kind_of(&format!("a: {escape}\n")),
            Some(YamlErrorKind::DecodedControlCharacter),
            "{escape}"
        );
    }
    assert_eq!(
        kind_of("\"\\x07\": 1\n"),
        Some(YamlErrorKind::DecodedControlCharacter),
        "keys are checked too"
    );
}

#[test]
fn the_escape_for_a_literal_space_and_tab_and_line_feed_are_accepted() {
    let node = ok(r#"a: "x\ty\nz \x20 \u00e9""#);
    assert_eq!(
        *entry(&node, "a"),
        Value::Text("x\ty\nz   \u{e9}".to_owned())
    );
}

#[test]
fn crlf_line_ends_parse_and_a_carriage_return_escape_does_not() {
    let node = ok("a: 1\r\nb: two\r\nc: |\r\n  x\r\n  y\r\n");
    assert_eq!(*entry(&node, "b"), Value::Text("two".to_owned()));
    assert_eq!(*entry(&node, "c"), Value::Text("x\ny\n".to_owned()));
    assert_eq!(
        kind_of("a: \"x\\ry\"\r\n"),
        Some(YamlErrorKind::DecodedControlCharacter)
    );
}

// ---- documents and aliases ----

#[test]
fn more_than_one_document_is_refused() {
    assert_eq!(
        kind_of("a: 1\n---\nb: 2\n"),
        Some(YamlErrorKind::MoreThanOneDocument)
    );
    assert_eq!(
        kind_of("---\na: 1\n...\n---\nb: 2\n"),
        Some(YamlErrorKind::MoreThanOneDocument)
    );
    assert!(load("---\na: 1\n...\n").is_ok());
}

#[test]
fn a_cyclic_alias_is_refused() {
    assert_eq!(kind_of("&a [*a]"), Some(YamlErrorKind::CyclicAlias));
    assert_eq!(
        kind_of("a: &a\n  b: *a\n"),
        Some(YamlErrorKind::CyclicAlias)
    );
    assert_eq!(
        kind_of("a: &a [&b [*a]]\n"),
        Some(YamlErrorKind::CyclicAlias)
    );
}

#[test]
fn an_unresolved_alias_is_refused_and_its_name_never_reaches_the_diagnostic() {
    let error = load("a: *marker\n").expect_err("unresolved");
    assert_eq!(error.kind(), &YamlErrorKind::UnresolvedAlias);
    let place = Where::repository_file(Stripped::from_untrusted("layer.yaml"));
    let line = error.into_diagnostic(place).to_string();
    assert!(!line.contains("marker"), "{line}");
    assert_eq!(
        line,
        "error [yaml] layer.yaml:1:4: an alias whose anchor is not defined"
    );
}

#[test]
fn an_alias_to_an_anchor_defined_after_it_is_unresolved() {
    assert_eq!(
        kind_of("a: *x\nb: &x 1\n"),
        Some(YamlErrorKind::UnresolvedAlias)
    );
}

#[test]
fn a_duplicate_key_is_refused_with_the_key_quoted_and_cut() {
    let error = load("a: 1\na: 2\n").expect_err("duplicate");
    assert_eq!(error.to_string(), "duplicate key 'a'");
    let long = "k".repeat(100);
    let error = load(&format!("{long}: 1\n{long}: 2\n")).expect_err("duplicate");
    assert_eq!(
        error.to_string(),
        format!("duplicate key '{}…'", "k".repeat(64))
    );
}

#[test]
fn text_that_is_not_yaml_is_a_syntax_error_at_its_position() {
    let error = load("a: [1,\nb: }\n").expect_err("not YAML");
    assert_eq!(error.kind(), &YamlErrorKind::Syntax);
    assert!(error.position().is_some());
}

#[test]
fn a_file_without_a_document_is_a_null_node() {
    assert_eq!(*ok("").value(), Value::Null);
    assert_eq!(*ok("# nothing\n").value(), Value::Null);
}

// ---- the tree ----

#[test]
fn scalars_follow_the_core_schema_and_quoted_scalars_stay_text() {
    let node = ok(concat!(
        "t: true\nq: \"true\"\nsq: 'true'\nyes: yes\nn: ~\ne:\nint: 17\nq16: \"16\"\n",
        "oct: 017\nbin: 0b101\nhex: 0x1F\nbig: 9007199254740993\nsci: 1e3\nfrac: 1.10\n",
        "under: 1_000\ninf: .inf\nlist: [a, 1]\n",
    ));
    assert_eq!(*entry(&node, "t"), Value::Bool(true));
    assert_eq!(*entry(&node, "q"), Value::Text("true".to_owned()));
    assert_eq!(*entry(&node, "sq"), Value::Text("true".to_owned()));
    assert_eq!(*entry(&node, "yes"), Value::Text("yes".to_owned()));
    assert_eq!(*entry(&node, "n"), Value::Null);
    assert_eq!(*entry(&node, "e"), Value::Null);
    assert_eq!(*entry(&node, "int"), Value::Int(17));
    assert_eq!(*entry(&node, "q16"), Value::Text("16".to_owned()));
    assert_ne!(*entry(&node, "oct"), Value::Int(15), "no legacy octal");
    assert_eq!(*entry(&node, "bin"), Value::Int(5));
    assert_eq!(*entry(&node, "hex"), Value::Int(31));
    assert_eq!(*entry(&node, "big"), Value::Int(9_007_199_254_740_993));
    assert_eq!(*entry(&node, "sci"), Value::Float(1000.0));
    assert_eq!(*entry(&node, "frac"), Value::Float(1.1));
    assert_eq!(*entry(&node, "under"), Value::Text("1_000".to_owned()));
    assert!(matches!(entry(&node, "inf"), Value::Float(number) if number.is_infinite()));
}

#[test]
fn merge_keys_are_ordinary_keys_and_a_mapping_keeps_document_order() {
    let node = ok("z: 1\n<<: {a: 1}\nb: 2\n");
    let Value::Map(mapping) = node.value() else {
        panic!("a mapping")
    };
    let keys: Vec<&Value> = mapping.entries().map(|(key, _)| key.value()).collect();
    assert_eq!(
        keys,
        [
            &Value::Text("z".to_owned()),
            &Value::Text("<<".to_owned()),
            &Value::Text("b".to_owned())
        ]
    );
    assert!(matches!(entry(&node, "<<"), Value::Map(_)));
}

#[test]
fn nodes_carry_the_position_they_were_read_at_and_an_alias_use_its_own() {
    let node = ok("a: &x [1]\nb:\n  - *x\n  - z\n");
    assert_eq!(node.at(), at(1, 1));
    let Value::Map(mapping) = node.value() else {
        panic!("a mapping")
    };
    let b = mapping.get("b").expect("b");
    assert_eq!(b.at(), at(3, 3));
    let Value::Seq(items) = b.value() else {
        panic!("a sequence")
    };
    assert_eq!(items.first().map(Node::at), Some(at(3, 5)), "the alias");
    assert_eq!(
        items.get(1).map(Node::at),
        Some(at(4, 5)),
        "the scalar after it"
    );
    let Value::Seq(replayed) = items.first().expect("alias").value() else {
        panic!("the replayed sequence")
    };
    assert_eq!(replayed.first().map(Node::at), Some(at(3, 5)));
}

// ---- fields ----

#[test]
fn fields_read_text_numbers_and_lists_and_report_paths_without_values() {
    let node = ok("ref: 1.10\nname: x\nn: 1e3\nm: 2.5\nl: [a, 2]\nbad: [a]\nextra: 1\n");
    let section = Section::new(&node, KeyPath::root()).expect("a mapping");
    assert_eq!(section.text("name").expect("text"), Some("x"));
    assert_eq!(section.text("missing").expect("absent"), None);
    let error = section.text("ref").expect_err("a number is not text");
    assert_eq!(error.problem(), &FieldProblem::NotText);
    assert_eq!(
        error.message().to_string(),
        "ref must be text; put the value in quotes to keep it as text"
    );
    assert_eq!(
        section.whole_number("n").expect("a whole float"),
        Some(1000)
    );
    assert!(section.whole_number("m").is_err());
    assert!(section.whole_number("name").is_err());
    let list = section.text_list("l").expect_err("an item is a number");
    assert_eq!(
        list.message().to_string(),
        "l[1] must be text; put the value in quotes to keep it as text"
    );
    assert_eq!(
        section.text_list("bad").expect("text list"),
        Some(vec!["a"])
    );
    assert!(section.text_list("name").is_err());
}

#[test]
fn unknown_fields_are_quoted_keys_in_document_order() {
    let node = ok("name: x\ncolo\u{202e}ur: 1\n'it''s': 2\n");
    let section = Section::new(&node, KeyPath::root().field("account")).expect("a mapping");
    let messages: Vec<String> = section
        .unknown_fields(&["name"])
        .iter()
        .map(|error| error.message().to_string())
        .collect();
    assert_eq!(
        messages,
        [
            "unknown field 'colour' in account",
            "unknown field 'it''s' in account"
        ]
    );
}

#[test]
fn a_section_that_is_not_a_mapping_is_an_error_at_its_path() {
    let node = ok("[a]");
    let error = Section::new(&node, KeyPath::root().field("stack").item(1)).expect_err("a list");
    assert_eq!(error.message().to_string(), "stack[1] must be a mapping");
}

// ---- the property: no input text in a diagnostic except one quoted key ----

const FRAGMENTS: &[&str] = &[
    "&SECRETanchor ",
    "*SECRETanchor",
    "*SECRETother",
    "!SECRETtag ",
    "!!SECRETglobal ",
    "!<tag:SECRET.example,1:x> ",
    "SECRETkey: ",
    "SECRETkey: 1\n",
    "[",
    "]",
    "{",
    "}",
    ", ",
    "\n",
    "- ",
    "  ",
    "---\n",
    "...\n",
    "\"\\x07SECRET\"",
    "\"SECRET\\e\"",
    "'SECRET'",
    "\"SECRET\\r\"",
    "SECRETvalue",
    ": ",
    "? ",
    "|\n  SECRET\n",
    "\u{7}SECRET",
    "\u{85}SECRET",
    "\r\n",
    "%YAML 1.1\n",
    "&a ",
    "*a",
    "<<: ",
    "!!binary ",
    "\"SECRET\"",
];

fn fragments() -> impl Strategy<Value = String> {
    proptest::collection::vec(select(FRAGMENTS), 0..14).prop_map(|parts| parts.concat())
}

fn bytes_and_fragments() -> impl Strategy<Value = Vec<u8>> {
    proptest::prop_oneof![
        fragments().prop_map(String::into_bytes),
        (fragments(), any::<u8>()).prop_map(|(text, byte)| {
            let mut bytes = text.into_bytes();
            bytes.push(byte);
            bytes.extend_from_slice(b"SECRET");
            bytes
        }),
        proptest::collection::vec(any::<u8>(), 0..64),
    ]
}

proptest! {
    #[test]
    fn a_rejected_input_leaves_no_input_text_in_its_diagnostic_except_one_quoted_key(
        bytes in bytes_and_fragments()
    ) {
        let result = YamlText::from_bytes(&bytes).and_then(|text| parse(&text));
        if let Err(error) = result {
            let place = Where::layer_file(
                Stripped::from_untrusted("team"),
                Stripped::from_untrusted("layer.yaml"),
            );
            let key = if let YamlErrorKind::DuplicateKey(Some(key)) = error.kind() {
                Some(key.to_string())
            } else {
                None
            };
            let line = error.clone().into_diagnostic(place).to_string();
            let without_the_key = key.as_ref().map_or(line.clone(), |key| line.replacen(key.as_str(), "", 1));
            prop_assert!(!without_the_key.contains("SECRET"), "{without_the_key}");
            // What is left is the fixed prefix, the fixed message and nothing from the input.
            let fixed = error.kind().message().to_string();
            let expected_tail = key.as_ref().map_or(fixed.clone(), |key| fixed.replacen(key.as_str(), "", 1));
            prop_assert!(without_the_key.ends_with(&expected_tail));
            prop_assert!(line.chars().all(|character| !character.is_control()));
        }
    }

    #[test]
    fn a_text_that_parses_holds_only_what_the_tree_types_allow(bytes in bytes_and_fragments()) {
        if let Ok(node) = YamlText::from_bytes(&bytes).and_then(|text| parse(&text)) {
            fn clean(node: &Node) -> bool {
                if let Value::Text(text) = node.value() {
                    text.chars().all(|c| !crate::yaml::text::is_forbidden_when_decoded(c))
                } else if let Value::Seq(items) = node.value() {
                    items.iter().all(clean)
                } else if let Value::Map(mapping) = node.value() {
                    mapping.entries().all(|(k, v)| clean(k) && clean(v))
                } else {
                    true
                }
            }
            prop_assert!(clean(&node));
        }
    }
}
