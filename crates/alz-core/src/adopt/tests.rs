//! Tests of adoption planning: each adopted kind against the files the spec says it becomes for the
//! fixtures of tests 14 and 20, the problems that reject a plan whole (TASK-73), and the laws of
//! the `layer.yaml` edit.

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::manifest_edit::{self, Refused};
use super::{AdoptionInput, AdoptionPlan, AdoptionRefused, NativeFile, plan};
use crate::lint::parse_document;
use crate::merge::testing::{layer_from_yaml, merged};
use crate::merge::{Merged, Placements};
use crate::path::SafeRelPath;
use crate::stack::{Manifest, ResolvedStack};
use crate::target::Registry;

const MANIFEST: &str = "stack:\n  - source: path:../personal\n  - source: path:./.layer   # the repo layer\ntargets: [claude]\n";

const PERSONAL: &str = "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n";
const REPO_LAYER: &str = "name: shop-hub\nkind: repo\nvisibility: private\n";
const MODELS: &str = "aliases:\n  strong: { claude: opus }\n  fast: { claude: sonnet }\n";

struct World {
    stack: ResolvedStack,
    manifest: Manifest,
    merged: Merged,
    manifest_text: String,
}

fn world(manifest_text: &str, repo_files: &[(&str, &str)]) -> World {
    world_with_models(MODELS, manifest_text, repo_files)
}

fn world_with_models(models: &str, manifest_text: &str, repo_files: &[(&str, &str)]) -> World {
    let layers = vec![
        layer_from_yaml(PERSONAL, &[("models.yaml", models)]),
        layer_from_yaml(REPO_LAYER, repo_files),
    ];
    let node = parse_document(manifest_text.as_bytes()).expect("a manifest");
    let manifest = Manifest::from_yaml(&node).expect("a valid manifest");
    let merging = merged(layers.clone());
    World {
        stack: ResolvedStack::validate(layers).expect("a stack"),
        manifest,
        merged: merging.merged().expect("the stack merges").clone(),
        manifest_text: manifest_text.to_owned(),
    }
}

fn natives(files: &[(&str, &str)]) -> BTreeMap<SafeRelPath, NativeFile> {
    files
        .iter()
        .map(|(path, text)| {
            (
                SafeRelPath::try_from(*path).expect("a safe path"),
                NativeFile::new(
                    text.as_bytes().to_vec(),
                    std::path::Path::new(path)
                        .extension()
                        .is_some_and(|ext| ext == "sh"),
                ),
            )
        })
        .collect()
}

fn run(world: &World, files: &[(&str, &str)]) -> Result<AdoptionPlan, AdoptionRefused> {
    let natives = natives(files);
    let agents_md = natives
        .iter()
        .find(|(path, _)| path.as_str() == "AGENTS.md")
        .map(|(_, file)| file.bytes().to_vec());
    plan(&AdoptionInput {
        stack: &world.stack,
        manifest: &world.manifest,
        manifest_text: world.manifest_text.as_bytes(),
        placements: &Placements::default(),
        merged: &world.merged,
        natives: &natives,
        agents_md: agents_md.as_deref(),
        registry: &Registry::builtin(),
    })
}

fn problems(refused: &AdoptionRefused) -> Vec<String> {
    refused
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn file<'a>(plan: &'a AdoptionPlan, path: &str) -> &'a str {
    let key = SafeRelPath::try_from(path).expect("a path");
    let Some(bytes) = plan.layer_files().get(&key) else {
        panic!("the plan writes {path}");
    };
    std::str::from_utf8(bytes).expect("UTF-8")
}

// The files of test 20's repo, as the fixture holds them.
const AGENTS_MD: &str =
    "# Shop hub\n- Contracts first.\n\n<!-- TOOL START -->\nUse the board.\n<!-- TOOL END -->\n";
const CLAUDE_MD: &str = "@AGENTS.md\n@.harness/PROTOCOL.md\n\n- Reviewers are read-only.\n\n<!-- TOOL START -->\nUse the board.\n<!-- TOOL END -->\n";
const SETTINGS: &str = r#"{
  "env": { "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "1" },
  "permissions": {
    "additionalDirectories": [".."],
    "deny": ["Read(./.env)", "Read(../**/secrets/**)", "Bash(git push --force:*)"]
  },
  "hooks": {
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.harness/scripts/session-brief.sh" }] }],
    "PreToolUse": [{ "matcher": "Edit|Write", "hooks": [{ "type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/guard.sh" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "cd \"$CLAUDE_PROJECT_DIR\" && (bash scripts/check.sh 1>&2 || exit 2)" }] }],
    "Notification": [{ "hooks": [{ "type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.harness/scripts/notify.sh" }] }]
  }
}
"#;
const MCP: &str = r#"{ "mcpServers": {
  "backlog": { "command": "backlog", "args": ["mcp", "start"] },
  "tracker": { "type": "http", "url": "https://tracker.example/mcp" }
} }
"#;
const ARCHITECT: &str = "---\nname: architect\ndescription: Plans slices and owns contracts.\ntools: Read, Grep, Glob, Write, Edit, Bash, WebSearch, WebFetch\nmodel: opus\neffort: xhigh\nmemory: project\nmaxTurns: 60\ncolor: blue\n---\nYou are the architect.\n";
const REVIEWER: &str = "---\nname: reviewer\ndescription: Read-only reviewer.\ntools: Read, Grep, Glob\npermissionMode: plan\nmodel: haiku\n---\nReview in a fresh context.\n";

fn fixture_20() -> Vec<(&'static str, &'static str)> {
    vec![
        ("AGENTS.md", AGENTS_MD),
        ("CLAUDE.md", CLAUDE_MD),
        (".claude/settings.json", SETTINGS),
        (".mcp.json", MCP),
        (".claude/agents/architect.md", ARCHITECT),
        (".claude/agents/reviewer.md", REVIEWER),
        (".claude/hooks/guard.sh", "#!/bin/sh\nexit 0\n"),
        (
            ".claude/rules/backend.md",
            "---\npaths:\n  - \"src/**\"\n---\n- Validate input.\n",
        ),
        (
            ".claude/skills/next-task/SKILL.md",
            "---\nname: next-task\ndescription: Picks the next task.\n---\nDo it.\n",
        ),
    ]
}

#[test]
fn adopting_fixture_20_plans_the_layer_files() {
    let world = world(MANIFEST, &[]);
    let plan = run(&world, &fixture_20()).expect("a plan");

    assert_eq!(
        file(&plan, "permissions.yaml"),
        "deny:\n  - read:./.env\n  - read:../**/secrets/**\n  - bash:git push --force:*\n"
    );
    assert_eq!(
        file(&plan, "hooks/hooks.yaml"),
        concat!(
            "- id: session-brief\n  event: session_start\n  command: '\"${PROJECT_DIR}\"/.harness/scripts/session-brief.sh'\n  on_fail: warn\n",
            "- id: guard\n  event: pre_tool\n  match:\n    tools:\n      - edit\n      - write\n  run: scripts/guard.sh\n  on_fail: warn\n",
            "- id: check\n  event: stop\n  command: cd \"${PROJECT_DIR}\" && (bash scripts/check.sh 1>&2 || exit 2)\n  on_fail: warn\n",
            "- id: notify\n  event: claude:Notification\n  command: '\"${PROJECT_DIR}\"/.harness/scripts/notify.sh'\n  on_fail: warn\n",
        )
    );
    assert_eq!(
        file(&plan, "settings.yaml"),
        "claude:\n  env:\n    CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: \"1\"\n"
    );
    assert_eq!(
        file(&plan, "models.yaml"),
        "aliases:\n  haiku:\n    claude: haiku\n"
    );
    assert_eq!(
        file(&plan, "mcp.yaml"),
        "backlog:\n  transport: stdio\n  command: backlog\n  args:\n    - mcp\n    - start\ntracker:\n  transport: http\n  url: https://tracker.example/mcp\n"
    );
    assert_eq!(
        file(&plan, "agents/architect.md"),
        "---\nname: architect\ndescription: Plans slices and owns contracts.\ntools:\n  - read\n  - grep\n  - glob\n  - write\n  - edit\n  - bash\n  - web_search\n  - web_fetch\nmodel: strong\nmemory: project\nclaude:\n  effort: xhigh\n  maxTurns: 60\n  color: blue\n---\nYou are the architect.\n"
    );
    assert_eq!(
        file(&plan, "agents/reviewer.md"),
        "---\nname: reviewer\ndescription: Read-only reviewer.\ntools:\n  - read\n  - grep\n  - glob\npermission_mode: plan\nmodel: haiku\n---\nReview in a fresh context.\n"
    );
    assert_eq!(
        file(&plan, "targets/claude/CLAUDE.extra.md"),
        "@.harness/PROTOCOL.md\n\n- Reviewers are read-only.\n"
    );
}

#[test]
fn adopting_fixture_20_moves_files_and_edits_only_what_it_adds_to_the_manifest() {
    let world = world(MANIFEST, &[]);
    let plan = run(&world, &fixture_20()).expect("a plan");
    let moves: Vec<(&str, &str)> = plan
        .moves()
        .iter()
        .map(|moved| (moved.from().as_str(), moved.to().as_str()))
        .collect();
    assert_eq!(
        moves,
        [
            ("AGENTS.md", "instructions/00-adopted.md"),
            (".claude/hooks/guard.sh", "hooks/scripts/guard.sh"),
            (".claude/rules/backend.md", "rules/backend.md"),
            (
                ".claude/skills/next-task/SKILL.md",
                "skills/next-task/SKILL.md"
            ),
        ]
    );
    assert_eq!(
        plan.manifest_edit().expect("an edit").text(),
        format!(
            "{MANIFEST}workspace:\n  add_dirs:\n    - ..\nmcp:\n  enable:\n    - backlog\n    - tracker\n"
        )
    );
    assert_eq!(plan.removes().len(), 5, "the converted files are removed");
}

#[test]
fn an_agent_that_holds_only_neutral_fields_moves_unchanged() {
    let world = world(MANIFEST, &[]);
    let agent = "---\nname: foo\ndescription: Hand-written.\n---\nHAND-WRITTEN body.\n";
    let plan = run(&world, &[(".claude/agents/foo.md", agent)]).expect("a plan");
    assert_eq!(plan.moves().len(), 1);
    assert_eq!(plan.moves()[0].to().as_str(), "agents/foo.md");
    assert!(plan.layer_files().is_empty());
}

#[test]
fn an_agent_a_general_layer_defines_becomes_a_replace_overlay() {
    let layers = vec![
        layer_from_yaml(
            PERSONAL,
            &[(
                "agents/foo.md",
                "---\nname: foo\ndescription: General.\n---\nBase.\n",
            )],
        ),
        layer_from_yaml(REPO_LAYER, &[]),
    ];
    let merging = merged(layers.clone());
    let world = World {
        stack: ResolvedStack::validate(layers).expect("a stack"),
        manifest: Manifest::from_yaml(&parse_document(MANIFEST.as_bytes()).expect("yaml"))
            .expect("manifest"),
        merged: merging.merged().expect("merges").clone(),
        manifest_text: MANIFEST.to_owned(),
    };
    let agent = "---\nname: foo\ndescription: Repo version.\n---\nRepo body.\n";
    let plan = run(&world, &[(".claude/agents/foo.md", agent)]).expect("a plan");
    assert_eq!(
        file(&plan, "agents/foo.overlay.md"),
        "---\nextends: foo\nmode: replace\ndescription: Repo version.\n---\nRepo body.\n"
    );
    assert_eq!(plan.notes().len(), 1);
}

#[test]
fn a_literal_env_value_in_mcp_json_rejects_the_plan_without_printing_it() {
    let world = world(MANIFEST, &[]);
    let mcp = r#"{ "mcpServers": { "tracker": { "command": "tracker-mcp", "env": { "TRACKER_TOKEN": "tk-not-a-real-value" } } } }"#;
    let refused =
        run(&world, &[(".mcp.json", mcp), (".claude/rules/a.md", "x\n")]).expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("error [adopt] .mcp.json: "),
        "{lines:?}"
    );
    assert!(lines[0].contains("TRACKER_TOKEN"), "{lines:?}");
    assert!(!lines[0].contains("tk-not-a-real-value"), "{lines:?}");
}

#[test]
fn an_unknown_permission_tool_and_a_hook_type_other_than_command_are_problems() {
    let world = world(MANIFEST, &[]);
    let settings = r#"{
      "permissions": { "allow": ["Frobnicate(x)"] },
      "hooks": { "Stop": [{ "hooks": [{ "type": "prompt", "command": "x" }] }] }
    }"#;
    let refused = run(&world, &[(".claude/settings.json", settings)]).expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines
            .iter()
            .all(|line| line.contains(".claude/settings.json")),
        "{lines:?}"
    );
}

#[test]
fn text_the_layer_reader_refuses_rejects_the_plan_and_the_problem_names_the_native_file() {
    let world = world(MANIFEST, &[]);
    // U+2028 in a value: the emitter writes it escaped, and the reader refuses it (spec §7.13).
    let settings = "{ \"env\": { \"A\": \"line\u{2028}separator\" } }";
    let refused = run(&world, &[(".claude/settings.json", settings)]).expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("error [adopt] .claude/settings.json: "),
        "{lines:?}"
    );
    assert!(!lines[0].contains('\u{2028}'), "{lines:?}");
}

#[test]
fn a_settings_key_the_typed_table_refuses_is_not_planned() {
    let world = world(MANIFEST, &[]);
    for settings in [
        r#"{ "notASettingsKey": true }"#,
        r#"{ "cleanupPeriodDays": "thirty" }"#,
    ] {
        let refused = run(&world, &[(".claude/settings.json", settings)]).expect_err("refused");
        assert!(
            problems(&refused)[0].starts_with("error [adopt] .claude/settings.json: "),
            "{settings}"
        );
    }
}

#[test]
fn a_native_file_adoption_cannot_take_is_a_problem() {
    let world = world(MANIFEST, &[]);
    let refused = run(&world, &[(".claude/commands/x.md", "x\n")]).expect_err("refused");
    assert_eq!(
        problems(&refused),
        [
            "error [adopt] .claude/commands/x.md: cannot be adopted automatically; move it into a layer by hand"
        ]
    );
}

#[test]
fn a_layer_file_that_exists_is_never_overwritten_and_yaml_merges_into_it() {
    let world = world(
        MANIFEST,
        &[
            ("models.yaml", "aliases:\n  mine: { claude: opus-x }\n"),
            ("instructions/00-adopted.md", "x\n"),
        ],
    );
    let refused = run(&world, &[("AGENTS.md", "y\n")]).expect_err("refused");
    assert!(problems(&refused)[0].contains("already exists"));
    let plan = run(&world, &[(".claude/agents/r.md", REVIEWER)]).expect("a plan");
    assert_eq!(
        file(&plan, "models.yaml"),
        "aliases:\n  mine:\n    claude: opus-x\n  haiku:\n    claude: haiku\n"
    );
}

#[test]
fn an_agent_with_native_hooks_is_refused_because_the_layer_reader_refuses_them() {
    let world = world(MANIFEST, &[]);
    let agent = "---\nname: a\ndescription: d\nhooks:\n  Stop:\n    - hooks:\n        - type: command\n          command: x\n---\nBody.\n";
    let refused = run(&world, &[(".claude/agents/a.md", agent)]).expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("error [adopt] .claude/agents/a.md: "),
        "{lines:?}"
    );
}

#[test]
fn planning_is_deterministic() {
    let world = world(MANIFEST, &[]);
    assert_eq!(run(&world, &fixture_20()), run(&world, &fixture_20()));
}

#[test]
fn a_manifest_without_a_repo_layer_refuses_adoption() {
    let layers = vec![layer_from_yaml(PERSONAL, &[])];
    let merging = merged(layers.clone());
    let text = "stack:\n  - source: path:../personal\n";
    let manifest =
        Manifest::from_yaml(&parse_document(text.as_bytes()).expect("yaml")).expect("manifest");
    let world = World {
        stack: ResolvedStack::validate(layers).expect("a stack"),
        manifest,
        merged: merging.merged().expect("merges").clone(),
        manifest_text: text.to_owned(),
    };
    let refused = run(&world, &[]).expect_err("refused");
    assert_eq!(
        problems(&refused),
        [
            "error [adopt] --adopt needs a repo layer inside the repo as the last stack entry (e.g. path:./.layer)"
        ]
    );
}

fn manifest_of(text: &str) -> Manifest {
    Manifest::from_yaml(&parse_document(text.as_bytes()).expect("yaml")).expect("manifest")
}

fn edited(text: &str, dirs: &[&str], ids: &[&str]) -> Result<Option<String>, Refused> {
    let manifest = manifest_of(text);
    let dirs: Vec<String> = dirs.iter().map(|dir| (*dir).to_owned()).collect();
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
    manifest_edit::edit(text, &manifest, &dirs, &ids).map(|edit| edit.map(|edit| edit.text))
}

#[test]
fn the_manifest_edit_handles_each_form_of_the_spec() {
    let base = "stack:\n  - source: path:./.layer\n";
    assert_eq!(
        edited(
            &format!("{base}workspace:\n  add_dirs:\n    - a  # keep\n"),
            &["b"],
            &[]
        )
        .expect("edit"),
        Some(format!(
            "{base}workspace:\n  add_dirs:\n    - a  # keep\n    - b\n"
        ))
    );
    assert_eq!(
        edited(
            &format!("{base}workspace:\n  add_dirs: [a, b]\n"),
            &["c"],
            &[]
        )
        .expect("edit"),
        Some(format!("{base}workspace:\n  add_dirs: [a, b, c]\n"))
    );
    assert_eq!(
        edited(&format!("{base}mcp:\n  enable: []\n"), &[], &["x"]).expect("edit"),
        Some(format!("{base}mcp:\n  enable: [x]\n"))
    );
    assert_eq!(
        edited(
            &format!("{base}workspace:\n  hub: ..\n# end\n"),
            &["c d"],
            &[]
        )
        .expect("edit"),
        Some(format!(
            "{base}workspace:\n  hub: ..\n  add_dirs:\n    - c d\n# end\n"
        ))
    );
    assert_eq!(
        edited("stack:\n  - source: path:./.layer", &[], &["x"]).expect("edit"),
        Some("stack:\n  - source: path:./.layer\nmcp:\n  enable:\n    - x\n".to_owned())
    );
    assert_eq!(
        edited(&format!("{base}workspace:\n  add_dirs: [a]\n"), &["a"], &[]).expect("edit"),
        None
    );
}

#[test]
fn the_manifest_edit_refuses_forms_it_cannot_edit() {
    let base = "stack:\n  - source: path:./.layer\n";
    for text in [
        format!("{base}workspace: {{ add_dirs: [a] }}\n"),
        format!("{base}workspace:\n  add_dirs: &dirs [a]\n"),
        format!("{base}workspace:\n  add_dirs: [a,\n    b]\n"),
        format!("{base}workspace:\n  add_dirs: [a] # why\n"),
        format!("{base}workspace:\n  add_dirs: !!seq [a]\n"),
    ] {
        assert_eq!(edited(&text, &["z"], &[]), Err(Refused::Form), "{text}");
    }
}

proptest! {
    /// The edit keeps every line it does not add, in order, and the edited text reads back as the
    /// manifest with the values appended.
    #[test]
    fn the_manifest_edit_keeps_every_byte_outside_the_added_lines(
        held in proptest::collection::vec("[a-z]{1,6}", 0..3),
        wanted in proptest::collection::vec("[a-z]{1,6}", 1..4),
        comment in "[a-z ]{0,8}",
    ) {
        let list: String = held.iter().map(|dir| ["    - ", dir.as_str(), "\n"].concat()).collect();
        let text = format!(
            "# {comment}\nstack:\n  - source: path:./.layer   # the repo layer\nworkspace:\n  add_dirs:\n{list}targets: [claude]\n"
        );
        let text = if held.is_empty() { text.replace("  add_dirs:\n", "") } else { text };
        let dirs: Vec<&str> = wanted.iter().map(String::as_str).collect();
        let result = edited(&text, &dirs, &[]);
        let Ok(Some(after)) = result else {
            prop_assert!(wanted.iter().all(|dir| held.contains(dir)), "{result:?}");
            return Ok(());
        };
        let kept: Vec<&str> = after.lines().collect();
        let original: Vec<&str> = text.lines().collect();
        let mut rest = kept.iter();
        prop_assert!(original.iter().all(|line| rest.any(|kept| kept == line)));
        let manifest = Manifest::from_yaml(&parse_document(after.as_bytes()).expect("yaml")).expect("manifest");
        let read: Vec<&str> = manifest.workspace().add_dirs().iter().map(crate::stack::WorkspaceDir::as_str).collect();
        let expected: Vec<&str> = held.iter().chain(wanted.iter()).map(String::as_str).fold(Vec::new(), |mut all, dir| {
            if !all.contains(&dir) { all.push(dir); }
            all
        });
        prop_assert_eq!(read, expected);
    }
}

/// Two aliases of one model: alz takes the one its merged `models.yaml` lists first,
/// with integer-like names ahead of the others, as the spec's name order puts them; alphabetical order does not
/// decide. Expected values are the spec's, written out by hand.
#[test]
fn an_alias_tie_follows_the_order_the_stack_defines_them_in() {
    let model_of = |models: &str| {
        let world = world_with_models(models, MANIFEST, &[]);
        let plan = run(&world, &[(".claude/agents/architect.md", ARCHITECT)]).expect("a plan");
        file(&plan, "agents/architect.md")
            .lines()
            .find_map(|line| line.strip_prefix("model: "))
            .map(str::to_owned)
            .expect("a model line")
    };
    assert_eq!(
        model_of("aliases:\n  zeta: { claude: opus }\n  alpha: { claude: opus }\n"),
        "zeta"
    );
    assert_eq!(
        model_of(
            "aliases:\n  zeta: { claude: opus }\n  \"10\": { claude: opus }\n  \"7\": { claude: opus }\n"
        ),
        "\"7\""
    );
}

/// The float of a native settings file is written as the shortest decimal that reads back (spec §6).
/// Expected text is written out by hand.
#[test]
fn a_float_of_native_settings_is_written_as_a_decimal_number() {
    let world = world(MANIFEST, &[]);
    let settings = r#"{"skillListingBudgetFraction": 0.5, "cleanupPeriodDays": 30}"#;
    let plan = run(&world, &[(".claude/settings.json", settings)]).expect("a plan");
    assert_eq!(
        file(&plan, "settings.yaml"),
        "claude:\n  skillListingBudgetFraction: 0.5\n  cleanupPeriodDays: 30\n"
    );
}

/// The emitter writes a null as `null`; whether the layer may hold it is the schema's
/// to say, and the settings schema has no key that takes one, so the reader check refuses the plan.
#[test]
fn a_null_of_native_settings_is_refused_by_the_layer_reader_not_by_the_emitter() {
    let world = world(MANIFEST, &[]);
    let settings = r#"{"sandbox": {"ignoreViolations": {"x": null}}}"#;
    let refused = run(&world, &[(".claude/settings.json", settings)]).expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines,
        [
            "error [adopt] .claude/settings.json: adopting it would write .layer/settings.yaml with text the layer reader refuses: 'claude.sandbox.ignoreViolations.*' must be a list whose items are text; fix it in .claude/settings.json and rerun"
        ]
    );
}

/// A float of a native agent stays refused: the Claude agent writer does not write one.
#[test]
fn a_float_or_a_null_of_a_native_agent_is_still_refused() {
    let world = world(MANIFEST, &[]);
    for field in ["maxTurns: 2.5", "maxTurns: null"] {
        let agent = format!("---\nname: a\ndescription: d\n{field}\n---\nBody.\n");
        let refused = run(&world, &[(".claude/agents/a.md", &agent)]).expect_err("refused");
        assert_eq!(refused.diagnostics().iter().count(), 1, "{field}");
    }
}

/// §4.3 "Adoption, in detail": the edited `layer.yaml` must parse as a manifest that holds the added
/// values and is otherwise unchanged, or nothing is adopted. A value the manifest does not accept
/// (an empty or absolute directory, a server id with a space) makes the edited text not read
/// back, so the edit is refused, and the whole plan with it.
#[test]
fn an_edited_manifest_that_does_not_read_back_adopts_nothing() {
    let base = "stack:\n  - source: path:./.layer\n";
    for dir in ["", "/abs", "x\u{85}y"] {
        assert_eq!(
            edited(base, &[dir], &[]),
            Err(Refused::ReadsBack),
            "{dir:?}"
        );
    }
    assert_eq!(edited(base, &[], &["a b"]), Err(Refused::ReadsBack));
    // In a plan the refusal is one problem naming layer.yaml and the values to add by hand, and
    // the native files that did convert are not planned either.
    let world = world(MANIFEST, &[]);
    let settings = r#"{"permissions": {"additionalDirectories": [""]}}"#;
    let refused = run(
        &world,
        &[
            (".claude/settings.json", settings),
            (".claude/rules/a.md", "x\n"),
        ],
    )
    .expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(
            "error [adopt] layer.yaml: would not read back as the same manifest after the edit; add by hand workspace.add_dirs: "
        ),
        "{lines:?}"
    );
}

/// §4.3 "Adoption, in detail": a whole number beyond 64 bits is refused, by one problem that names
/// the native file and never prints the number.
#[test]
fn a_whole_number_beyond_64_bits_is_refused_naming_the_native_file() {
    let world = world(MANIFEST, &[]);
    for (path, text) in [
        (
            ".claude/settings.json",
            r#"{"cleanupPeriodDays": 18446744073709551616}"#,
        ),
        (
            ".claude/settings.json",
            r#"{"cleanupPeriodDays": -9223372036854775809}"#,
        ),
        (
            ".mcp.json",
            r#"{"mcpServers": {"a": {"command": "x", "timeout": 18446744073709551616}}}"#,
        ),
    ] {
        let refused = run(&world, &[(path, text)]).expect_err(text);
        let lines = problems(&refused);
        assert_eq!(lines.len(), 1, "{text}: {lines:?}");
        assert!(
            lines[0].starts_with(&format!("error [adopt] {path}: ")),
            "{lines:?}"
        );
        assert!(!lines[0].contains("18446744073709551616"), "{lines:?}");
        assert!(!lines[0].contains("9223372036854775809"), "{lines:?}");
    }
}

/// §4.3 "Adoption, in detail": the reader check of the planned layer runs only when every native
/// file converted. A native file that cannot convert hides the reader's problem, and the reader's
/// problem shows in the run after it is fixed.
#[test]
fn the_reader_check_runs_only_after_every_conversion_succeeded() {
    let world = world(MANIFEST, &[]);
    let literal =
        r#"{"mcpServers": {"t": {"command": "t", "env": {"TOKEN": "tk-not-a-real-value"}}}}"#;
    let reader_only = "{ \"env\": { \"A\": \"line\u{2028}separator\" } }";

    let first = run(
        &world,
        &[
            (".mcp.json", literal),
            (".claude/settings.json", reader_only),
        ],
    )
    .expect_err("refused");
    let first = problems(&first);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(
        first[0].starts_with("error [adopt] .mcp.json: "),
        "{first:?}"
    );
    assert!(
        first
            .iter()
            .all(|line| !line.contains("with text the layer reader refuses")),
        "{first:?}"
    );

    let second = run(&world, &[(".claude/settings.json", reader_only)]).expect_err("refused");
    let second = problems(&second);
    assert_eq!(second.len(), 1, "{second:?}");
    assert!(
        second[0].contains(
            ".claude/settings.json: adopting it would write .layer/settings.yaml with text the layer reader refuses: "
        ),
        "{second:?}"
    );
}

/// §4.3 "Adoption, in detail": a layer file that adoption adds to is merged key by key, and a value
/// of another shape than the one on disk (a number where the file has another number, a list where
/// it has a scalar) is a problem that names the native file, not an overwrite.
#[test]
fn a_value_of_another_shape_than_the_one_on_disk_is_a_problem() {
    let world = world(
        MANIFEST,
        &[("settings.yaml", "claude:\n  cleanupPeriodDays: 7\n")],
    );
    let refused = run(
        &world,
        &[(".claude/settings.json", r#"{"cleanupPeriodDays": 30}"#)],
    )
    .expect_err("refused");
    let lines = problems(&refused);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("error [adopt] .claude/settings.json: ")
            && lines[0].contains("settings.yaml has a different shape; merge it by hand"),
        "{lines:?}"
    );
    // The same value is no conflict: the file is merged and written again.
    let same = run(
        &world,
        &[(".claude/settings.json", r#"{"cleanupPeriodDays": 7}"#)],
    )
    .expect("a plan");
    assert_eq!(
        file(&same, "settings.yaml"),
        "claude:\n  cleanupPeriodDays: 7\n"
    );
}
