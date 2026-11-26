//! Tests of `allowed.yaml` as account values (spec §7.13 "Allow state"): each rule that needs no
//! file-system call, the text that is written, the round trip, and the guarantee that no warning
//! holds a refused value.

use std::num::NonZeroUsize;

use proptest::prelude::{Just, Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest};

use crate::account::AccountFamily;
use crate::account::{
    AccountName, AllowEntry, AllowFile, AllowState, AllowedSource, NonEmptyTargetMap, Reading,
};
use crate::diagnostic::{Position, Where};
use crate::path::AbsolutePath;
use crate::untrusted_text::Stripped;
use crate::yaml::{Mapping, Node, Value};

fn file() -> Where {
    Where::given(Stripped::from_untrusted("allowed.yaml"))
}

fn read(document: &str) -> Reading<AllowFile> {
    AllowFile::from_bytes(document.as_bytes(), &file())
}

fn warnings(reading: &Reading<AllowFile>) -> Vec<String> {
    reading.warnings().iter().map(ToString::to_string).collect()
}

fn list(reading: &Reading<AllowFile>) -> (&AllowState, Vec<usize>) {
    match reading.value() {
        AllowFile::List { state, ignored, .. } => {
            (state, ignored.iter().map(|number| number.get()).collect())
        }
        AllowFile::Ignored => panic!("the file was ignored"),
    }
}

fn roots(state: &AllowState) -> Vec<&str> {
    state
        .entries()
        .iter()
        .map(|entry| entry.root().as_str())
        .collect()
}

fn entry(root: &str, claude: &str, codex: Option<&str>, source: &str) -> AllowEntry {
    let account = NonEmptyTargetMap::from_entries(
        [(AccountFamily::Claude, claude)]
            .into_iter()
            .chain(codex.map(|name| (AccountFamily::Codex, name)))
            .map(|(target, name)| (target, AccountName::try_from(name).expect("a name"))),
    )
    .expect("a target");
    AllowEntry::new(
        AbsolutePath::try_from(root).expect("absolute"),
        account,
        AllowedSource::try_from(source).expect("a source"),
    )
}

const EXAMPLE: &str = "# written by alz account allow; deleting the file withdraws every approval\n- root: /home/you/work/client-c/api\n  account:\n    claude: client-c\n    codex: client-c\n  source: /home/you/work/client-c/hub/layer\n";

#[test]
fn reads_the_example_of_the_spec() {
    let reading = read(EXAMPLE);
    assert_eq!(reading.warnings().warning_count(), 0);
    let (state, ignored) = list(&reading);
    assert_eq!(ignored, Vec::<usize>::new());
    let [only] = state.entries() else {
        panic!("one entry");
    };
    assert_eq!(only.root().as_str(), "/home/you/work/client-c/api");
    assert_eq!(
        only.account()
            .get(AccountFamily::Codex)
            .map(AccountName::as_str),
        Some("client-c")
    );
    assert_eq!(only.source().as_str(), "/home/you/work/client-c/hub/layer");
}

#[test]
fn an_empty_file_a_comment_and_an_empty_list_are_an_empty_list() {
    for document in ["", "# nothing\n", "[]\n", "~\n"] {
        let reading = read(document);
        assert_eq!(reading.warnings().warning_count(), 0, "{document:?}");
        let (state, ignored) = list(&reading);
        assert!(state.entries().is_empty() && ignored.is_empty());
    }
}

#[test]
fn a_top_level_that_is_not_a_list_ignores_the_whole_file_with_one_warning() {
    for document in ["root: /a\n", "just text\n", "42\n", "true\n"] {
        let reading = read(document);
        assert_eq!(reading.value(), &AllowFile::Ignored, "{document:?}");
        assert_eq!(
            warnings(&reading),
            [
                "warning [account-allowed-shape] allowed.yaml is ignored: its top level must be a list"
            ]
        );
    }
}

#[test]
fn an_entry_that_does_not_fit_is_ignored_with_a_warning_that_holds_its_position_only() {
    let cases = [
        ("- just text", "it needs exactly root, account and source"),
        ("- [a]", "it needs exactly root, account and source"),
        ("- ~", "it needs exactly root, account and source"),
        (
            "- {root: /a, account: {claude: x}, source: s, extra: 1}",
            "it needs exactly root, account and source",
        ),
        (
            "- {root: /a, account: {claude: x}}",
            "it needs exactly root, account and source",
        ),
        (
            "- {root: ./a, account: {claude: x}, source: s}",
            "its root is not an absolute path",
        ),
        (
            "- {root: 5, account: {claude: x}, source: s}",
            "its root is not an absolute path",
        ),
        (
            "- {root: ~, account: {claude: x}, source: s}",
            "its root is not an absolute path",
        ),
        (
            "- {root: /a, account: {claude: x}, source: ''}",
            "its source is not a non-empty string",
        ),
        (
            "- {root: /a, account: {claude: x}, source: 5}",
            "its source is not a non-empty string",
        ),
        (
            "- {root: /a, account: {claude: x}, source: [s]}",
            "its source is not a non-empty string",
        ),
        (
            "- {root: /a, account: client, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
        (
            "- {root: /a, account: {}, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
        (
            "- {root: /a, account: {claude: Bad-Name}, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
        (
            "- {root: /a, account: {claude: null}, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
        (
            "- {root: /a, account: {gemini: x}, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
        (
            "- {root: /a, account: {claude: x, codex: ../y}, source: s}",
            "its account is not a mapping of target to a valid account name",
        ),
    ];
    for (document, reason) in cases {
        let reading = read(&format!(
            "- {{root: /fine, account: {{claude: ok}}, source: s}}\n{document}\n"
        ));
        let (state, ignored) = list(&reading);
        assert_eq!(roots(state), ["/fine"], "{document}");
        assert_eq!(ignored, [2], "{document}");
        assert_eq!(
            warnings(&reading),
            [format!(
                "warning [account-allowed-entry] allowed.yaml: entry 2 is ignored ({reason})"
            )],
            "{document}"
        );
    }
}

#[test]
fn a_root_that_names_an_account_folder_or_is_not_real_is_left_to_the_file_system_layer() {
    // §7.13: these need $HOME, the environment and real paths, which alz-core never reads, so a
    // well-shaped entry is kept here whatever its root is.
    let reading = read(
        "- {root: /home/you/.claude-x/repo, account: {claude: x}, source: s}\n- {root: /a/../b, account: {claude: x}, source: s}\n",
    );
    assert_eq!(reading.warnings().warning_count(), 0);
    assert_eq!(list(&reading).0.entries().len(), 2);
}

#[test]
fn a_file_past_a_yaml_limit_or_not_yaml_is_ignored_with_one_warning() {
    let too_large = vec![b'#'; 1_048_577];
    let documents: [&[u8]; 3] = [&too_large, b"- [\n", b"\xff"];
    for bytes in documents {
        let reading = AllowFile::from_bytes(bytes, &file());
        assert_eq!(reading.value(), &AllowFile::Ignored);
        assert_eq!(reading.warnings().iter().count(), 1);
        assert!(warnings(&reading)[0].ends_with("; the file is ignored"));
    }
}

#[test]
fn the_entries_are_sorted_by_root_in_name_order() {
    let reading = read(
        "- {root: /b, account: {claude: x}, source: s}\n- {root: /a/b, account: {claude: x}, source: s}\n- {root: /a-b, account: {claude: x}, source: s}\n- {root: /\u{e9}, account: {claude: x}, source: s}\n- {root: /a, account: {claude: x}, source: s}\n",
    );
    assert_eq!(
        roots(list(&reading).0),
        ["/a", "/a-b", "/a/b", "/b", "/\u{e9}"]
    );
}

#[test]
fn the_written_text_is_the_header_then_the_sorted_entries() {
    let state = AllowState::from_entries(vec![
        entry("/z", "b", None, "https://example.test/hub.git"),
        entry(
            "/home/you/work/client-c/api",
            "client-c",
            Some("client-c"),
            "/home/you/work/client-c/hub/layer",
        ),
    ]);
    assert_eq!(
        state.to_yaml(),
        format!(
            "{EXAMPLE}- root: /z\n  account:\n    claude: b\n  source: https://example.test/hub.git\n"
        )
    );
    assert_eq!(
        AllowState::default().to_yaml(),
        "# written by alz account allow; deleting the file withdraws every approval\n[]\n"
    );
}

#[test]
fn an_emitted_state_reads_back_equal() {
    let state = AllowState::from_entries(vec![
        entry("/b", "x", Some("y"), "path with: colon # and hash"),
        entry("/a", "client-a", None, " leading space"),
        entry("/a-b", "x", None, "true"),
        entry("/a/b", "x", None, "123"),
    ]);
    let reading = AllowFile::from_bytes(state.to_yaml().as_bytes(), &file());
    assert_eq!(reading.warnings().warning_count(), 0);
    let (read, ignored) = list(&reading);
    assert_eq!(read, &state);
    assert_eq!(ignored, Vec::<usize>::new());
}

#[test]
fn a_root_and_source_with_a_c1_control_or_a_line_separator_are_written_escaped_and_read_back() {
    // A path may hold U+0085, U+2028 and U+2029; the file holds them only as escapes, and the
    // reader of this file reads them back (TASK-73).
    let state = AllowState::from_entries(vec![entry(
        "/work/re\u{85}po\u{2028}x\u{2029}y",
        "client-a",
        None,
        "/src/\u{7f}\u{9f}",
    )]);
    let text = state.to_yaml();
    assert!(!text.contains(['\u{7f}', '\u{85}', '\u{9f}', '\u{2028}', '\u{2029}']));
    let reading = AllowFile::from_bytes(text.as_bytes(), &file());
    assert_eq!(reading.warnings().warning_count(), 0);
    assert_eq!(list(&reading).0, &state);
}

#[test]
fn a_decoded_control_character_of_the_first_block_still_ignores_the_file() {
    let reading = read("- root: \"/r/\\x07\"\n  account: {claude: a}\n  source: s\n");
    assert!(matches!(reading.value(), AllowFile::Ignored));
}

fn any_entry() -> impl Strategy<Value = AllowEntry> {
    (
        "(/[a-z0-9 _.\u{e9}\u{4e2d}-]{1,6}){1,3}",
        "[a-z0-9][a-z0-9-]{0,10}",
        proptest::option::of("[a-z0-9][a-z0-9-]{0,10}"),
        "[ -~\u{e9}\u{4e2d}\u{1f600}]{1,24}",
    )
        .prop_map(|(root, claude, codex, source)| entry(&root, &claude, codex.as_deref(), &source))
}

proptest! {
    /// An emitted state parses back to the same state, with entries in name order of root.
    #[test]
    fn an_emitted_state_parses_back_to_the_same_sorted_state(
        entries in proptest::collection::vec(any_entry(), 0..6),
    ) {
        let state = AllowState::from_entries(entries);
        let reading = AllowFile::from_bytes(state.to_yaml().as_bytes(), &file());
        prop_assert_eq!(reading.warnings().warning_count(), 0);
        let sorted = roots(&state).windows(2).all(|pair| {
            pair[0].chars().cmp(pair[1].chars()).is_le()
        });
        prop_assert!(sorted);
        let (read, ignored) = list(&reading);
        prop_assert_eq!(read, &state);
        prop_assert!(ignored.is_empty());
    }
}

// ---- no warning holds a refused value ----

const MARKER: &str = "S3CR3T";

fn node(value: Value) -> Node {
    Node::new(value, Position::FIRST)
}

fn text(value: &str) -> Node {
    node(Value::Text(value.to_owned()))
}

fn mapping(entries: Vec<(&str, Node)>) -> Node {
    node(Value::Map(
        Mapping::from_entries(
            entries
                .into_iter()
                .map(|(key, value)| (text(key), value))
                .collect(),
        )
        .unwrap_or_default(),
    ))
}

fn root() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text("/fine")),
        Just(text(MARKER)),
        Just(text(&format!("./{MARKER}"))),
        Just(node(Value::Seq(vec![text(MARKER)]))),
        any::<i64>().prop_map(|number| node(Value::Int(i128::from(number)))),
    ]
}

fn account() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(mapping(vec![("claude", text("fine"))])),
        Just(text(MARKER)),
        Just(mapping(vec![("claude", text(MARKER))])),
        Just(mapping(vec![(MARKER, text("fine"))])),
        Just(mapping(vec![(
            "claude",
            node(Value::Seq(vec![text(MARKER)]))
        )])),
        Just(mapping(vec![])),
    ]
}

fn source() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text("/fine/layer")),
        Just(text("")),
        Just(node(Value::Seq(vec![text(MARKER)]))),
        Just(mapping(vec![("a", text(MARKER))])),
        Just(node(Value::Null)),
    ]
}

fn item() -> impl Strategy<Value = Node> {
    prop_oneof![
        Just(text(MARKER)),
        Just(node(Value::Seq(vec![text(MARKER)]))),
        (root(), account(), source(), proptest::bool::ANY).prop_map(
            |(root, account, source, extra)| {
                let base = vec![("root", root), ("account", account), ("source", source)];
                let extra = extra.then(|| ("extra", text(MARKER)));
                mapping(base.into_iter().chain(extra).collect())
            }
        ),
    ]
}

fn document() -> impl Strategy<Value = Node> {
    prop_oneof![
        proptest::collection::vec(item(), 0..5).prop_map(|items| node(Value::Seq(items))),
        item(),
    ]
}

proptest! {
    #[test]
    fn no_warning_holds_a_refused_value(document in document()) {
        let reading = AllowFile::from_node(&document, &file());
        for warning in reading.warnings() {
            prop_assert!(!warning.to_string().contains(MARKER), "{warning}");
        }
        let kept = format!("{:?}", reading.value());
        prop_assert!(!kept.contains(MARKER), "an accepted value holds a refused one");
    }

    #[test]
    fn every_entry_is_kept_or_ignored_with_its_number(items in proptest::collection::vec(item(), 0..6)) {
        let count = items.len();
        let reading = AllowFile::from_node(&node(Value::Seq(items)), &file());
        let AllowFile::List { state, ignored, .. } = reading.value() else {
            return Err(proptest::test_runner::TestCaseError::fail("a list is read as a list"));
        };
        prop_assert_eq!(state.entries().len() + ignored.len(), count);
        prop_assert_eq!(reading.warnings().iter().count(), ignored.len());
        prop_assert!(ignored.windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert!(ignored.iter().all(|number| *number >= NonZeroUsize::MIN));
    }
}

#[test]
fn each_entry_that_fits_keeps_its_number_in_the_file_in_the_order_of_the_state() {
    let reading = read(
        "- {root: /b, account: {claude: x}, source: s}\n\
         - nonsense\n\
         - {root: /a, account: {claude: x}, source: s}\n",
    );
    let AllowFile::List {
        state,
        ignored,
        numbers,
    } = reading.value()
    else {
        panic!("the file was ignored");
    };
    let roots: Vec<&str> = state.entries().iter().map(|e| e.root().as_str()).collect();
    assert_eq!(roots, ["/a", "/b"]);
    assert_eq!(numbers.iter().map(|n| n.get()).collect::<Vec<_>>(), [3, 1]);
    assert_eq!(ignored.iter().map(|n| n.get()).collect::<Vec<_>>(), [2]);
}
