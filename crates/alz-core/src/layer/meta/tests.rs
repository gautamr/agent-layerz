//! Tests of `layer.yaml` as validated types: the account field's schema table (test 29) and the
//! commit identity tables (test 26) with the same verdicts, the messages of unknown fields and of
//! a number in a text field, every error of one file at once, and the property that no error
//! holds a refused value.

use proptest::prelude::{prop_assert, proptest};

use crate::account::{AccountChoice, AccountDeclaration, AccountName};
use crate::identity::{EmailAddress, IdentityName};
use crate::layer::{Description, ForbidPatternText, Kind, LayerMeta, LayerVersion, Visibility};
use crate::schema::SchemaErrors;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;
use crate::yaml::{Node, YamlErrorKind, YamlText, parse};

const BASE: &str = "name: personal\nkind: personal\nvisibility: public\n";

fn tree(document: &str) -> Node {
    let text = YamlText::from_bytes(document.as_bytes()).expect("the test document is text");
    parse(&text).expect("the test document is YAML")
}

fn read(document: &str) -> Result<LayerMeta, SchemaErrors> {
    LayerMeta::from_yaml(&tree(document))
}

fn with(extra: &str) -> Result<LayerMeta, SchemaErrors> {
    read(&format!("{BASE}{extra}\n"))
}

fn messages(errors: &SchemaErrors) -> Vec<String> {
    errors
        .iter()
        .map(|error| error.message().to_string())
        .collect()
}

/// Everything a person would see for these errors, as the lint command prints it.
fn printed(errors: &SchemaErrors) -> String {
    errors
        .diagnostics(&Stripped::from_untrusted("personal"))
        .to_string()
}

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("a valid name")
}

// ---- account field: schema (test 29) ----

#[test]
fn account_accepts_what_test_29_accepts() {
    let named = |text| AccountDeclaration::Every(AccountChoice::Named(name(text)));
    let cases = [
        (r#"account: "client-a""#, named("client-a")),
        (
            r#"account: "default""#,
            AccountDeclaration::Every(AccountChoice::Default),
        ),
        (
            &format!(r#"account: "{}""#, "a".repeat(64)),
            named(&"a".repeat(64)),
        ),
        (r#"account: "2client""#, named("2client")),
    ];
    for (yaml, expected) in cases {
        assert_eq!(with(yaml).expect(yaml).account(), Some(&expected), "{yaml}");
    }
}

#[test]
fn account_per_target_keeps_each_target_and_leaves_the_rest_undecided() {
    let both = with(r#"account: {"claude": "client-a", "codex": "client-a-api"}"#).expect("valid");
    let declared = both.account().expect("declared");
    assert_eq!(
        declared.choice_for(TargetName::Claude),
        Some(&AccountChoice::Named(name("client-a")))
    );
    assert_eq!(
        declared.choice_for(TargetName::Codex),
        Some(&AccountChoice::Named(name("client-a-api")))
    );

    let one = with(r#"account: {"codex": "client-a"}"#).expect("valid");
    let declared = one.account().expect("declared");
    assert_eq!(declared.choice_for(TargetName::Claude), None);
    assert!(declared.choice_for(TargetName::Codex).is_some());
}

#[test]
fn a_name_for_every_target_decides_each_target() {
    let meta = with(r#"account: "client-a""#).expect("valid");
    let declared = meta.account().expect("declared");
    for target in TargetName::ALL {
        assert_eq!(
            declared.choice_for(target),
            Some(&AccountChoice::Named(name("client-a")))
        );
    }
}

#[test]
fn default_in_a_map_is_the_default_account() {
    let meta = with(r#"account: {"claude": "default"}"#).expect("valid");
    assert_eq!(
        meta.account()
            .and_then(|account| account.choice_for(TargetName::Claude)),
        Some(&AccountChoice::Default)
    );
}

#[test]
fn account_refuses_what_test_29_refuses_and_prints_no_value() {
    let sixty_five = "a".repeat(65);
    let cases: Vec<(&str, String)> = vec![
        ("65 characters", format!("\"{sixty_five}\"")),
        ("upper case", r#""Client-A""#.into()),
        ("a leading '-'", r#""-leading""#.into()),
        ("a home path", r#""~/secret-dir""#.into()),
        ("an absolute path", r#""/etc/secret-dir""#.into()),
        ("a parent path", r#""../secret-dir""#.into()),
        ("a slash", r#""a/secret-dir""#.into()),
        ("a token-like value", r#""AKIAIOSFODNN7EXAMPLE""#.into()),
        ("a space", r#""client a""#.into()),
        ("an empty string", r#""""#.into()),
        (
            "a map with an unknown target",
            r#"{"cursor": "secret-dir"}"#.into(),
        ),
        ("a map with a path", r#"{"claude": "~/secret-dir"}"#.into()),
        (
            "a map with a name too long",
            format!("{{\"codex\": \"{sixty_five}\"}}"),
        ),
        ("an empty map", "{}".into()),
        ("null", "null".into()),
        ("a number", "5".into()),
        ("a boolean", "true".into()),
        ("a list", r#"["secret-dir"]"#.into()),
        (
            "a nested map",
            r#"{"claude": {"name": "secret-dir"}}"#.into(),
        ),
        ("a map with a null name", r#"{"claude": null}"#.into()),
    ];
    for (label, value) in &cases {
        let errors = with(&format!("account: {value}")).expect_err(label);
        let text = printed(&errors);
        assert!(
            text.starts_with("error [layer] personal:layer.yaml:"),
            "{label}: {text}"
        );
        assert!(text.contains("account"), "{label}: {text}");
        for refused in [
            "secret-dir",
            "AKIAIOSFODNN7EXAMPLE",
            "Client-A",
            "client a",
            &sixty_five,
        ] {
            assert!(!text.contains(refused), "{label}: {text}");
        }
    }
}

#[test]
fn account_errors_say_what_is_wrong_with_the_path_of_the_field() {
    let said = |value: &str| messages(&with(&format!("account: {value}")).expect_err(value));
    let rule = "must be an account name: lower-case letters, digits and '-', starting with a letter or digit, at most 64 characters";
    assert_eq!(said(r#""Client-A""#), [format!("account {rule}")]);
    assert_eq!(
        said(r#"{"codex": "x/y"}"#),
        [format!("account.codex {rule}")]
    );
    assert_eq!(said(r#"{"claude": null}"#), ["account.claude must be text"]);
    assert_eq!(said("{}"), ["account must not be an empty mapping"]);
    assert_eq!(
        said("[a]"),
        ["account must be an account name or a mapping of target to name"]
    );
    assert_eq!(
        said(r#"{"cursor": "x"}"#),
        ["unknown field 'cursor' in account"]
    );
}

// ---- commit identity (test 26) ----

fn identity_document(name: &str, email: &str) -> String {
    format!(
        "{BASE}commit_identity:\n  name: {}\n  email: {}\n",
        serde_json_string(name),
        serde_json_string(email)
    )
}

fn identity(name: &str, email: &str) -> Result<LayerMeta, SchemaErrors> {
    read(&identity_document(name, email))
}

/// A JSON string, which is a YAML double-quoted string, with every non-ASCII character escaped.
fn serde_json_string(text: &str) -> String {
    let body: String = text
        .chars()
        .flat_map(|character| match character {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            ' '..='~' => vec![character],
            other => format!("\\u{:04x}", u32::from(other)).chars().collect(),
        })
        .collect();
    format!("\"{body}\"")
}

#[test]
fn a_commit_identity_that_test_26_accepts_is_read() {
    let meta = identity("Owner O'Example", "owner@example.com").expect("valid");
    let declared = meta.commit_identity().expect("declared");
    assert_eq!(declared.name().as_str(), "Owner O'Example");
    assert_eq!(declared.email().as_str(), "owner@example.com");
    assert!(identity("Owner", "a.b+c%d_e-f@sub.example-1.com").is_ok());
    assert!(identity(&"a".repeat(128), "owner@example.com").is_ok());
}

type IdentityCase = (&'static str, String, String);

const NAME_OK: &str = "Zqowner";
const MAIL_OK: &str = "zqowner@example.com";

fn name_cases() -> Vec<IdentityCase> {
    let row = |label, name: &str| (label, name.to_owned(), MAIL_OK.to_owned());
    vec![
        row("a newline in the name", "Zqown\ner"),
        row("a control character in the name", "Zqown\u{7}er"),
        row("a C1 control character in the name", "Zqown\u{85}er"),
        row("a line separator in the name", "Zqown\u{2028}er"),
        row("a zero-width space in the name", "Zqown\u{200b}er"),
        row("a bidi control in the name", "Zqown\u{202e}er"),
        row("a soft hyphen in the name", "Zqown\u{ad}er"),
        row("a name over 128 characters", &"Zq".repeat(65)),
        row("an option-like name", "--zq-upload-pack=x"),
        row("a name with a leading space", " Zqowner"),
        row("a name with a trailing space", "Zqowner "),
        row("a name ending in a period", "Zqowner."),
        row("a name starting with a period", ".Zqowner"),
        row("a name ending in a comma", "Zqowner,"),
        row("a name ending in a semicolon", "Zqowner;"),
        row("a name ending in a colon", "Zqowner:"),
        row("a name starting with a double quote", "\"Zqowner"),
        row("a name ending in an apostrophe", "Zqowner'"),
        row("a name with a backslash", "Zqown\\er"),
        row("a name with a backslash and an apostrophe", "Zqowner\\'x"),
        row("a name with <", "Zqown<er"),
        row("a name with >", "Zqown>er"),
        row("a name with a backtick", "Zqown`er"),
    ]
}

fn email_cases() -> Vec<IdentityCase> {
    let row = |label, email: &str| (label, NAME_OK.to_owned(), email.to_owned());
    vec![
        row("an option-like email", "-zq@example.com"),
        row("an email starting with a period", ".zq@example.com"),
        row("an email with a zero-width space", "zq@exam\u{200b}ple.com"),
        row("an email without @", "zqowner.example.com"),
        row(
            "an email over 254 characters",
            &format!("zqlong{}@example.com", "a".repeat(250)),
        ),
    ]
}

#[test]
fn a_commit_identity_that_test_26_refuses_is_refused_and_no_value_is_printed() {
    let cases: Vec<IdentityCase> = name_cases().into_iter().chain(email_cases()).collect();
    for (label, name, email) in &cases {
        let document = identity_document(name, email);
        let text = YamlText::from_bytes(document.as_bytes()).expect("the test document is text");
        match parse(&text) {
            // A control character that an escape spells is refused by the YAML entry point
            // already (§7.13); the identity type refuses it too, so the verdict is the same.
            Err(error) => {
                assert_eq!(
                    error.kind(),
                    &YamlErrorKind::DecodedControlCharacter,
                    "{label}"
                );
                assert!(
                    IdentityName::try_from(name.as_str()).is_err()
                        || EmailAddress::try_from(email.as_str()).is_err(),
                    "{label}"
                );
            }
            Ok(node) => {
                let errors = LayerMeta::from_yaml(&node).expect_err(label);
                let text = printed(&errors);
                assert!(text.contains("commit_identity."), "{label}: {text}");
                for value in ["Zq", "zq"] {
                    assert!(!text.contains(value), "{label}: {text}");
                }
            }
        }
    }
}

#[test]
fn a_commit_identity_value_that_is_not_text_says_to_quote_it() {
    let name_is_number = read(&format!(
        "{BASE}commit_identity:\n  name: 12\n  email: owner@example.com\n"
    ))
    .expect_err("a number");
    assert_eq!(
        messages(&name_is_number),
        ["commit_identity.name must be text; put the value in quotes to keep it as text"]
    );
    let email_is_bool = read(&format!(
        "{BASE}commit_identity:\n  name: Owner\n  email: true\n"
    ))
    .expect_err("a boolean");
    assert_eq!(
        messages(&email_is_bool),
        ["commit_identity.email must be text"]
    );
}

#[test]
fn a_commit_identity_has_exactly_the_keys_name_and_email() {
    let extra = read(&format!(
        "{BASE}commit_identity:\n  name: Owner\n  email: owner@example.com\n  signing_key: abc\n"
    ))
    .expect_err("an extra key");
    assert_eq!(
        messages(&extra),
        ["unknown field 'signing_key' in commit_identity"]
    );

    let no_email = read(&format!("{BASE}commit_identity:\n  name: Owner\n")).expect_err("no email");
    assert_eq!(messages(&no_email), ["commit_identity.email is required"]);

    let not_a_map = with("commit_identity: Owner").expect_err("text");
    assert_eq!(messages(&not_a_map), ["commit_identity must be a mapping"]);
}

// ---- unknown fields, number in text, all errors at once ----

#[test]
fn unknown_fields_are_reported_as_quoted_keys_and_never_with_their_values() {
    let errors = with("colour: SECRETVALUE\nit's: 2\nstack_x: [SECRETVALUE]").expect_err("unknown");
    assert_eq!(
        messages(&errors),
        [
            "unknown field 'colour'",
            "unknown field 'it''s'",
            "unknown field 'stack_x'"
        ]
    );
    assert!(!printed(&errors).contains("SECRETVALUE"));
}

#[test]
fn an_unknown_key_is_stripped_and_cut_like_every_quoted_key() {
    let long = "k".repeat(70);
    let errors = with(&format!("\"a\\u202eb\": 1\n{long}: 1")).expect_err("unknown");
    assert_eq!(
        messages(&errors),
        [
            "unknown field 'ab'".to_owned(),
            format!("unknown field '{}…'", "k".repeat(64))
        ]
    );
}

#[test]
fn a_number_in_a_text_field_gets_the_quoting_hint() {
    let errors = with("version: 1.10").expect_err("a number");
    assert_eq!(
        messages(&errors),
        ["version must be text; put the value in quotes to keep it as text"]
    );
    assert_eq!(
        with("version: \"1.10\"")
            .expect("quoted")
            .version()
            .map(|v| v.as_str().to_owned()),
        Some("1.10".to_owned())
    );
    let description = with("description: true").expect_err("a boolean");
    assert_eq!(messages(&description), ["description must be text"]);
}

#[test]
fn every_field_error_of_one_file_is_reported_at_once_in_document_order() {
    let document = "colour: x\n\
        name: \"Bad Name\"\n\
        kind: \"\"\n\
        visibility: secret\n\
        version: 1.10\n\
        provides_slots: skill\n\
        forbid_patterns: [a, 3]\n\
        commit_identity:\n  name: \"Own<er\"\n  email: nope\n\
        account: \"~/x\"\n";
    let errors = read(document).expect_err("many problems");
    assert_eq!(
        messages(&errors),
        [
            "unknown field 'colour'",
            "name must be lower-case letters and digits in groups joined by a single '-' or '.'",
            "kind must not be empty",
            "visibility must be public, private or confidential",
            "version must be text; put the value in quotes to keep it as text",
            "provides_slots must be a list",
            "forbid_patterns[1] must be text; put the value in quotes to keep it as text",
            "commit_identity.name must not contain '<', '>', a backtick or a backslash",
            "commit_identity.email must be a plain ASCII email address",
            "account must be an account name: lower-case letters, digits and '-', starting with a letter or digit, at most 64 characters",
        ]
    );
    let lines: Vec<String> = errors
        .iter()
        .map(|error| error.position().to_string())
        .collect();
    assert_eq!(
        lines,
        [
            "1:1", "2:7", "3:7", "4:13", "5:10", "6:17", "7:22", "9:9", "10:10", "11:10"
        ]
    );
}

#[test]
fn every_error_becomes_a_layer_diagnostic_at_the_layer_file() {
    let errors = with("version: 1.10").expect_err("a number");
    assert_eq!(
        printed(&errors),
        "error [layer] personal:layer.yaml:4:10: version must be text; put the value in quotes to keep it as text\n"
    );
}

#[test]
fn required_fields_that_are_missing_are_reported_together() {
    let errors = read("description: x\n").expect_err("nothing required is there");
    assert_eq!(
        messages(&errors),
        [
            "name is required",
            "kind is required",
            "visibility is required"
        ]
    );
    let empty = read("").expect_err("an empty file is not a layer");
    assert_eq!(messages(&empty), ["the document must be a mapping"]);
    let list = read("- a\n").expect_err("a list");
    assert_eq!(messages(&list), ["the document must be a mapping"]);
}

#[test]
fn a_stack_manifest_is_not_a_layer() {
    let errors = read("stack: [path:../a]\ntargets: [claude]\n").expect_err("a manifest");
    assert_eq!(
        messages(&errors),
        ["this file has a 'stack' field, so it is a repository's stack manifest and not a layer"]
    );
}

#[test]
fn a_whole_layer_is_read_into_its_fields() {
    let meta = read(
        "name: client.team-1\nkind: client\nvisibility: confidential\nversion: \"1.4.0\"\n\
         description: Standards\nprovides_slots: [a, b]\nrequires_slots: [c]\n\
         forbid_patterns: [\"(?i)acme\"]\n",
    )
    .expect("valid");
    assert_eq!(meta.name().as_str(), "client.team-1");
    assert_eq!(meta.kind(), &Kind::Client);
    assert_eq!(meta.visibility(), Visibility::Confidential);
    assert_eq!(meta.version().map(LayerVersion::as_str), Some("1.4.0"));
    assert_eq!(
        meta.description().map(Description::as_str),
        Some("Standards")
    );
    assert_eq!(meta.provides_slots().len(), 2);
    assert_eq!(meta.requires_slots().len(), 1);
    assert_eq!(
        meta.forbid_patterns()
            .first()
            .map(ForbidPatternText::as_str),
        Some("(?i)acme")
    );
    assert!(meta.commit_identity().is_none());
    assert!(meta.account().is_none());
}

#[test]
fn a_layer_without_the_optional_fields_has_empty_lists() {
    let meta = read(BASE).expect("valid");
    assert!(meta.version().is_none() && meta.description().is_none());
    assert!(meta.provides_slots().is_empty() && meta.forbid_patterns().is_empty());
}

// ---- no error holds a refused value ----

proptest! {
    #[test]
    fn no_schema_error_holds_a_refused_account_value(tail in "[ -~]{0,20}") {
        let value = format!("QZX9{tail}").replace('\'', "''");
        let errors = with(&format!("account: '{value}'")).expect_err("upper case is refused");
        prop_assert!(!printed(&errors).contains("QZX9"));
    }

    #[test]
    fn no_schema_error_holds_a_refused_layer_name_or_kind_or_visibility(tail in "[ -~]{0,20}") {
        let value = format!("QZX9{tail}").replace('\'', "''");
        let errors = read(&format!("name: '{value}'\nkind: ''\nvisibility: '{value}'\n"))
            .expect_err("every field is refused");
        prop_assert!(errors.len() == 3);
        prop_assert!(!printed(&errors).contains("QZX9"));
    }

    #[test]
    fn no_schema_error_holds_a_refused_identity_value(tail in "[ -~]{0,20}") {
        let value = format!("QZX9<{tail}").replace('\'', "''");
        let errors = read(&format!(
            "{BASE}commit_identity:\n  name: '{value}'\n  email: '{value}'\n"
        )).expect_err("both are refused");
        prop_assert!(errors.len() == 2);
        prop_assert!(!printed(&errors).contains("QZX9"));
    }
}

// ---- null counts as absent (spec 5.1) ----

#[test]
fn an_optional_field_that_is_null_counts_as_absent() {
    let meta = with(
        "version:\ndescription: ~\nprovides_slots: null\nrequires_slots:\nforbid_patterns: ~\ncommit_identity:\n",
    )
    .expect("null optional fields are absent");
    assert!(meta.version().is_none() && meta.description().is_none());
    assert!(meta.provides_slots().is_empty() && meta.requires_slots().is_empty());
    assert!(meta.forbid_patterns().is_empty() && meta.commit_identity().is_none());
    assert_eq!(meta, read(BASE).expect("the base layer"));
}

#[test]
fn a_required_field_that_is_null_is_reported_as_missing() {
    let errors = read("name:\nkind: ~\nvisibility: null\n").expect_err("required fields");
    assert_eq!(
        messages(&errors),
        [
            "name is required",
            "kind is required",
            "visibility is required"
        ]
    );
}

#[test]
fn the_account_field_still_refuses_null_as_test_29_says() {
    assert_eq!(
        messages(&with("account:").expect_err("refused")),
        ["account must be an account name or a mapping of target to name"]
    );
    assert_eq!(
        messages(&with("account: {claude: ~}").expect_err("refused")),
        ["account.claude must be text"]
    );
}
