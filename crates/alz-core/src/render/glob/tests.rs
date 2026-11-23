use proptest::prelude::{prop_assert_eq, proptest};

use super::PathGlob;
use super::reference_table::{MATCHES, PATHS};

fn matches(pattern: &str, path: &str) -> bool {
    PathGlob::parse(pattern).matches(path)
}

#[test]
fn agrees_with_the_reference_matcher_on_every_pattern_and_path_of_the_table() {
    for (pattern, accepted) in MATCHES {
        for path in PATHS {
            assert_eq!(
                matches(pattern, path),
                accepted.contains(path),
                "pattern {pattern:?} against {path:?}"
            );
        }
    }
}

#[test]
fn the_built_in_patterns_cover_what_section_4_5_names() {
    for (pattern, path) in [
        ("backlog/**", "backlog/tasks/a.md"),
        ("docs/**", "docs/a.md"),
        ("specs/**", "specs/001/spec.md"),
        ("*.local.*", ".claude/settings.local.json"),
        ("*.local.*", "a.local.md"),
        (".claude/agent-memory/**", ".claude/agent-memory/x/y.md"),
    ] {
        assert!(matches(pattern, path), "{pattern} {path}");
    }
}

#[test]
fn a_pattern_without_a_slash_matches_the_file_name_at_any_depth() {
    assert!(matches("*.local.*", "deep/er/a.local.json"));
    assert!(!matches("*.md", "a.md/b.txt"));
    assert!(
        !matches("docs", "docs/a.md"),
        "a folder name is not its files"
    );
}

#[test]
fn star_and_question_mark_stop_at_a_slash_and_double_star_does_not() {
    assert!(!matches("a/*/c", "a/b/b/c"));
    assert!(matches("a/**/c", "a/b/b/c"));
    assert!(matches("a/**/c", "a/c"));
    assert!(!matches("a?c", "a/c"));
    assert!(matches("a/**c", "a/b/c"));
}

#[test]
fn matching_ignores_ascii_case_in_the_pattern_and_the_path() {
    assert!(matches("backlog/**", "Backlog/x"));
    assert!(matches("Docs/**", "docs/a.md"));
    assert!(matches("*.LOCAL.*", ".claude/Settings.Local.JSON"));
    assert!(matches(".claude/agent-memory/**", ".CLAUDE/Agent-Memory/x"));
}

#[test]
fn only_ascii_case_is_folded() {
    assert!(!matches("\u{e9}/**", "\u{c9}/a"));
    assert!(matches("\u{e9}/**", "\u{e9}/a"));
}

#[test]
fn a_leading_slash_is_dropped_but_still_matches_the_whole_path() {
    assert!(matches("/docs/**", "docs/a"));
    assert!(!matches("/docs/**", "x/docs/a"));
    assert!(matches("/a", "a"));
    assert!(!matches("/a", "x/a"));
}

#[test]
fn regular_expression_characters_are_literals() {
    assert!(matches("a+b", "a+b"));
    assert!(!matches("a.b", "axb"));
    assert!(matches("[ab]", "[ab]"));
    assert!(!matches("[ab]", "a"));
    assert!(matches("(x)", "(x)"));
}

#[test]
fn a_pattern_that_forces_backtracking_in_a_regular_expression_runs_in_linear_time() {
    let pattern = format!("{}b", "a*".repeat(200));
    let path = "a".repeat(20_000);
    assert!(!matches(&pattern, &path));
    assert!(matches(&pattern, &format!("{path}b")));
    let deep = format!("{}z", "**/".repeat(200));
    assert!(!matches(&deep, &"a/".repeat(5_000)));
}

#[test]
fn a_very_long_pattern_does_not_overflow_the_stack() {
    let pattern = "a".repeat(200_000);
    assert!(!matches(&pattern, "a"));
}

/// The textbook meaning of the five tokens, by recursion over the pattern and the path.
fn naive(pattern: &[char], path: &[char]) -> bool {
    match pattern {
        [] => path.is_empty(),
        ['*', '*', '/', rest @ ..] => {
            naive(rest, path)
                || (0..path.len()).any(|end| {
                    path.get(end) == Some(&'/') && naive(rest, path.get(end + 1..).unwrap_or(&[]))
                })
        }
        ['*', '*', rest @ ..] => {
            (0..=path.len()).any(|at| naive(rest, path.get(at..).unwrap_or(&[])))
        }
        ['*', rest @ ..] => (0..=path.len())
            .take_while(|at| path.get(..*at).is_some_and(|read| !read.contains(&'/')))
            .any(|at| naive(rest, path.get(at..).unwrap_or(&[]))),
        ['?', rest @ ..] => path
            .split_first()
            .is_some_and(|(first, tail)| *first != '/' && naive(rest, tail)),
        [literal, rest @ ..] => path
            .split_first()
            .is_some_and(|(first, tail)| first.eq_ignore_ascii_case(literal) && naive(rest, tail)),
    }
}

proptest! {
    /// The linear matcher and the naive one agree on small alphabets, which hit every token.
    #[test]
    fn agrees_with_a_naive_matcher_on_generated_patterns_and_paths(
        pattern in "[ab/*?A]{0,8}",
        path in "[abA/]{0,10}",
    ) {
        let subject = if pattern.contains('/') {
            path.clone()
        } else {
            path.rsplit('/').next().unwrap_or(&path).to_owned()
        };
        let chars: Vec<char> = pattern.trim_start_matches('/').chars().collect();
        let expected = naive(&chars, &subject.chars().collect::<Vec<_>>());
        prop_assert_eq!(matches(&pattern, &path), expected, "{} against {}", pattern, path);
    }

    /// A pattern with no wildcard matches exactly itself, ignoring ASCII case.
    #[test]
    fn a_literal_pattern_matches_only_itself(
        text in "[a-zA-Z0-9/._-]{1,12}",
        other in "[a-zA-Z0-9/._-]{1,12}",
    ) {
        let pattern = format!("x/{text}");
        prop_assert_eq!(
            matches(&pattern, &format!("x/{other}")),
            text.eq_ignore_ascii_case(&other)
        );
    }
}
