//! Tests of `accounts.yaml` as account values (spec §7.13 "User config"): each rule that needs no
//! file-system call, the warning text, and the guarantee that no warning holds a refused value.

use proptest::prelude::{Just, Strategy, any, prop_assert, prop_oneof, proptest};

use crate::account::AccountFamily;
use crate::account::{
    AccountChoice, AccountDeclaration, AccountName, FolderKey, Reading, UserConfig,
};
use crate::diagnostic::{Position, Where};
use crate::untrusted_text::Stripped;
use crate::yaml::{Mapping, Node, Value};

fn file() -> Where {
    Where::given(Stripped::from_untrusted("accounts.yaml"))
}

fn read(document: &str) -> Reading<UserConfig> {
    UserConfig::from_bytes(document.as_bytes(), &file())
}

fn warnings(reading: &Reading<UserConfig>) -> Vec<String> {
    reading.warnings().iter().map(ToString::to_string).collect()
}

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("an account name")
}

fn named(text: &str) -> AccountChoice {
    AccountChoice::Named(name(text))
}

fn keys(config: &UserConfig) -> Vec<String> {
    config
        .folders()
        .iter()
        .map(|rule| match rule.key() {
            FolderKey::Absolute(path) => path.to_string(),
            FolderKey::Home(rest) => format!("~/{}", rest.as_str()),
        })
        .collect()
}

#[test]
fn reads_the_example_of_the_spec() {
    let reading = read(
        "default_name: personal\nfolders:\n  ~/work/client-a: client-a\n  ~/work/client-b:\n    claude: client-b\n    codex: default\n",
    );
    assert_eq!(reading.warnings().warning_count(), 0);
    let config = reading.value();
    assert_eq!(config.default_name(), Some(&name("personal")));
    assert_eq!(keys(config), ["~/work/client-a", "~/work/client-b"]);
    let [first, second] = config.folders() else {
        panic!("two folders");
    };
    assert_eq!(
        first.choice(),
        &AccountDeclaration::Every(named("client-a"))
    );
    assert_eq!(
        second.choice().choice_for(AccountFamily::Claude),
        Some(&named("client-b"))
    );
    assert_eq!(
        second.choice().choice_for(AccountFamily::Codex),
        Some(&AccountChoice::Default)
    );
}

#[test]
fn an_empty_file_and_a_null_document_are_an_empty_config() {
    for document in ["", "# nothing\n", "~\n", "null\n"] {
        let reading = read(document);
        assert_eq!(reading.warnings().warning_count(), 0, "{document:?}");
        assert_eq!(reading.value(), &UserConfig::default(), "{document:?}");
    }
}

#[test]
fn a_top_level_that_is_not_a_mapping_ignores_the_whole_file_with_one_warning() {
    for document in ["- a\n- b\n", "just text\n", "42\n", "true\n"] {
        let reading = read(document);
        assert_eq!(
            warnings(&reading),
            [
                "warning [account-config-shape] accounts.yaml is ignored: its top level must be a mapping"
            ],
            "{document:?}"
        );
        assert_eq!(reading.value(), &UserConfig::default());
    }
}

#[test]
fn an_unknown_key_is_ignored_with_one_warning_that_quotes_only_the_key() {
    let reading = read("default_name: personal\nsecret_token: hunter2\nfolders: ~\n");
    assert_eq!(
        warnings(&reading),
        [
            "warning [account-config-key] accounts.yaml: the key \"secret_token\" is not known and is ignored"
        ]
    );
    assert_eq!(reading.value().default_name(), Some(&name("personal")));
}

#[test]
fn an_unknown_key_is_stripped_cut_and_doubles_its_quotes() {
    let long = "k".repeat(70);
    let reading = read(&format!("\"it's\\u200B\": 1\n{long}: 2\n"));
    let [quoted, cut] = warnings(&reading).try_into().expect("two warnings");
    assert!(quoted.ends_with("the key \"it's\" is not known and is ignored"));
    assert!(cut.contains(&format!("the key \"{}…\" is not known", "k".repeat(64))));
}

#[test]
fn a_default_name_that_is_not_an_account_name_is_ignored_without_printing_it() {
    for value in [
        "Client-A",
        "../x",
        "AKIAIOSFODNN7EXAMPLE",
        "42",
        "~",
        "[a]",
        "{a: b}",
        "true",
        &format!("\"{}\"", "a".repeat(65)),
    ] {
        let reading = read(&format!("default_name: {value}\n"));
        assert_eq!(
            warnings(&reading),
            [
                "warning [account-config-default-name] accounts.yaml: default_name must be an account name; it is ignored"
            ],
            "{value}"
        );
        assert_eq!(reading.value().default_name(), None);
    }
}

#[test]
fn a_folders_key_that_is_not_an_absolute_path_or_tilde_slash_is_ignored_with_its_key_quoted() {
    let reading = read(
        "folders:\n  work/a: x\n  \"~\": x\n  ~other/a: x\n  7: x\n  /ok: x\n  \"it's\": x\n  ~/ok: x\n",
    );
    assert_eq!(keys(reading.value()), ["/ok", "~/ok"]);
    let messages = warnings(&reading);
    let expected = [
        "the folders key \"work/a\"",
        "the folders key \"~\"",
        "the folders key \"~other/a\"",
        "the folders key \"a number\"",
        "the folders key \"it's\"",
    ];
    assert_eq!(messages.len(), expected.len());
    for (message, start) in messages.iter().zip(expected) {
        assert!(
            message.contains(&format!(
                "{start} is not an absolute path or ~/..., so it is ignored"
            )),
            "{message}"
        );
    }
}

#[test]
fn a_folders_value_that_is_not_an_account_name_or_a_target_map_is_ignored() {
    let reading = read(
        "folders:\n  /a: Client-A\n  /b: {}\n  /c: {gemini: x}\n  /d: {claude: null}\n  /e: [x]\n  /f: ~\n  /g: 5\n  /h: {claude: Bad}\n  /i: default\n  /j: {codex: x}\n",
    );
    assert_eq!(keys(reading.value()), ["/i", "/j"]);
    let messages = warnings(&reading);
    assert_eq!(messages.len(), 8);
    for (message, key) in messages.iter().zip("abcdefgh".chars()) {
        assert!(
            message.contains(&format!(
                "the folders entry \"/{key}\" must be an account name or a mapping of target to name, so it is ignored"
            )),
            "{message}"
        );
    }
}

#[test]
fn a_folders_value_that_is_not_a_mapping_is_ignored_and_a_null_is_nothing() {
    let reading = read("folders: [a]\n");
    assert_eq!(
        warnings(&reading),
        [
            "warning [account-config-shape] accounts.yaml: 'folders' must be a mapping; it is ignored"
        ]
    );
    for document in ["folders:\n", "folders: ~\n", "folders: {}\n"] {
        let reading = read(document);
        assert_eq!(reading.warnings().warning_count(), 0, "{document}");
        assert_eq!(reading.value().folders(), []);
    }
}

#[test]
fn the_folders_keep_the_order_of_the_file() {
    let reading = read("folders:\n  /z: a\n  /a: b\n  ~/m: c\n");
    assert_eq!(keys(reading.value()), ["/z", "/a", "~/m"]);
}

#[test]
fn a_file_past_a_yaml_limit_or_not_yaml_is_ignored_with_one_warning() {
    let too_large = vec![b'#'; 1_048_577];
    let documents: [&[u8]; 4] = [&too_large, b"a: [\n", b"\xff\xfe", b"a: 1\n---\nb: 2\n"];
    for bytes in documents {
        let reading = UserConfig::from_bytes(bytes, &file());
        assert_eq!(reading.warnings().iter().count(), 1);
        assert!(
            warnings(&reading)[0].ends_with("; the file is ignored"),
            "{}",
            warnings(&reading)[0]
        );
        assert_eq!(reading.value(), &UserConfig::default());
    }
}

// ---- no warning holds a refused value ----

const MARKER: &str = "S3CR3T";

fn at() -> Position {
    Position::FIRST
}

fn node(value: Value) -> Node {
    Node::new(value, at())
}

fn text(value: &str) -> Node {
    node(Value::Text(value.to_owned()))
}

fn mapping(entries: Vec<(Node, Node)>) -> Node {
    // The generator never repeats a key inside a mapping.
    node(Value::Map(
        Mapping::from_entries(entries).unwrap_or_default(),
    ))
}

/// A value that no field of `accounts.yaml` accepts, and that holds the marker.
fn refused() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text(MARKER)),
        Just(text(&format!("../{MARKER}"))),
        Just(node(Value::Seq(vec![text(MARKER)]))),
        Just(mapping(vec![(text("claude"), text(MARKER))])),
        Just(mapping(vec![(text(MARKER), text("client-a"))])),
        Just(mapping(vec![(text("claude"), node(Value::Null))])),
        any::<i64>().prop_map(|number| node(Value::Int(i128::from(number)))),
    ]
}

/// A key of a folders mapping or the top level, which a warning may quote.
fn key() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text("/ok")),
        Just(text("~/ok")),
        Just(text("relative")),
        Just(text("unknown")),
        Just(node(Value::Int(1))),
    ]
}

fn folders() -> impl Strategy<Value = Node> {
    proptest::collection::btree_map("[a-z/~]{1,6}", refused(), 0..4).prop_map(|entries| {
        mapping(
            entries
                .into_iter()
                .map(|(key, value)| (text(&key), value))
                .collect(),
        )
    })
}

/// A top level that is not a mapping: a mapping's own keys may be quoted, so it is left out here.
fn not_a_mapping() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text(MARKER)),
        Just(node(Value::Seq(vec![text(MARKER)]))),
        any::<i64>().prop_map(|number| node(Value::Int(i128::from(number)))),
    ]
}

fn document() -> impl Strategy<Value = Node> {
    prop_oneof![
        not_a_mapping(),
        (
            refused(),
            prop_oneof![folders(), not_a_mapping()],
            proptest::option::of(key()),
            refused()
        )
            .prop_map(|(default_name, folders, extra, extra_value)| {
                let base = vec![
                    (text("default_name"), default_name),
                    (text("folders"), folders),
                ];
                let extra = extra
                    .filter(|key| !matches!(key.value(), Value::Text(text) if text == "default_name" || text == "folders"))
                    .map(|key| (key, extra_value));
                mapping(base.into_iter().chain(extra).collect())
            }),
    ]
}

proptest! {
    #[test]
    fn no_warning_holds_a_refused_value(document in document()) {
        let reading = UserConfig::from_node(&document, &file());
        for warning in reading.warnings() {
            prop_assert!(!warning.to_string().contains(MARKER), "{warning}");
        }
        // What was accepted holds no refused value either.
        let kept = format!("{:?}", reading.value());
        prop_assert!(!kept.contains(MARKER), "an accepted value holds a refused one");
    }
}
