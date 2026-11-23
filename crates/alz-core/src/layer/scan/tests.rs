use proptest::prelude::{Just, Strategy, any, prop_assert_eq, prop_assert_ne, proptest};

use super::{LayerScan, NameProblem, Purpose};
use crate::layer::{LayerRefusal, scan_verdict};
use crate::lock::{ContentHash, FileDigest};
use crate::path::SafeRelPath;
use crate::untrusted_text::Stripped;

const MIB: u64 = 1024 * 1024;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

fn entries(count: usize) -> LayerScan {
    (0..count).fold(LayerScan::new(), |scan, _| scan.listed_entry())
}

/// One fact the walk can add to a scan, so that a test can add them in any order.
#[derive(Debug, Clone)]
enum Fact {
    Entries(usize),
    Name(NameProblem),
    File(&'static str, u64),
    EnvFile(&'static str),
    OverBudget,
    Unreadable,
}

fn apply(scan: LayerScan, fact: &Fact) -> LayerScan {
    match fact {
        Fact::Entries(count) => (0..*count).fold(scan, |scan, _| scan.listed_entry()),
        Fact::Name(problem) => scan.with_name_problem(*problem),
        Fact::File(name, length) => {
            scan.with_file(path(name), FileDigest::of(name.as_bytes()), *length)
        }
        Fact::EnvFile(name) => scan.with_env_file(path(name)),
        Fact::OverBudget => scan.with_bytes(16 * MIB + 1),
        Fact::Unreadable => scan.with_unreadable(),
    }
}

fn scan_of(facts: &[Fact]) -> LayerScan {
    facts.iter().fold(LayerScan::new(), apply)
}

fn any_fact() -> impl Strategy<Value = Fact> {
    proptest::prop_oneof![
        proptest::prop_oneof![
            Just(0_usize),
            Just(1),
            Just(2000),
            Just(2001),
            0_usize..2100
        ]
        .prop_map(Fact::Entries),
        Just(Fact::Name(NameProblem::NotUtf8)),
        Just(Fact::Name(NameProblem::NotSafe)),
        proptest::sample::select(vec!["a.md", "b/c.md", "z"])
            .prop_flat_map(|name| (0..40_u64).prop_map(move |length| Fact::File(name, length))),
        proptest::sample::select(vec![".env", "a/.envrc"]).prop_map(Fact::EnvFile),
        Just(Fact::OverBudget),
        Just(Fact::Unreadable),
    ]
}

/// The first check that fails, worked out from the facts alone, for account resolution.
fn expected_resolution(facts: &[Fact]) -> Option<LayerRefusal> {
    let listed: usize = facts
        .iter()
        .map(|fact| {
            if let Fact::Entries(count) = fact {
                *count
            } else {
                0
            }
        })
        .sum();
    let has = |wanted: &dyn Fn(&Fact) -> bool| facts.iter().any(wanted);
    if listed > 2000 {
        Some(LayerRefusal::TooManyEntries)
    } else if has(&|fact| matches!(fact, Fact::Name(NameProblem::NotUtf8))) {
        Some(LayerRefusal::NameNotUtf8)
    } else if has(&|fact| matches!(fact, Fact::Name(NameProblem::NotSafe))) {
        Some(LayerRefusal::NameNotSafe)
    } else if has(&|fact| matches!(fact, Fact::Unreadable | Fact::OverBudget)) {
        Some(LayerRefusal::Unreadable)
    } else {
        None
    }
}

#[test]
fn a_layer_with_a_name_that_is_not_valid_utf8_is_a_cache_miss_layer_invalid_in_resolution() {
    // TASK-69 AC #2: the verdict that account resolution uses.
    let scan = entries(3)
        .with_name_problem(NameProblem::NotUtf8)
        .with_file(path("layer.yaml"), FileDigest::of(b"x"), 1);
    assert_eq!(
        scan_verdict(Purpose::Resolution, scan),
        Err(LayerRefusal::NameNotUtf8)
    );
    assert_eq!(LayerRefusal::NameNotUtf8.miss(), "layer-invalid");
}

#[test]
fn each_refusal_is_the_miss_the_spec_names() {
    for (refusal, miss) in [
        (LayerRefusal::TooManyEntries, "unreadable"),
        (LayerRefusal::NameNotUtf8, "layer-invalid"),
        (LayerRefusal::NameNotSafe, "layer-invalid"),
        (LayerRefusal::Unreadable, "unreadable"),
        (LayerRefusal::EnvFile, "layer-invalid"),
    ] {
        assert_eq!(refusal.miss(), miss);
    }
}

#[test]
fn entry_2000_is_listed_and_entry_2001_is_too_many_in_resolution_only() {
    assert!(scan_verdict(Purpose::Resolution, entries(2000)).is_ok());
    assert_eq!(
        scan_verdict(Purpose::Resolution, entries(2001)),
        Err(LayerRefusal::TooManyEntries)
    );
    assert!(!entries(2000).is_past_entry_limit(Purpose::Resolution));
    assert!(entries(2001).is_past_entry_limit(Purpose::Resolution));
    assert!(scan_verdict(Purpose::Build, entries(100_000)).is_ok());
    assert!(!entries(100_000).is_past_entry_limit(Purpose::Build));
}

#[test]
fn sixteen_mib_is_read_and_one_byte_more_is_unreadable_in_resolution_only() {
    let at_limit = LayerScan::new().with_bytes(16 * MIB);
    let past = LayerScan::new().with_bytes(16 * MIB + 1);
    assert!(scan_verdict(Purpose::Resolution, at_limit).is_ok());
    assert_eq!(
        scan_verdict(Purpose::Resolution, past.clone()),
        Err(LayerRefusal::Unreadable)
    );
    assert!(scan_verdict(Purpose::Build, past).is_ok());
}

#[test]
fn an_unreadable_file_is_refused_in_both_purposes() {
    for purpose in [Purpose::Resolution, Purpose::Build] {
        assert_eq!(
            scan_verdict(purpose, LayerScan::new().with_unreadable()),
            Err(LayerRefusal::Unreadable)
        );
    }
}

#[test]
fn in_the_build_the_name_error_wins_over_everything_that_follows_it() {
    let scan = entries(5000)
        .with_bytes(20 * MIB)
        .with_unreadable()
        .with_env_file(path(".env"))
        .with_name_problem(NameProblem::NotUtf8);
    assert_eq!(
        scan_verdict(Purpose::Build, scan.clone()),
        Err(LayerRefusal::NameNotUtf8)
    );
    // In resolution the entry limit is checked first.
    assert_eq!(
        scan_verdict(Purpose::Resolution, scan),
        Err(LayerRefusal::TooManyEntries)
    );
}

#[test]
fn a_name_that_is_not_utf8_wins_over_one_that_is_not_safe() {
    let scan = LayerScan::new()
        .with_name_problem(NameProblem::NotSafe)
        .with_name_problem(NameProblem::NotUtf8)
        .with_name_problem(NameProblem::NotSafe);
    assert_eq!(
        scan_verdict(Purpose::Build, scan),
        Err(LayerRefusal::NameNotUtf8)
    );
}

#[test]
fn the_env_check_follows_the_content_hash_and_is_the_last_refusal() {
    let scan = LayerScan::new()
        .with_file(path("a.md"), FileDigest::of(b"a"), 1)
        .with_env_file(path("sub/.env.dev"))
        .with_env_file(path(".env"));
    let read = scan_verdict(Purpose::Resolution, scan).expect("passes the checks before the hash");
    assert_eq!(read.files().len(), 1);
    let listed: Vec<&str> = read.env_files().iter().map(SafeRelPath::as_str).collect();
    assert_eq!(listed, [".env", "sub/.env.dev"]);
    assert_eq!(read.without_env_files(), Err(LayerRefusal::EnvFile));
    assert!(LayerRefusal::Unreadable < LayerRefusal::EnvFile);
}

#[test]
fn a_layer_without_env_files_gives_the_files_the_hash_covers() {
    let scan = LayerScan::new()
        .with_file(path("b.md"), FileDigest::of(b"b"), 1)
        .with_file(path("a.md"), FileDigest::of(b"a"), 1)
        .with_file(path("node_modules/x"), FileDigest::of(b"x"), 1);
    let files = scan_verdict(Purpose::Resolution, scan)
        .expect("passes")
        .without_env_files()
        .expect("no .env file");
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a.md", "b.md"]);
    assert_eq!(
        ContentHash::of(&files).to_string(),
        ContentHash::of(&crate::lock::LayerFiles::from_contents([
            (path("a.md"), b"a".as_slice()),
            (path("b.md"), b"b".as_slice()),
        ]))
        .to_string()
    );
}

#[test]
fn the_build_error_for_a_bad_name_names_the_layer_and_never_the_name() {
    let line = LayerRefusal::NameNotUtf8
        .file_name_diagnostic(Stripped::from_untrusted("path:../hub"))
        .expect("the name error has a message")
        .to_string();
    assert_eq!(
        line,
        "error [layer] a file or folder name in layer 'path:../hub' is not valid UTF-8; rename it"
    );
    assert_eq!(
        LayerRefusal::TooManyEntries.file_name_diagnostic(Stripped::from_untrusted("x")),
        None
    );
}

proptest! {
    #[test]
    fn the_resolution_verdict_is_the_first_failing_check_for_any_order_of_names(
        facts in proptest::collection::vec(any_fact(), 0..12).prop_shuffle(),
    ) {
        let verdict = scan_verdict(Purpose::Resolution, scan_of(&facts)).map(|_| ());
        prop_assert_eq!(verdict, expected_resolution(&facts).map_or(Ok(()), Err));
    }

    #[test]
    fn the_verdict_does_not_depend_on_the_order_the_facts_arrive_in(
        facts in proptest::collection::vec(any_fact(), 0..12),
        seed in any::<u64>(),
    ) {
        let mut reordered = facts.clone();
        // A deterministic permutation: rotate by the seed, then reverse.
        let length = reordered.len().max(1);
        reordered.rotate_left(usize::try_from(seed).unwrap_or(0) % length);
        reordered.reverse();
        for purpose in [Purpose::Resolution, Purpose::Build] {
            prop_assert_eq!(
                scan_verdict(purpose, scan_of(&facts)),
                scan_verdict(purpose, scan_of(&reordered))
            );
        }
    }

    #[test]
    fn in_the_build_a_name_error_precedes_every_other_refusal(
        facts in proptest::collection::vec(any_fact(), 0..12),
    ) {
        let has_name = facts.iter().any(|fact| matches!(fact, Fact::Name(NameProblem::NotUtf8)));
        let verdict = scan_verdict(Purpose::Build, scan_of(&facts)).map(|_| ());
        if has_name {
            prop_assert_eq!(verdict, Err(LayerRefusal::NameNotUtf8));
        }
        // The build never reports the entry limit.
        prop_assert_ne!(verdict, Err(LayerRefusal::TooManyEntries));
    }
}
