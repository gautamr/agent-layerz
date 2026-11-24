//! `.claude/settings.json`, the copied hook scripts and `.mcp.json`: byte for byte against the
//! reference's output for the fixtures (`tests/fixtures/claude-settings`), and the rules of §8.1
//! and §7.6 case by case.

use std::collections::BTreeSet;

use proptest::prelude::{prop_assert_eq, proptest};

use super::super::ClaudeSettingsFiles;
use crate::json::{JsonValue, parse};
use crate::lint::parse_document;
use crate::merge::testing::{layer_executable, merged_with};
use crate::render::tree::{FileMode, OutputPath};
use crate::stack::Manifest;

/// The text of a file of an acceptance fixture, read at compile time.
macro_rules! fixture {
    ($path:literal) => {
        include_str!(concat!("../../../../../../test/fixtures/", $path))
    };
}

/// The text of a file of the reference's output.
macro_rules! golden {
    ($path:literal) => {
        include_str!(concat!(
            "../../../../tests/fixtures/claude-settings/",
            $path
        ))
    };
}

/// A layer: its `layer.yaml`, its other files, and the files with the owner-execute bit.
struct Part<'a> {
    yaml: &'a str,
    files: Vec<(&'a str, &'a str)>,
    executable: &'a [&'a str],
}

impl<'a> Part<'a> {
    fn new(yaml: &'a str, files: Vec<(&'a str, &'a str)>) -> Self {
        Self {
            yaml,
            files,
            executable: &[],
        }
    }

    fn executable(self, executable: &'a [&'a str]) -> Self {
        Self { executable, ..self }
    }
}

/// A file of the reference's output: its path, its text and its mode.
type Golden = (&'static str, &'static str, FileMode);

const SCRIPT: &str = "hooks/scripts/block-secrets.sh";
const REPO: &str = "name: shop-api\nkind: repo\nvisibility: private\n";

/// Renders the stack under a manifest that adds `options` to its stack.
fn render(parts: &[Part<'_>], options: &str) -> ClaudeSettingsFiles {
    let layers = parts
        .iter()
        .map(|part| layer_executable(part.yaml, &part.files, None, part.executable))
        .collect();
    let found = merged_with(layers, options);
    assert!(!found.diagnostics().has_errors(), "{}", found.diagnostics());
    ClaudeSettingsFiles::of(
        found.merged().expect("no error was found"),
        &workspace(options),
    )
}

fn workspace(options: &str) -> crate::stack::Workspace {
    let manifest = format!("stack:\n  - source: path:./.layer\n{options}");
    let node = parse_document(manifest.as_bytes()).expect("valid YAML");
    Manifest::from_yaml(&node)
        .expect("a manifest")
        .workspace()
        .clone()
}

fn path(text: &str) -> OutputPath {
    OutputPath::try_from(text).expect("a safe path")
}

fn lines(files: &ClaudeSettingsFiles) -> Vec<String> {
    files
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Compares the paths, the bytes and the modes with the reference's output.
fn matches_reference(files: &ClaudeSettingsFiles, golden: &[Golden]) {
    assert!(!files.diagnostics().has_errors(), "{}", files.diagnostics());
    let made: Vec<&str> = files.tree().paths().map(OutputPath::as_str).collect();
    let mut wanted: Vec<&str> = golden.iter().map(|(name, _, _)| *name).collect();
    wanted.sort_unstable();
    assert_eq!(made, wanted);
    for (name, text, mode) in golden {
        let file = files.tree().get(&path(name)).expect("a made file");
        assert_eq!(
            String::from_utf8_lossy(&file.bytes),
            *text,
            "{name} differs from the reference"
        );
        assert_eq!(file.mode, *mode, "{name}");
    }
}

fn personal(yaml: &'static str, files: Vec<(&'static str, &'static str)>) -> Part<'static> {
    Part::new(yaml, files)
}

#[test]
fn fixture_01_matches_the_reference_byte_for_byte() {
    let parts: Vec<Part<'_>> = super::stacks::CASE_01
        .iter()
        .map(|layer| Part::new(layer.yaml, layer.files.to_vec()).executable(&[SCRIPT]))
        .collect();
    let files = render(
        &parts,
        "options:\n  strict_slots: true\nmcp:\n  enable: [context7]\n",
    );
    matches_reference(
        &files,
        &[
            (
                ".claude/hooks/personal/scripts/block-secrets.sh",
                golden!("01-determinism/.claude/hooks/personal/scripts/block-secrets.sh"),
                FileMode::Executable,
            ),
            (
                ".claude/settings.json",
                golden!("01-determinism/.claude/settings.json"),
                FileMode::Regular,
            ),
            (
                ".mcp.json",
                golden!("01-determinism/.mcp.json"),
                FileMode::Regular,
            ),
        ],
    );
    assert_eq!(
        lines(&files),
        Vec::<String>::new(),
        "no hook-on-fail warning"
    );
}

#[test]
fn fixture_07_matches_the_reference_byte_for_byte() {
    let parts = [
        personal(
            fixture!("07-permission-floor/personal/layer.yaml"),
            vec![(
                "permissions.yaml",
                fixture!("07-permission-floor/personal/permissions.yaml"),
            )],
        ),
        personal(
            fixture!("07-permission-floor/repo-combine/.layer/layer.yaml"),
            vec![(
                "permissions.yaml",
                fixture!("07-permission-floor/repo-combine/.layer/permissions.yaml"),
            )],
        ),
    ];
    matches_reference(
        &render(&parts, ""),
        &[(
            ".claude/settings.json",
            golden!("07-permission-floor/.claude/settings.json"),
            FileMode::Regular,
        )],
    );
}

#[test]
fn fixture_08_matches_the_reference_byte_for_byte() {
    let scripts = ["hooks/scripts/block-secrets.sh", "hooks/scripts/format.sh"];
    let parts = [
        personal(
            fixture!("08-locked-hook/personal/layer.yaml"),
            vec![
                (
                    "hooks/hooks.yaml",
                    fixture!("08-locked-hook/personal/hooks/hooks.yaml"),
                ),
                (
                    "hooks/scripts/block-secrets.sh",
                    fixture!("08-locked-hook/personal/hooks/scripts/block-secrets.sh"),
                ),
                (
                    "hooks/scripts/format.sh",
                    fixture!("08-locked-hook/personal/hooks/scripts/format.sh"),
                ),
            ],
        )
        .executable(&scripts),
        personal(
            fixture!("08-locked-hook/repo-unlocked/.layer/layer.yaml"),
            vec![(
                "hooks/hooks.yaml",
                fixture!("08-locked-hook/repo-unlocked/.layer/hooks/hooks.yaml"),
            )],
        ),
    ];
    matches_reference(
        &render(&parts, ""),
        &[
            (
                ".claude/hooks/personal/scripts/block-secrets.sh",
                golden!("08-locked-hook/.claude/hooks/personal/scripts/block-secrets.sh"),
                FileMode::Executable,
            ),
            (
                ".claude/settings.json",
                golden!("08-locked-hook/.claude/settings.json"),
                FileMode::Regular,
            ),
        ],
    );
}

#[test]
fn fixture_09_matches_the_reference_byte_for_byte() {
    let parts = [
        personal(
            fixture!("09-mcp-allowlist/personal/layer.yaml"),
            vec![("mcp.yaml", fixture!("09-mcp-allowlist/personal/mcp.yaml"))],
        ),
        personal(fixture!("09-mcp-allowlist/repo/.layer/layer.yaml"), vec![]),
    ];
    matches_reference(
        &render(&parts, "mcp:\n  enable: [context7]\n"),
        &[(
            ".mcp.json",
            golden!("09-mcp-allowlist/.mcp.json"),
            FileMode::Regular,
        )],
    );
}

#[test]
fn fixture_16_grants_the_hub_as_an_additional_directory_like_the_reference() {
    let parts = [
        personal(fixture!("16-hub/personal/layer.yaml"), vec![]),
        personal(fixture!("16-hub/hub/layer/layer.yaml"), vec![]),
        personal(fixture!("16-hub/repo-a/.layer/layer.yaml"), vec![]),
    ];
    matches_reference(
        &render(&parts, "workspace:\n  hub: ../hub\n"),
        &[(
            ".claude/settings.json",
            golden!("16-hub/.claude/settings.json"),
            FileMode::Regular,
        )],
    );
}

#[test]
fn fixture_19_matches_the_reference_byte_for_byte() {
    let parts = [
        personal(fixture!("19-hook-commands/personal/layer.yaml"), vec![]),
        personal(
            fixture!("19-hook-commands/repo/.layer/layer.yaml"),
            vec![(
                "hooks/hooks.yaml",
                fixture!("19-hook-commands/repo/.layer/hooks/hooks.yaml"),
            )],
        ),
    ];
    matches_reference(
        &render(&parts, ""),
        &[(
            ".claude/settings.json",
            golden!("19-hook-commands/.claude/settings.json"),
            FileMode::Regular,
        )],
    );
}

/// The stack of every key the output has: all events, a script two hooks run, a hook for Codex
/// only, every transport, the default model, settings of the target and workspace folders.
fn mix() -> ClaudeSettingsFiles {
    let parts = [
        personal(
            golden!("mix/input/personal/layer.yaml"),
            vec![
                (
                    "hooks/hooks.yaml",
                    golden!("mix/input/personal/hooks/hooks.yaml"),
                ),
                (
                    "hooks/scripts/guard.sh",
                    golden!("mix/input/personal/hooks/scripts/guard.sh"),
                ),
                (
                    "hooks/scripts/fmt.sh",
                    golden!("mix/input/personal/hooks/scripts/fmt.sh"),
                ),
                ("mcp.yaml", golden!("mix/input/personal/mcp.yaml")),
                ("models.yaml", golden!("mix/input/personal/models.yaml")),
                (
                    "permissions.yaml",
                    golden!("mix/input/personal/permissions.yaml"),
                ),
                ("settings.yaml", golden!("mix/input/personal/settings.yaml")),
            ],
        )
        .executable(&["hooks/scripts/guard.sh"]),
        personal(REPO, vec![]),
    ];
    render(
        &parts,
        "mcp:\n  enable: [zeta, alpha, beta, plain, empty]\nworkspace:\n  hub: ../hub\n  add_dirs: [../shared, ../hub, ../other]\n",
    )
}

#[test]
fn the_mixed_stack_matches_the_reference_byte_for_byte() {
    let files = mix();
    matches_reference(
        &files,
        &[
            (
                ".claude/hooks/personal/scripts/fmt.sh",
                golden!("mix/.claude/hooks/personal/scripts/fmt.sh"),
                FileMode::Regular,
            ),
            (
                ".claude/hooks/personal/scripts/guard.sh",
                golden!("mix/.claude/hooks/personal/scripts/guard.sh"),
                FileMode::Executable,
            ),
            (
                ".claude/settings.json",
                golden!("mix/.claude/settings.json"),
                FileMode::Regular,
            ),
            (".mcp.json", golden!("mix/.mcp.json"), FileMode::Regular),
        ],
    );
}

#[test]
fn args_written_empty_are_written_and_absent_args_are_left_out_like_the_reference() {
    // `plain` has no `args` and `empty` has `args: []`; the golden is the reference's output.
    let files = mix();
    let text = String::from_utf8(
        files
            .tree()
            .get(&path(".mcp.json"))
            .expect("an mcp file")
            .bytes
            .clone(),
    )
    .expect("UTF-8");
    assert!(
        text.contains("\"empty\": {\n      \"args\": [],\n      \"command\": \"./empty\"\n    }")
    );
    assert!(text.contains("\"plain\": {\n      \"command\": \"./plain\"\n    }"));
}

#[test]
fn deny_on_stop_subagent_stop_and_post_tool_gives_the_hook_on_fail_warnings() {
    assert_eq!(
        lines(&mix()),
        [
            "warning [hook-on-fail] personal:hooks/hooks.yaml: hook 'fmt': post_tool cannot block; a failure is shown to Claude",
            "warning [hook-on-fail] personal:hooks/hooks.yaml: hook 'finish': on_fail: deny has no effect on stop in Claude; the script must exit 2 itself",
            "warning [hook-on-fail] personal:hooks/hooks.yaml: hook 'sub-finish': on_fail: deny has no effect on subagent_stop in Claude; the script must exit 2 itself",
        ]
    );
}

#[test]
fn nothing_is_written_when_nothing_is_set_and_no_server_is_enabled() {
    let files = render(
        &[Part::new(
            REPO,
            vec![(
                "mcp.yaml",
                "a:\n  transport: http\n  url: https://a.invalid/mcp\n",
            )],
        )],
        "",
    );
    assert_eq!(files.tree().len(), 0);
    assert!(files.diagnostics().iter().next().is_none());
}

#[test]
fn settings_keys_are_in_code_point_order_whole_numbers_included() {
    // U+FF61 is one UTF-16 unit above the high surrogate of U+1F600: UTF-16 order puts the emoji
    // first, code point order puts it last (spec §6, TASK-64). `modelOverrides` is a free map of
    // text, so it takes any key (TASK-79).
    let settings = "claude:\n  modelOverrides:\n    \"\u{FF61}\": m\n    \"\u{1F600}\": m\n    \"10\": m\n    \"9\": m\n    \"-x\": m\n    \"a\": m\n";
    let files = render(&[Part::new(REPO, vec![("settings.yaml", settings)])], "");
    let text = String::from_utf8(
        files
            .tree()
            .get(&path(".claude/settings.json"))
            .expect("a settings file")
            .bytes
            .clone(),
    )
    .expect("UTF-8");
    let keys: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("    \""))
        .filter_map(|line| line.split_once("\": "))
        .map(|(key, _)| key)
        .collect();
    assert_eq!(keys, ["-x", "10", "9", "a", "\u{FF61}", "\u{1F600}"]);
}

#[test]
fn a_native_event_of_another_target_and_a_hook_for_codex_only_are_not_written() {
    let hooks = "- id: a\n  event: codex:Notification\n  command: ./a.sh\n- id: b\n  event: stop\n  command: ./b.sh\n  targets: [codex]\n";
    let files = render(&[Part::new(REPO, vec![("hooks/hooks.yaml", hooks)])], "");
    assert_eq!(files.tree().len(), 0);
}

/// The parsed `settings.json` of a layer that holds `permissions` and nothing else.
fn permissions_of(yaml: &str) -> Option<JsonValue> {
    let files = render(&[Part::new(REPO, vec![("permissions.yaml", yaml)])], "");
    files
        .tree()
        .get(&path(".claude/settings.json"))
        .map(|file| parse(&file.bytes).expect("settings.json is JSON"))
}

fn permission_key(settings: &JsonValue, key: &str) -> Option<JsonValue> {
    let permissions = match settings {
        JsonValue::Object(top) => top.get("permissions"),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => None,
    };
    match permissions {
        Some(JsonValue::Object(found)) => found.get(key).cloned(),
        Some(
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Text(_)
            | JsonValue::Array(_),
        )
        | None => None,
    }
}

const FLAGS: [(&str, &str); 3] = [
    ("disable_bypass_mode", "disableBypassPermissionsMode"),
    ("block_reads_outside", "blockReadsOutsideWorkingDirectories"),
    (
        "block_writes_outside",
        "blockWritesOutsideWorkingDirectories",
    ),
];

#[test]
fn the_floor_fields_have_the_types_claude_takes() {
    let settings = permissions_of(
        "disable_bypass_mode: true\nblock_reads_outside: true\nblock_writes_outside: true\n",
    )
    .expect("a settings file");
    assert_eq!(
        permission_key(&settings, "disableBypassPermissionsMode"),
        Some(JsonValue::Text("disable".to_owned()))
    );
    assert_eq!(
        permission_key(&settings, "blockReadsOutsideWorkingDirectories"),
        Some(JsonValue::Bool(true))
    );
    assert_eq!(
        permission_key(&settings, "blockWritesOutsideWorkingDirectories"),
        Some(JsonValue::Bool(true))
    );
}

#[test]
fn a_floor_field_that_is_false_or_unset_writes_nothing() {
    assert_eq!(
        permissions_of("block_reads_outside: false\nblock_writes_outside: false\n"),
        None
    );
    assert_eq!(
        permissions_of("allow: [read]\n")
            .map(|s| { FLAGS.map(|(_, key)| permission_key(&s, key)) }),
        Some([None, None, None])
    );
}

#[test]
fn a_later_layer_cannot_unset_a_floor_field_the_file_keeps_it() {
    let parts = [
        Part::new(
            "name: client\nkind: client\nvisibility: private\n",
            vec![("permissions.yaml", "block_writes_outside: true\n")],
        ),
        Part::new(REPO, vec![("permissions.yaml", "allow: [read]\n")]),
    ];
    let files = render(&parts, "");
    let settings = parse(
        &files
            .tree()
            .get(&path(".claude/settings.json"))
            .expect("a settings file")
            .bytes,
    )
    .expect("JSON");
    assert_eq!(
        permission_key(&settings, "blockWritesOutsideWorkingDirectories"),
        Some(JsonValue::Bool(true))
    );
}

#[test]
fn settings_yaml_cannot_write_a_permission_key() {
    let found = merged_with(
        vec![layer_executable(
            REPO,
            &[(
                "settings.yaml",
                "claude:\n  permissions:\n    disableBypassPermissionsMode: disable\n",
            )],
            None,
            &[],
        )],
        "",
    );
    assert!(found.diagnostics().has_errors());
    assert!(found.merged().is_none() || found.diagnostics().error_count() > 0);
}

proptest! {
    #[test]
    fn the_floor_keys_are_written_exactly_for_the_fields_a_layer_set(
        set in proptest::array::uniform3(proptest::bool::ANY),
        allow in proptest::bool::ANY,
    ) {
        let text: String = FLAGS
            .iter()
            .zip(set)
            .filter(|(_, on)| *on)
            .map(|((field, _), _)| format!("{field}: true\n"))
            .chain(allow.then(|| "allow: [read]\n".to_owned()))
            .collect();
        let settings = permissions_of(&text);
        for ((_, key), on) in FLAGS.iter().zip(set) {
            let found = settings.as_ref().and_then(|s| permission_key(s, key));
            prop_assert_eq!(found.is_some(), on, "{}", key);
        }
        prop_assert_eq!(settings.is_some(), allow || set.iter().any(|on| *on));
    }
}

fn hook_stack(entries: &[(usize, usize, usize)]) -> String {
    const EVENTS: [&str; 6] = [
        "session_start",
        "pre_tool",
        "post_tool",
        "stop",
        "subagent_stop",
        "prompt_submit",
    ];
    const ON_FAIL: [&str; 3] = ["deny", "warn", "feedback"];
    entries
        .iter()
        .enumerate()
        .map(|(index, (event, on_fail, action))| {
            let run = match action {
                0 => "command: ./c.sh".to_owned(),
                1 => "run: scripts/a.sh".to_owned(),
                _ => "run: scripts/b.sh".to_owned(),
            };
            format!(
                "- id: h{index}\n  event: {}\n  {run}\n  on_fail: {}\n",
                EVENTS[*event], ON_FAIL[*on_fail]
            )
        })
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn a_script_two_hooks_run_is_copied_once() {
    let hooks = hook_stack(&[(1, 1, 1), (2, 1, 1), (3, 1, 2)]);
    let files = render(
        &[Part::new(
            REPO,
            vec![
                ("hooks/hooks.yaml", &hooks),
                ("hooks/scripts/a.sh", "a\n"),
                ("hooks/scripts/b.sh", "b\n"),
            ],
        )],
        "",
    );
    let copies: Vec<&str> = files
        .tree()
        .paths()
        .map(OutputPath::as_str)
        .filter(|name| name.starts_with(".claude/hooks/"))
        .collect();
    assert_eq!(
        copies,
        [
            ".claude/hooks/shop-api/scripts/a.sh",
            ".claude/hooks/shop-api/scripts/b.sh"
        ]
    );
    assert!(!files.diagnostics().has_errors(), "{}", files.diagnostics());
}

proptest! {
    #[test]
    fn rendering_is_deterministic_and_every_output_is_a_path_of_the_tree(
        entries in proptest::collection::vec((0_usize..6, 0_usize..3, 0_usize..3), 0..8),
    ) {
        let hooks = hook_stack(&entries);
        let parts = [Part::new(
            REPO,
            vec![
                ("hooks/hooks.yaml", &hooks),
                ("hooks/scripts/a.sh", "a\n"),
                ("hooks/scripts/b.sh", "b\n"),
            ],
        )];
        let first = render(&parts, "");
        let second = render(&parts, "");
        prop_assert_eq!(&first, &second);
        let used: BTreeSet<&str> = entries
            .iter()
            .filter_map(|(_, _, action)| match action {
                1 => Some(".claude/hooks/shop-api/scripts/a.sh"),
                2 => Some(".claude/hooks/shop-api/scripts/b.sh"),
                _ => None,
            })
            .collect();
        let expected: BTreeSet<&str> = used
            .iter()
            .copied()
            .chain((!entries.is_empty()).then_some(".claude/settings.json"))
            .collect();
        let made: BTreeSet<&str> = first.tree().paths().map(OutputPath::as_str).collect();
        prop_assert_eq!(made, expected);
        prop_assert_eq!(first.tree().paths().count(), first.tree().len());
    }
}
