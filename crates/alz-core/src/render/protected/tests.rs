use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use super::{CLAUDE_MEMORY_PATTERNS, Protected};
use crate::render::tree::{FileTree, GeneratedFile, OutputPath};
use crate::stack::ProtectedPattern;

fn path(text: &str) -> OutputPath {
    OutputPath::try_from(text).expect("an output path")
}

fn claude() -> Protected {
    Protected::new(&CLAUDE_MEMORY_PATTERNS, &[])
}

#[test]
fn the_defaults_and_the_memory_folders_are_protected() {
    let protected = claude();
    for text in [
        "backlog/tasks/task-1.md",
        "docs/a.md",
        "specs/001/spec.md",
        ".claude/settings.local.json",
        "x/notes.local.md",
        ".claude/agent-memory/architect/MEMORY.md",
        ".claude/agent-memory-local/x",
    ] {
        assert!(protected.covers(&path(text)), "{text}");
    }
    for text in [
        "AGENTS.md",
        ".claude/settings.json",
        ".claude/agents/a.md",
        "docs",
        "notbacklog/a",
    ] {
        assert!(!protected.covers(&path(text)), "{text}");
    }
}

#[test]
fn case_variants_of_a_protected_path_are_protected() {
    let protected = claude();
    for text in [
        "Backlog/x",
        "DOCS/a.md",
        ".claude/Settings.LOCAL.json",
        ".Claude/Agent-Memory/x",
        "Specs/a",
    ] {
        assert!(protected.covers(&path(text)), "{text}");
    }
}

#[test]
fn the_manifests_patterns_add_to_the_defaults() {
    let protected = Protected::new(&[], &[ProtectedPattern::from("generated/**")]);
    assert!(protected.covers(&path("Generated/a")));
    assert!(protected.covers(&path("docs/a")));
    assert!(!protected.covers(&path(".claude/agent-memory/x")));
}

#[test]
fn dropping_removes_each_protected_file_with_one_warning() {
    let tree = FileTree::from_files(
        [
            "AGENTS.md",
            "docs/a.md",
            ".claude/settings.json",
            "Specs/b.md",
        ]
        .map(|text| (path(text), GeneratedFile::text(text))),
    )
    .expect("no collisions");
    let (kept, warnings) = claude().drop_from(tree);
    assert_eq!(
        kept.paths().map(OutputPath::as_str).collect::<Vec<_>>(),
        [".claude/settings.json", "AGENTS.md"]
    );
    assert_eq!(
        warnings.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "warning [protected] Specs/b.md: protected path; a layer ships it, but the build leaves it alone",
            "warning [protected] docs/a.md: protected path; a layer ships it, but the build leaves it alone",
        ]
    );
    assert_eq!(warnings.error_count(), 0);
}

proptest! {
    /// Dropping leaves a tree no pattern covers, and loses only covered paths.
    #[test]
    fn what_is_left_is_never_protected_and_what_is_dropped_always_was(
        names in proptest::collection::vec("[a-zA-Z.]{1,6}(/[a-zA-Z.]{1,6}){0,2}", 0..12),
    ) {
        let protected = claude();
        let paths: Vec<OutputPath> = names.iter().filter_map(|n| OutputPath::try_from(n.as_str()).ok()).collect();
        let tree = paths.iter().fold(FileTree::new(), |mut tree, path| {
            tree.insert(path.clone(), GeneratedFile::text("x")).unwrap_or(());
            tree
        });
        let before = tree.len();
        let (kept, warnings) = protected.drop_from(tree);
        prop_assert!(kept.paths().all(|path| !protected.covers(path)));
        prop_assert_eq!(kept.len() + warnings.warning_count(), before);
    }
}

/// §4.5: `?` matches one Unicode scalar value within one segment, also one outside the Basic
/// Multilingual Plane (a matcher over UTF-16 units or bytes would need two or more `?`).
#[test]
fn a_question_mark_matches_one_scalar_value_also_above_u_ffff_and_never_a_slash() {
    let protected = Protected::new(&[], &[ProtectedPattern::from("notes/?.md")]);
    // One ASCII letter, one letter in the BMP (two bytes), one in the BMP (three bytes) and one
    // outside it (four bytes, a surrogate pair in UTF-16).
    for letter in ["a", "\u{e9}", "\u{20ac}", "\u{1f600}", "\u{10348}"] {
        assert!(
            protected.covers(&path(&format!("notes/{letter}.md"))),
            "one scalar value {letter:?}"
        );
    }
    for text in [
        "notes/.md",
        "notes/ab.md",
        "notes/\u{1f600}\u{1f600}.md",
        "notes/a/b.md",
    ] {
        assert!(!protected.covers(&path(text)), "{text}");
    }
    let slash = Protected::new(&[], &[ProtectedPattern::from("a?b")]);
    assert!(!slash.covers(&path("a/b")));
    assert!(slash.covers(&path("a\u{1f600}b")));
}

#[test]
fn a_protected_path_is_covered_in_either_unicode_spelling() {
    let protected = Protected::new(&[], &[ProtectedPattern::from("caf\u{e9}/**")]);
    assert!(protected.covers(&path("cafe\u{301}/x.md")));
    assert!(protected.covers(&path("caf\u{e9}/x.md")));
    let decomposed = Protected::new(&[], &[ProtectedPattern::from("cafe\u{301}/**")]);
    assert!(decomposed.covers(&path("caf\u{e9}/x.md")));
}
