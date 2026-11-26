use std::collections::BTreeSet;

use proptest::collection::vec;
use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};

use super::{Rendered, render};
use crate::lint::parse_document;
use crate::merge::testing::{layer, layer_executable, layer_from_yaml};
use crate::merge::{Placements, merge};
use crate::render::tree::{FileMode, OutputPath, fold};
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack};
use crate::target::Registry;
use crate::target::claude::tests::stacks;

/// A file of the acceptance fixtures, read at compile time.
macro_rules! fixture {
    ($path:literal) => {
        include_str!(concat!("../../../../../fixtures/", $path))
    };
}

/// A file of the expected output, kept under `tests/fixtures`.
macro_rules! golden {
    ($path:literal) => {
        include_str!(concat!("../../../tests/fixtures/", $path))
    };
}

const REPO_MANIFEST: &str = "stack:\n  - source: path:./.layer\n";

/// The build of `layers` under the manifest text `manifest`, and the stack that was built.
fn build(layers: Vec<ResolvedLayer>, manifest: &str) -> (Rendered, ResolvedStack) {
    let node = parse_document(manifest.as_bytes()).expect("valid YAML");
    let manifest = Manifest::from_yaml(&node).expect("a manifest");
    let stack = ResolvedStack::validate(layers).expect("a stack");
    let (merged, found) = merge(&stack, &manifest, &Placements::default()).into_parts();
    assert!(!found.has_errors(), "{found}");
    let merged = merged.expect("no error was found");
    (render(&Registry::builtin(), &merged, &manifest), stack)
}

fn lines(rendered: &Rendered) -> Vec<String> {
    rendered
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn path(text: &str) -> OutputPath {
    OutputPath::try_from(text).expect("a safe path")
}

fn paths(rendered: &Rendered) -> Vec<&str> {
    rendered.tree().paths().map(OutputPath::as_str).collect()
}

/// The whole tree: the paths, the bytes and the modes, against `wanted`.
fn assert_whole_tree(rendered: &Rendered, wanted: &[(&str, &str, FileMode)]) {
    let mut names: Vec<&str> = wanted.iter().map(|(name, _, _)| *name).collect();
    names.sort_unstable();
    assert_eq!(paths(rendered), names);
    for (name, text, mode) in wanted {
        let file = rendered.tree().get(&path(name)).expect("a made file");
        assert_eq!(
            String::from_utf8_lossy(&file.bytes),
            *text,
            "{name} differs from the expected file"
        );
        assert_eq!(file.mode, *mode, "{name}");
    }
}

fn regular(golden: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str, FileMode)> {
    golden
        .iter()
        .map(|(name, text)| (*name, *text, FileMode::Regular))
        .collect()
}

/// A repo layer that makes `.claude/settings.json` through `permissions.yaml`, plus `files`.
fn with_settings(files: &[(&str, &str)]) -> Vec<ResolvedLayer> {
    let all: Vec<(&str, &str)> = [("permissions.yaml", "deny: [\"read:.env*\"]\n")]
        .into_iter()
        .chain(files.iter().copied())
        .collect();
    vec![layer("shop-api", "repo", &all)]
}

#[test]
fn a_passthrough_path_equal_to_a_generated_one_after_case_folding_is_the_error_passthrough() {
    let (rendered, _) = build(
        with_settings(&[("targets/claude/.claude/Settings.json", "{}\n")]),
        REPO_MANIFEST,
    );
    assert_eq!(
        lines(&rendered)
            .into_iter()
            .filter(|line| line.starts_with("error"))
            .collect::<Vec<_>>(),
        [
            "error [passthrough] shop-api:targets/claude/.claude/Settings.json: passthrough .claude/Settings.json collides with a generated file"
        ]
    );
    let kept = rendered
        .tree()
        .get(&path(".claude/settings.json"))
        .expect("the generated file stays");
    assert!(String::from_utf8_lossy(&kept.bytes).contains("deny"));
    assert!(
        rendered
            .tree()
            .get(&path(".claude/Settings.json"))
            .is_none()
    );
    assert!(
        rendered.to_write().is_none(),
        "a build with an error writes nothing"
    );
}

#[test]
fn a_passthrough_path_equal_to_a_generated_one_exactly_is_the_error_passthrough() {
    let (rendered, _) = build(
        with_settings(&[("targets/claude/.claude/settings.json", "{}\n")]),
        REPO_MANIFEST,
    );
    assert_eq!(rendered.diagnostics().error_count(), 1);
    assert!(lines(&rendered).iter().any(|line| {
        line.starts_with("error [passthrough] shop-api:targets/claude/.claude/settings.json")
    }));
}

#[test]
fn two_passthrough_files_that_fold_together_are_a_collision_and_the_first_stays() {
    let (rendered, _) = build(
        vec![layer(
            "shop-api",
            "repo",
            &[
                ("targets/claude/tools/Run.sh", "a\n"),
                ("targets/claude/tools/run.sh", "b\n"),
            ],
        )],
        REPO_MANIFEST,
    );
    assert_eq!(paths(&rendered), ["AGENTS.md", "CLAUDE.md", "tools/Run.sh"]);
    assert!(
        lines(&rendered)
            .iter()
            .any(|line| line.starts_with("error [collision]"))
    );
}

#[test]
fn a_passthrough_file_is_copied_with_its_bytes_and_its_executable_bit() {
    let (rendered, _) = build(
        vec![layer_executable(
            "name: shop-api\nkind: repo\nvisibility: private\n",
            &[
                ("targets/claude/tools/run.sh", "#!/bin/sh\n"),
                ("targets/claude/notes/readme.txt", "hello\n"),
            ],
            None,
            &["targets/claude/tools/run.sh"],
        )],
        "stack:\n  - source: path:./.layer\nprotected_paths: [\"notes/**\"]\n",
    );
    let script = rendered.tree().get(&path("tools/run.sh")).expect("copied");
    assert_eq!(script.bytes, b"#!/bin/sh\n");
    assert_eq!(script.mode, FileMode::Executable);
    assert!(rendered.tree().get(&path("notes/readme.txt")).is_none());
    assert_eq!(
        lines(&rendered)
            .into_iter()
            .filter(|line| line.contains("[protected]"))
            .collect::<Vec<_>>(),
        [
            "warning [protected] notes/readme.txt: protected path; a layer ships it, but the build leaves it alone"
        ]
    );
}

#[test]
fn a_file_under_a_protected_path_in_any_ascii_case_is_dropped_with_the_warning_and_not_locked() {
    let shipped = [
        "Docs/Guide.md",
        "BACKLOG/tasks/task-1.md",
        "Specs/x/spec.md",
        "notes/team.LOCAL.md",
        ".CLAUDE/Agent-Memory/reviewer/MEMORY.md",
        ".claude/AGENT-MEMORY-LOCAL/x",
    ];
    let files: Vec<(String, &str)> = shipped
        .iter()
        .map(|name| (format!("targets/claude/{name}"), "{}\n"))
        .chain([("targets/claude/kept/ok.md".to_owned(), "ok\n")])
        .collect();
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, text)| (name.as_str(), *text))
        .collect();
    let (rendered, stack) = build(vec![layer("shop-api", "repo", &refs)], REPO_MANIFEST);
    assert_eq!(paths(&rendered), ["AGENTS.md", "CLAUDE.md", "kept/ok.md"]);
    let warnings: Vec<String> = lines(&rendered)
        .into_iter()
        .filter(|line| line.contains("[protected]"))
        .collect();
    assert_eq!(warnings.len(), shipped.len(), "{warnings:?}");
    let locked: Vec<String> = rendered
        .lock(&stack)
        .files
        .keys()
        .map(ToString::to_string)
        .collect();
    assert_eq!(locked, ["AGENTS.md", "CLAUDE.md", "kept/ok.md"]);
}

#[test]
fn a_target_without_an_adapter_is_the_error_target_at_layer_yaml() {
    let (rendered, _) = build(
        vec![layer("shop-api", "repo", &[])],
        "stack:\n  - source: path:./.layer\ntargets: [claude, codex]\n",
    );
    assert_eq!(
        lines(&rendered),
        [
            "error [target] layer.yaml: the 'codex' adapter is not implemented yet (implemented: claude)"
        ]
    );
    assert!(rendered.to_write().is_none());
}

#[test]
fn an_instruction_file_over_the_budget_is_the_error_budget() {
    let (rendered, _) = build(
        vec![layer(
            "shop-api",
            "repo",
            &[("instructions/00-a.md", "one\n\ntwo\n")],
        )],
        "stack:\n  - source: path:./.layer\nbudgets:\n  instructions_max_bytes: 10\n",
    );
    assert!(
        lines(&rendered)
            .iter()
            .any(|line| line.starts_with("error [budget]"))
    );
    assert!(rendered.to_write().is_none());
}

#[test]
fn a_header_source_that_closes_its_comment_makes_no_file_and_an_error() {
    let (rendered, _) = build(
        vec![layer(
            "shop-api",
            "repo",
            &[("instructions/00-a-->b.md", "text\n")],
        )],
        REPO_MANIFEST,
    );
    assert!(rendered.diagnostics().has_errors());
    assert!(rendered.tree().get(&path("AGENTS.md")).is_none());
    assert!(rendered.to_write().is_none());
}

#[test]
fn fixture_01_whole_tree_is_written_as_expected() {
    let layers = stacks::CASE_01
        .iter()
        .map(|part| {
            layer_executable(
                part.yaml,
                part.files,
                None,
                &["hooks/scripts/block-secrets.sh"],
            )
        })
        .collect();
    let (rendered, stack) = build(
        layers,
        "stack:\n  - source: path:./.layer\noptions:\n  strict_slots: true\nmcp:\n  enable: [context7]\n",
    );
    assert!(
        !rendered.diagnostics().has_errors(),
        "{}",
        rendered.diagnostics()
    );
    let wanted: Vec<(&str, &str, FileMode)> = [
        (
            "AGENTS.md",
            golden!("instructions/01-determinism/AGENTS.md"),
            FileMode::Regular,
        ),
        (
            "CLAUDE.md",
            golden!("instructions/01-determinism/CLAUDE.md"),
            FileMode::Regular,
        ),
        (
            ".claude/settings.json",
            golden!("claude-settings/01-determinism/.claude/settings.json"),
            FileMode::Regular,
        ),
        (
            ".mcp.json",
            golden!("claude-settings/01-determinism/.mcp.json"),
            FileMode::Regular,
        ),
        (
            ".claude/hooks/personal/scripts/block-secrets.sh",
            golden!(
                "claude-settings/01-determinism/.claude/hooks/personal/scripts/block-secrets.sh"
            ),
            FileMode::Executable,
        ),
    ]
    .into_iter()
    .chain(regular(stacks::GOLDEN_01))
    .collect();
    assert_whole_tree(&rendered, &wanted);
    assert_eq!(
        rendered.lock(&stack).files.len(),
        rendered.tree().len(),
        "the lock lists the tree"
    );
}

#[test]
fn fixture_02_whole_tree_matches_expected() {
    let layers = vec![
        layer_from_yaml(
            fixture!("02-instructions/personal/layer.yaml"),
            &[
                (
                    "instructions/00-core.md",
                    fixture!("02-instructions/personal/instructions/00-core.md"),
                ),
                (
                    "instructions/10-style.md",
                    fixture!("02-instructions/personal/instructions/10-style.md"),
                ),
            ],
        ),
        layer_from_yaml(
            fixture!("02-instructions/client/layer.yaml"),
            &[(
                "instructions/10-domain.md",
                fixture!("02-instructions/client/instructions/10-domain.md"),
            )],
        ),
        layer_from_yaml(
            fixture!("02-instructions/repo/.layer/layer.yaml"),
            &[
                (
                    "instructions/10-style.md",
                    fixture!("02-instructions/repo/.layer/instructions/10-style.md"),
                ),
                (
                    "instructions/50-repo.md",
                    fixture!("02-instructions/repo/.layer/instructions/50-repo.md"),
                ),
            ],
        ),
    ];
    let (rendered, _) = build(layers, fixture!("02-instructions/repo/layer.yaml"));
    assert!(
        !rendered.diagnostics().has_errors(),
        "{}",
        rendered.diagnostics()
    );
    assert_whole_tree(
        &rendered,
        &[
            (
                "AGENTS.md",
                fixture!("02-instructions/expected/AGENTS.md"),
                FileMode::Regular,
            ),
            (
                "CLAUDE.md",
                fixture!("02-instructions/expected/CLAUDE.md"),
                FileMode::Regular,
            ),
        ],
    );
}

#[test]
fn fixture_03_agents_and_skills_match_expected() {
    let layers = stacks::CASE_03
        .iter()
        .map(|part| layer_from_yaml(part.yaml, part.files))
        .collect();
    let (rendered, _) = build(layers, REPO_MANIFEST);
    let agent = rendered
        .tree()
        .get(&path(".claude/agents/architect.md"))
        .expect("the agent");
    assert_eq!(
        String::from_utf8_lossy(&agent.bytes),
        fixture!("03-agent-overlay/expected/.claude/agents/architect.md")
    );
    let claude_files: Vec<&str> = paths(&rendered)
        .into_iter()
        .filter(|name| name.starts_with(".claude/agents/") || name.starts_with(".claude/skills/"))
        .collect();
    let mut wanted: Vec<&str> = stacks::GOLDEN_03.iter().map(|(name, _)| *name).collect();
    wanted.sort_unstable();
    assert_eq!(claude_files, wanted);
}

#[test]
fn fixture_15_leaves_every_protected_path_out_of_the_tree_and_the_lock() {
    const SHIPPED: [(&str, &str); 7] = [
        (
            "targets/claude/.claude/agent-memory-local/reviewer/MEMORY.md",
            fixture!(
                "15-protected-paths/personal/targets/claude/.claude/agent-memory-local/reviewer/MEMORY.md"
            ),
        ),
        (
            "targets/claude/.claude/agent-memory/reviewer/MEMORY.md",
            fixture!(
                "15-protected-paths/personal/targets/claude/.claude/agent-memory/reviewer/MEMORY.md"
            ),
        ),
        (
            "targets/claude/.claude/settings.local.json",
            fixture!("15-protected-paths/personal/targets/claude/.claude/settings.local.json"),
        ),
        (
            "targets/claude/backlog/tasks/task-1.md",
            fixture!("15-protected-paths/personal/targets/claude/backlog/tasks/task-1.md"),
        ),
        (
            "targets/claude/docs/guide.md",
            fixture!("15-protected-paths/personal/targets/claude/docs/guide.md"),
        ),
        (
            "targets/claude/notes/team.md",
            fixture!("15-protected-paths/personal/targets/claude/notes/team.md"),
        ),
        (
            "targets/claude/specs/001-slice/spec.md",
            fixture!("15-protected-paths/personal/targets/claude/specs/001-slice/spec.md"),
        ),
    ];
    let personal: Vec<(&str, &str)> = SHIPPED
        .into_iter()
        .chain([(
            "instructions/00-core.md",
            fixture!("15-protected-paths/personal/instructions/00-core.md"),
        )])
        .collect();
    let layers = vec![
        layer_from_yaml(
            fixture!("15-protected-paths/personal/layer.yaml"),
            &personal,
        ),
        layer_from_yaml(fixture!("15-protected-paths/repo/.layer/layer.yaml"), &[]),
    ];
    let (rendered, stack) = build(layers, fixture!("15-protected-paths/repo/layer.yaml"));
    assert_eq!(paths(&rendered), ["AGENTS.md", "CLAUDE.md"]);
    assert_eq!(
        lines(&rendered)
            .iter()
            .filter(|line| line.starts_with("warning [protected]"))
            .count(),
        SHIPPED.len()
    );
    assert!(!rendered.diagnostics().has_errors());
    let lock = rendered.lock(&stack).emit();
    assert!(
        !lock.contains("backlog") && !lock.contains("notes"),
        "{lock}"
    );
}

/// A passthrough path of one to three segments from a small alphabet that holds the generated
/// paths, the protected folders and case variants of both.
fn passthrough_path() -> impl Strategy<Value = String> {
    vec(
        proptest::sample::select(vec![
            ".claude",
            ".CLAUDE",
            "settings.json",
            "Settings.json",
            "agents",
            "docs",
            "Docs",
            "backlog",
            "agent-memory",
            "a.local.md",
            "A.LOCAL.md",
            "x",
            "X",
            "notes",
        ]),
        1..4,
    )
    .prop_map(|parts| parts.join("/"))
}

proptest! {
    #[test]
    fn render_is_deterministic_never_folds_two_paths_together_and_locks_exactly_its_tree(
        names in vec(passthrough_path(), 0..8),
    ) {
        let files: Vec<(String, &str)> = names
            .iter()
            .map(|name| (format!("targets/claude/{name}"), "{}\n"))
            .collect();
        let refs: Vec<(&str, &str)> =
            files.iter().map(|(name, text)| (name.as_str(), *text)).collect();
        let build_once = || build(with_settings(&refs), "stack:\n  - source: path:./.layer\nprotected_paths: [\"x/**\"]\n");
        let (first, stack) = build_once();
        let (second, _) = build_once();
        prop_assert_eq!(&first, &second);
        prop_assert_eq!(first.lock(&stack).emit(), second.lock(&stack).emit());

        let folded: BTreeSet<String> = first.tree().paths().map(OutputPath::folded).collect();
        prop_assert_eq!(folded.len(), first.tree().len());
        prop_assert!(first.tree().paths().all(|name| !first.protected().covers(name)));
        let locked: Vec<String> =
            first.lock(&stack).files.keys().map(ToString::to_string).collect();
        let made: Vec<String> = first.tree().paths().map(ToString::to_string).collect();
        prop_assert_eq!(locked, made);
        prop_assert!(first.tree().paths().all(|name| fold(name.as_safe()) == name.folded()));
    }
}
