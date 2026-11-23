use proptest::prelude::{Just, Strategy, prop_assert, prop_assume, proptest};

use super::{covers, widened};
use crate::item::ToolGrant;
use crate::merge::OrderedSet;

fn grant(text: &str) -> ToolGrant {
    ToolGrant::try_from(text).expect("a tool entry")
}

fn covered(base: &str, candidate: &str) -> bool {
    covers(&grant(base), &grant(candidate))
}

#[test]
fn a_path_scope_covers_by_whole_segments() {
    let table = [
        ("write:docs/**", "write:docs/adr/**", true),
        ("write:docs/**", "write:docs/a.md", true),
        ("write:docs/**", "write:docs/**", true),
        ("write:docs/**", "write:docs-private/a.md", false),
        ("write:docs/**", "write:docs-private/**", false),
        ("write:docs*", "write:docs-private/a.md", false),
        ("write:docs*", "write:docs-private/**", false),
        ("write:docs*", "write:docs/a.md", false),
        ("write:docs*", "write:docs*", true),
        ("write:docs/*", "write:docs/a/b.md", false),
        ("write:docs/*", "write:docs/a.md", false),
        ("write:docs/**/x.md", "write:docs/a/x.md", false),
        ("write:docs/**", "write:docs/../etc/passwd", false),
        ("write:docs/**", "write:docs/./a.md", false),
        ("write:docs/**", "write:../docs/a.md", false),
        ("write:docs/**", "write:docs/**/..", false),
        ("write:docs/**", "write:src/a.md", false),
        ("write:docs/**", "write", false),
        ("write:./docs/**", "write:./docs/a.md", false),
        ("write:./docs/**", "write:./docs/**", true),
        ("write:**", "write:docs/a.md", true),
        ("write:**", "write:docs/..", false),
        ("write", "write:docs/a.md", true),
        ("write", "write", true),
        ("write:docs/**", "edit:docs/a.md", false),
        ("write:docs/a/**", "write:docs/**", false),
        ("write:**", "write:/etc/passwd", false),
        ("write:**", "write:~/.ssh/id", false),
        ("write:**", "write:~", false),
        ("write:**", "write:C:/Users/a", false),
        ("write:**", "write:c:x", false),
        ("write:**", "write:docs\\a.md", false),
        ("write:docs/**", "write:docs\\..\\a", false),
        ("write:/etc/**", "write:/etc/passwd", false),
        ("write:/etc/**", "write:/etc/**", true),
        ("write:~/**", "write:~/a", false),
        ("write:~/**", "write:~/**", true),
        ("write:**", "write:cd:x", true),
        ("write:**", "write:docs:b", true),
    ];
    for (base, candidate, verdict) in table {
        assert_eq!(
            covered(base, candidate),
            verdict,
            "{base} covers {candidate}"
        );
    }
}

#[test]
fn every_path_tool_compares_paths() {
    for tool in ["read", "write", "edit", "grep", "glob", "notebook_edit"] {
        assert!(covered(
            &format!("{tool}:docs/**"),
            &format!("{tool}:docs/a")
        ));
        assert!(!covered(
            &format!("{tool}:docs/**"),
            &format!("{tool}:docs-private/a")
        ));
    }
}

#[test]
fn a_command_scope_covers_what_its_star_stands_for_unless_it_holds_an_operator() {
    let table = [
        ("bash:mise run *", "bash:mise run test", true),
        ("bash:mise run *", "bash:mise run *", true),
        ("bash:mise run *", "bash:mise run test --all", true),
        ("bash:mise run *", "bash:mise run a; b", false),
        ("bash:mise run *", "bash:mise run a && b", false),
        ("bash:mise run *", "bash:mise run a & b", false),
        ("bash:mise run *", "bash:mise run a | b", false),
        ("bash:mise run *", "bash:mise run $(a)", false),
        ("bash:mise run *", "bash:mise run a > f", false),
        ("bash:mise run *", "bash:mise run a < f", false),
        ("bash:mise run *", "bash:mise run `a`", false),
        ("bash:mise run *", "bash:mise run a\nb", false),
        ("bash:mise run *", "bash:mise run a\rb", false),
        ("bash:mise run *", "bash:mise run (a)", false),
        ("bash:mise run *", "bash:mise runs", false),
        ("bash:mise run *", "bash:mise run", false),
        ("bash:mise run *", "bash", false),
        ("bash:mise run test", "bash:mise run test", true),
        ("bash:mise run test", "bash:mise run test2", false),
        ("bash:mise run a; b", "bash:mise run a; b", true),
        ("bash:mise *", "bash:mise run *", true),
        ("bash:mise run **", "bash:mise run test", true),
        ("bash:mise run **", "bash:mise run a;", false),
        ("bash:*", "bash:ls", true),
        ("bash:*", "bash:ls; rm", false),
        ("bash", "bash:mise run a; b", true),
        ("bash:mise run *", "read:mise run test", false),
    ];
    for (base, candidate, verdict) in table {
        assert_eq!(
            covered(base, candidate),
            verdict,
            "{base} covers {candidate}"
        );
    }
}

#[test]
fn any_other_scope_covers_only_an_equal_scope() {
    assert!(covered(
        "web_fetch:example.invalid",
        "web_fetch:example.invalid"
    ));
    assert!(!covered("web_fetch:*", "web_fetch:example.invalid"));
    assert!(!covered("web_fetch:example.invalid", "web_fetch"));
    assert!(covered("web_fetch", "web_fetch:example.invalid"));
    assert!(!covered("mcp__a__b:x*", "mcp__a__b:xy"));
    assert!(covered("mcp__a__b", "mcp__a__b:xy"));
    assert!(!covered("mcp__a__b", "mcp__a__c"));
}

#[test]
fn a_base_that_ends_in_two_stars_covers_only_an_equal_scope() {
    assert!(covered("write:docs/**/**", "write:docs/**/**"));
    assert!(!covered("write:docs/**/**", "write:docs/**"));
    assert!(!covered("write:docs/**/**", "write:docs/a"));
}

#[test]
fn the_entries_no_base_entry_covers_are_the_widened_ones() {
    let set = |entries: &[&str]| -> OrderedSet<ToolGrant> {
        entries.iter().map(|entry| grant(entry)).collect()
    };
    let base = set(&["read", "bash:mise run *"]);
    let result = set(&["read", "bash:mise run test", "write", "bash:mise run a; b"]);
    assert_eq!(
        widened(Some(&base), &result)
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["write", "bash:mise run a; b"]
    );
    assert_eq!(widened(None, &result).len(), 0);
}

/// The scopes of a path tool over a small alphabet, so equal prefixes are common.
fn path_scopes() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::prop_oneof![
            Just("a"),
            Just("b"),
            Just("*"),
            Just("**"),
            Just("."),
            Just(".."),
            Just(""),
            Just("~"),
            Just("C:"),
            Just("a\\b")
        ],
        1..5,
    )
    .prop_map(|segments| segments.join("/"))
}

/// The scopes of a command tool over a small alphabet, so equal prefixes are common.
fn command_scopes() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::prop_oneof![
            Just('m'),
            Just(' '),
            Just('*'),
            Just(';'),
            Just('&'),
            Just('|'),
            Just('<'),
            Just('>'),
            Just('('),
            Just(')'),
            Just('`'),
            Just('\n'),
            Just('\r'),
            Just('x')
        ],
        0..6,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn grants() -> impl Strategy<Value = ToolGrant> {
    proptest::prop_oneof![
        Just(grant("write")),
        Just(grant("bash")),
        path_scopes().prop_map(|scope| grant(&format!("write:{scope}"))),
        command_scopes().prop_map(|scope| grant(&format!("bash:{scope}"))),
        Just(grant("web_fetch:a")),
        Just(grant("web_fetch:b")),
    ]
}

proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: 1024, max_global_rejects: 200_000, ..proptest::prelude::ProptestConfig::default() })]

    #[test]
    fn covers_is_reflexive(entry in grants()) {
        prop_assert!(covers(&entry, &entry));
    }

    #[test]
    fn covers_is_transitive(a in grants(), b in grants(), c in grants()) {
        prop_assume!(covers(&a, &b) && covers(&b, &c));
        prop_assert!(covers(&a, &c), "{a} covers {b} covers {c}");
    }

    #[test]
    fn a_path_scope_that_is_not_the_base_is_covered_only_without_dot_segments(
        base in path_scopes(),
        candidate in path_scopes(),
    ) {
        let (base_entry, candidate_entry) = (
            grant(&format!("read:{base}")),
            grant(&format!("read:{candidate}")),
        );
        prop_assume!(base != candidate && covers(&base_entry, &candidate_entry));
        prop_assert!(base.ends_with("**"));
        prop_assert!(candidate.split('/').all(|segment| segment != "." && segment != ".."));
    }
}

/// Every scope of `alphabet` up to `length` pieces, joined by `join`.
fn scopes_of(alphabet: &[&str], length: usize, join: &str) -> Vec<String> {
    (0..=length)
        .flat_map(|size| {
            (0..size).fold(vec![Vec::<&str>::new()], |words, _| {
                words
                    .into_iter()
                    .flat_map(|word| {
                        alphabet
                            .iter()
                            .map(move |piece| word.iter().copied().chain([*piece]).collect())
                    })
                    .collect()
            })
        })
        .map(|word| word.join(join))
        .collect()
}

fn transitive_over(tool: &str, scopes: &[String]) -> bool {
    let entries: Vec<ToolGrant> = scopes
        .iter()
        .map(|scope| grant(&format!("{tool}:{scope}")))
        .collect();
    entries.iter().all(|a| {
        entries.iter().filter(|b| covers(a, b)).all(|b| {
            entries
                .iter()
                .filter(|c| covers(b, c))
                .all(|c| covers(a, c))
        })
    })
}

#[test]
fn covers_is_transitive_over_every_short_path_scope() {
    let scopes = scopes_of(&["a", "**", ".", "*"], 4, "/");
    assert!(transitive_over("write", &scopes));
}

#[test]
fn covers_is_transitive_over_every_short_command_scope() {
    let scopes = scopes_of(&["m", " ", "*", ";"], 4, "");
    assert!(transitive_over("bash", &scopes));
}
