use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};

use super::{FileMode, FileTree, GeneratedFile, OutputPath, OutputPathError};

fn path(text: &str) -> OutputPath {
    OutputPath::try_from(text).expect("an output path")
}

fn file(text: &str) -> GeneratedFile {
    GeneratedFile::text(text)
}

#[test]
fn a_path_with_a_control_or_format_character_is_not_an_output_path() {
    for text in [
        "a\nb",
        "a\u{7f}",
        "a\u{85}",
        "a\u{2028}b",
        "a\u{200b}b",
        "a\u{202e}b",
    ] {
        assert_eq!(
            OutputPath::try_from(text),
            Err(OutputPathError::Unprintable),
            "{text:?}"
        );
    }
    assert_eq!(OutputPath::try_from("../a"), Err(OutputPathError::Unsafe));
    assert_eq!(path("dir/\u{e9}.md").as_str(), "dir/\u{e9}.md");
}

#[test]
fn paths_that_differ_only_by_ascii_case_cannot_both_be_in_a_tree() {
    let mut tree = FileTree::new();
    tree.insert(path(".claude/settings.json"), file("one"))
        .expect("first");
    let collision = tree
        .insert(path(".claude/Settings.json"), file("two"))
        .expect_err("a case variant");
    assert_eq!(collision.kept().as_str(), ".claude/settings.json");
    assert_eq!(collision.rejected().as_str(), ".claude/Settings.json");
    assert_eq!(
        collision.to_string(),
        "two outputs claim one file: .claude/settings.json and .claude/Settings.json"
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(
        tree.get(&path(".claude/settings.json")),
        Some(&file("one")),
        "the first file is never replaced"
    );
}

#[test]
fn paths_that_differ_only_by_unicode_normalisation_cannot_both_be_in_a_tree() {
    let composed = path("skills/caf\u{e9}/SKILL.md");
    let decomposed = path("skills/cafe\u{301}/SKILL.md");
    assert_ne!(composed, decomposed);
    let mut tree = FileTree::new();
    tree.insert(composed.clone(), file("a")).expect("first");
    let collision = tree.insert(decomposed.clone(), file("b")).expect_err("NFD");
    assert_eq!(
        (collision.kept(), collision.rejected()),
        (&composed, &decomposed)
    );
}

#[test]
fn the_same_path_twice_is_a_collision() {
    let mut tree = FileTree::new();
    tree.insert(path("AGENTS.md"), file("a")).expect("first");
    assert!(tree.insert(path("AGENTS.md"), file("b")).is_err());
}

#[test]
fn only_ascii_letters_are_folded() {
    let mut tree = FileTree::new();
    tree.insert(path("\u{3a3}.md"), file("a"))
        .expect("Greek capital sigma");
    tree.insert(path("\u{3c3}.md"), file("b"))
        .expect("Greek small sigma is another file: the fold is ASCII only");
}

#[test]
fn an_accented_ascii_letter_folds_through_its_decomposition() {
    let mut tree = FileTree::new();
    tree.insert(path("\u{c9}.md"), file("a")).expect("E acute");
    assert!(
        tree.insert(path("\u{e9}.md"), file("b")).is_err(),
        "e acute is e and an accent, so it folds with E acute"
    );
}

#[test]
fn from_files_reports_every_collision_and_keeps_the_first_of_each() {
    let collisions = FileTree::from_files([
        (path("a.md"), file("1")),
        (path("A.md"), file("2")),
        (path("b.md"), file("3")),
        (path("B.MD"), file("4")),
    ])
    .expect_err("two collisions");
    assert_eq!(
        collisions
            .iter()
            .map(|collision| collision.rejected().as_str())
            .collect::<Vec<_>>(),
        ["A.md", "B.MD"]
    );
}

#[test]
fn paths_iterate_in_code_point_order() {
    let tree = FileTree::from_files(
        ["a/b", "a-b", "a", "b", "\u{e9}", "z", "10", "9", "a.b"]
            .map(|text| (path(text), file(text))),
    )
    .expect("no collisions");
    let order: Vec<&str> = tree.paths().map(OutputPath::as_str).collect();
    assert_eq!(
        order,
        ["10", "9", "a", "a-b", "a.b", "a/b", "b", "z", "\u{e9}"]
    );
}

#[test]
fn without_drops_the_accepted_paths_and_frees_their_names() {
    let tree = FileTree::from_files([
        (path("docs/a.md"), file("1")),
        (path("keep.md"), file("2")),
        (path("docs/b.md"), file("3")),
    ])
    .expect("no collisions");
    let (mut kept, dropped) = tree.without(|path| path.as_str().starts_with("docs/"));
    assert_eq!(
        dropped.iter().map(OutputPath::as_str).collect::<Vec<_>>(),
        ["docs/a.md", "docs/b.md"]
    );
    assert_eq!(kept.len(), 1);
    kept.insert(path("DOCS/a.md"), file("4"))
        .expect("a dropped path no longer claims its name");
}

#[test]
fn an_executable_file_keeps_its_mode() {
    let mut tree = FileTree::new();
    let script = GeneratedFile {
        bytes: b"#!/bin/sh\n".to_vec(),
        mode: FileMode::Executable,
    };
    tree.insert(path(".claude/hooks/run.sh"), script.clone())
        .expect("first");
    assert_eq!(tree.get(&path(".claude/hooks/run.sh")), Some(&script));
}

fn any_path() -> impl Strategy<Value = OutputPath> {
    proptest::collection::vec("[a-zA-Z0-9 _.\u{e9}\u{c9}\u{301}\u{4e2d}-]{1,5}", 1..4)
        .prop_filter_map("not a safe path", |segments| {
            OutputPath::try_from(segments.join("/").as_str()).ok()
        })
}

proptest! {
    /// Whatever is inserted, the tree never holds two paths with one folded key, and every
    /// refused path names a kept path with its key.
    #[test]
    fn a_tree_never_holds_two_paths_with_one_folded_key(
        paths in proptest::collection::vec(any_path(), 0..24),
    ) {
        let mut tree = FileTree::new();
        for path in &paths {
            match tree.insert(path.clone(), GeneratedFile::text("x")) {
                Ok(()) => {}
                Err(collision) => {
                    prop_assert_eq!(collision.kept().folded(), collision.rejected().folded());
                    prop_assert!(tree.get(collision.kept()).is_some());
                    prop_assert!(tree.get(collision.rejected()).is_none()
                        || collision.kept() == collision.rejected());
                }
            }
        }
        let keys: Vec<String> = tree.paths().map(OutputPath::folded).collect();
        let mut distinct = keys.clone();
        distinct.sort();
        distinct.dedup();
        prop_assert_eq!(keys.len(), distinct.len());
        prop_assert!(paths.iter().all(|path| tree.claim_of(path).is_some()));
    }

    /// A case variant of a path in the tree is always refused, whatever the path.
    #[test]
    fn a_case_variant_is_always_refused(path in any_path()) {
        let variant = OutputPath::try_from(path.as_str().to_ascii_uppercase().as_str())
            .expect("a variant is a safe path");
        let mut tree = FileTree::new();
        tree.insert(path.clone(), GeneratedFile::text("x")).expect("first");
        prop_assert!(variant == path || tree.insert(variant, GeneratedFile::text("y")).is_err());
    }

    /// The tree lists its paths in code point order, whatever order they came in.
    #[test]
    fn the_tree_iterates_in_code_point_order(
        paths in proptest::collection::vec(any_path(), 0..24),
    ) {
        let tree = paths.iter().fold(FileTree::new(), |mut tree, path| {
            tree.insert(path.clone(), GeneratedFile::text("x")).unwrap_or(());
            tree
        });
        let listed: Vec<&str> = tree.paths().map(OutputPath::as_str).collect();
        let ordered = listed.windows(2).all(|pair| {
            let [left, right] = pair else { return true };
            left.chars().cmp(right.chars()).is_lt()
        });
        prop_assert!(ordered);
    }
}
