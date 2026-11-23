use crate::merge::provenance::Action;
use crate::merge::testing::{layer, lines, merged};
use crate::path::SafeRelPath;
use crate::target::TargetName;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

const OUTPUT_STYLE: &str = ".claude/output-styles/x.md";
const OUTPUT_STYLE_SOURCE: &str = "targets/claude/.claude/output-styles/x.md";

#[test]
fn the_most_specific_layer_wins_per_path_and_other_paths_stay() {
    let found = merged(vec![
        layer(
            "personal",
            "personal",
            &[
                (OUTPUT_STYLE_SOURCE, "PERSONAL"),
                ("targets/claude/.claude/keep.md", "KEEP"),
                ("targets/codex/.codex/prompts/p.md", "CODEX"),
            ],
        ),
        layer("repo", "repo", &[(OUTPUT_STYLE_SOURCE, "REPO")]),
    ]);
    let model = found.merged().expect("merged");
    let claude = model
        .passthrough()
        .get(&TargetName::Claude)
        .expect("claude files");
    let style = claude.get(&path(OUTPUT_STYLE)).expect("the file");
    assert_eq!(style.value().bytes(), b"REPO");
    let makers: Vec<(&str, Action)> = style
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action()))
        .collect();
    assert_eq!(
        makers,
        [("personal", Action::Define), ("repo", Action::Replace)]
    );
    let kept = claude
        .get(&path(".claude/keep.md"))
        .expect("the other path");
    assert_eq!(kept.value().bytes(), b"KEEP");
    assert_eq!(kept.provenance().contributions().len(), 1);
    let codex = model
        .passthrough()
        .get(&TargetName::Codex)
        .expect("codex files");
    assert_eq!(codex.len(), 1);
}

#[test]
fn the_same_path_under_two_targets_is_two_files() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("targets/claude/shared.md", "FOR CLAUDE"),
            ("targets/codex/shared.md", "FOR CODEX"),
        ],
    )]);
    let model = found.merged().expect("merged");
    let text = |target| {
        model
            .passthrough()
            .get(&target)
            .and_then(|files| files.get(&path("shared.md")))
            .map(|traced| traced.value().bytes().to_vec())
    };
    assert_eq!(text(TargetName::Claude), Some(b"FOR CLAUDE".to_vec()));
    assert_eq!(text(TargetName::Codex), Some(b"FOR CODEX".to_vec()));
}

#[test]
fn every_use_warns_even_a_file_that_a_more_specific_layer_replaces() {
    let found = merged(vec![
        layer("personal", "personal", &[(OUTPUT_STYLE_SOURCE, "A")]),
        layer("repo", "repo", &[(OUTPUT_STYLE_SOURCE, "B")]),
    ]);
    assert_eq!(
        lines(&found),
        [
            "warning [passthrough] personal:targets/claude/.claude/output-styles/x.md: raw passthrough to .claude/output-styles/x.md; consider turning it into a first-class item",
            "warning [passthrough] repo:targets/claude/.claude/output-styles/x.md: raw passthrough to .claude/output-styles/x.md; consider turning it into a first-class item",
        ]
    );
    assert!(
        found.merged().is_some(),
        "a warning does not stop the build"
    );
}

#[test]
fn a_layer_without_passthrough_files_adds_nothing() {
    let found = merged(vec![layer("personal", "personal", &[("README.md", "x")])]);
    assert_eq!(lines(&found), Vec::<String>::new());
    assert!(found.merged().expect("merged").passthrough().is_empty());
}

#[test]
fn a_refused_passthrough_file_stops_the_merge_and_is_not_in_it() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[(
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"HOME\": \"x\"}}",
        )],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(found.diagnostics().error_count(), 1);
    assert_eq!(found.diagnostics().warning_count(), 0);
}

#[test]
fn a_folder_of_targets_that_is_not_a_target_stops_the_merge_and_names_the_file() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("targets/claud/x.json", "{}"),
            ("targets/README.md", "notes"),
        ],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [passthrough] personal:targets/claud/x.json: this file is below a folder of targets/ that is not a target: claude or codex (\u{a7}7.11)"
        ]
    );
}

#[test]
fn files_are_kept_in_target_then_path_order() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("targets/codex/b.md", "x"),
            ("targets/claude/z.md", "x"),
            ("targets/claude/a/b.md", "x"),
            ("targets/claude/a-b.md", "x"),
        ],
    )]);
    let model = found.merged().expect("merged");
    let order: Vec<(&str, Vec<&str>)> = model
        .passthrough()
        .iter()
        .map(|(target, files)| {
            (
                target.as_str(),
                files.iter().map(|(path, _)| path.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("claude", vec!["a-b.md", "a/b.md", "z.md"]),
            ("codex", vec!["b.md"]),
        ]
    );
}
