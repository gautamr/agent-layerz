use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use crate::item::{Settings, SettingsItems};
use crate::merge::Tree;
use crate::merge::testing::layer;
use crate::target::TargetName;

/// What the layer `personal` with this `settings.yaml` gives.
fn read(yaml: &str) -> (Option<Settings>, Vec<String>) {
    let (items, found) =
        SettingsItems::read(&layer("personal", "personal", &[("settings.yaml", yaml)]));
    (
        items.into_file().map(|(_, settings)| settings),
        found.iter().map(ToString::to_string).collect(),
    )
}

fn only(yaml: &str) -> Settings {
    let (settings, found) = read(yaml);
    assert_eq!(found, Vec::<String>::new());
    settings.expect("a settings file")
}

fn refused(yaml: &str) -> Vec<String> {
    let (settings, found) = read(yaml);
    assert_eq!(settings, None, "{yaml}");
    assert!(
        found.iter().all(|line| line.starts_with("error [")),
        "{found:?}"
    );
    found
}

#[test]
fn a_layer_without_settings_yaml_or_with_an_empty_one_has_no_settings() {
    let (items, found) = SettingsItems::read(&layer("personal", "personal", &[]));
    assert!(items.into_file().is_none());
    assert_eq!(found.iter().count(), 0);
    assert_eq!(read(""), (None, Vec::new()));
}

#[test]
fn the_settings_of_a_file_are_an_ordered_map_keyed_by_target() {
    let settings = only("codex:\n  b: 1\n  a: [x]\nclaude:\n  z: true\n  env:\n    FOO: x\n");
    let targets: Vec<TargetName> = settings.iter().map(|(target, _)| target).collect();
    assert_eq!(targets, [TargetName::Claude, TargetName::Codex]);
    let codex = settings.of(TargetName::Codex).expect("codex");
    let keys: Vec<&str> = codex.iter().map(|(key, _)| key).collect();
    assert_eq!(keys, ["a", "b"]);
    assert_eq!(codex.get("b"), Some(&Tree::Int(1)));
    assert!(settings.of(TargetName::Claude).is_some());
}

#[test]
fn a_file_may_name_one_target_only() {
    let settings = only("codex:\n  a: 1\n");
    assert!(settings.of(TargetName::Claude).is_none());
    assert!(settings.of(TargetName::Codex).is_some());
}

#[test]
fn a_top_level_key_that_is_not_a_known_target_is_refused_naming_the_file_whatever_the_key() {
    for key in [
        "__proto__",
        "constructor",
        "prototype",
        "toString",
        "hasOwnProperty",
        "Claude",
        "CLAUDE",
        "claude ",
        " codex",
        "claude.env",
        "codex2",
        "cursor",
        "env",
        "model",
        "_",
        "0",
        "true",
        "~",
    ] {
        for body in ["x: 1\n", "claude:\n    permissions: x\n", "claude: {}\n"] {
            let yaml = format!("\"{key}\":\n  {}\n", body.replace('\n', "\n  ").trim_end());
            assert_eq!(
                refused(&yaml),
                [format!(
                    "error [settings] personal:settings.yaml:1:1: unknown field '{key}'"
                )],
                "{key:?} over {body:?}"
            );
        }
    }
}

#[test]
fn no_settings_of_a_file_are_read_when_any_top_level_key_is_refused() {
    let found = refused("claude:\n  a: 1\ncodex:\n  b: 2\nother:\n  c: 3\n");
    assert_eq!(
        found,
        ["error [settings] personal:settings.yaml:5:1: unknown field 'other'"]
    );
}

#[test]
fn every_unknown_key_is_reported_and_one_that_is_not_text_is_named_by_its_kind() {
    assert_eq!(
        refused("1: x\nfirst: x\nclaude:\n  a: 1\nsecond: x\n"),
        [
            "error [settings] personal:settings.yaml:1:1: an unknown key that is a number",
            "error [settings] personal:settings.yaml:2:1: unknown field 'first'",
            "error [settings] personal:settings.yaml:5:1: unknown field 'second'",
        ]
    );
}

#[test]
fn a_key_from_the_file_is_printed_quoted_with_its_quotes_doubled_and_cut() {
    let found = refused("\"it's\": x\n");
    assert_eq!(
        found,
        ["error [settings] personal:settings.yaml:1:1: unknown field 'it''s'"]
    );
    let long = "k".repeat(80);
    let found = refused(&format!("{long}: x\n"));
    assert_eq!(
        found,
        [format!(
            "error [settings] personal:settings.yaml:1:1: unknown field '{}\u{2026}'",
            "k".repeat(64)
        )]
    );
}

#[test]
fn a_file_that_is_not_a_mapping_and_a_target_that_is_not_a_mapping_are_refused() {
    assert_eq!(
        refused("[claude]\n"),
        ["error [settings] personal:settings.yaml:1:1: the document must be a mapping"]
    );
    assert_eq!(
        refused("claude:\n"),
        ["error [settings] personal:settings.yaml:1:7: claude must be a mapping"]
    );
    assert_eq!(
        refused("claude: [a]\ncodex: x\n"),
        [
            "error [settings] personal:settings.yaml:1:9: claude must be a mapping",
            "error [settings] personal:settings.yaml:2:8: codex must be a mapping",
        ]
    );
}

#[test]
fn a_key_that_duplicates_a_first_class_item_is_refused_per_target_naming_the_file_and_the_key() {
    let claude = [
        ("permissions", "permissions.yaml"),
        ("hooks", "hooks/hooks.yaml"),
        ("model", "models.yaml (default)"),
        ("enabledMcpjsonServers", "mcp.enable in the repo manifest"),
        ("disabledMcpjsonServers", "mcp.enable in the repo manifest"),
        (
            "enableAllProjectMcpServers",
            "mcp.enable in the repo manifest",
        ),
        (
            "disableAllHooks",
            "hooks.yaml `disabled` (locked hooks cannot be switched off)",
        ),
    ];
    let codex = [
        ("model", "models.yaml"),
        ("mcp_servers", "mcp.yaml"),
        ("hooks", "hooks/hooks.yaml"),
        ("approval_policy", "permissions.yaml"),
        ("sandbox_mode", "permissions.yaml"),
    ];
    for (target, keys) in [("claude", &claude[..]), ("codex", &codex[..])] {
        for (key, owner) in keys {
            assert_eq!(
                refused(&format!("{target}:\n  {key}: true\n")),
                [format!(
                    "error [settings] personal:settings.yaml:2:3: '{target}.{key}' is set through {owner}"
                )],
                "{target}.{key}"
            );
        }
    }
}

#[test]
fn a_key_that_target_facts_corrected_is_refused_with_its_own_reason() {
    for (yaml, expected) in [
        (
            "claude:\n  mcpServers:\n    s: x\n",
            "error [settings] personal:settings.yaml:2:3: 'claude.mcpServers' is not set in settings.yaml: Claude Code documents no such settings key; servers are set through mcp.yaml",
        ),
        (
            "claude:\n  allowAllClaudeAiMcps: true\n",
            "error [settings] personal:settings.yaml:2:3: 'claude.allowAllClaudeAiMcps' is not set in settings.yaml: it is read from a user or managed scope, not from a project",
        ),
        (
            "codex:\n  profile: work\n",
            "error [settings] personal:settings.yaml:2:3: 'codex.profile' is not set in settings.yaml: removed by Codex 0.169.0 (use --profile)",
        ),
        (
            "codex:\n  profiles:\n    work:\n      model_provider: x\n",
            "error [settings] personal:settings.yaml:2:3: 'codex.profiles' is not set in settings.yaml: Codex ignores it in a project's config.toml",
        ),
        (
            "codex:\n  windows:\n    sandbox: elevated\n",
            "error [settings] personal:settings.yaml:2:3: 'codex.windows' is not set in settings.yaml: it is read from a user or managed scope, not from a project",
        ),
    ] {
        assert_eq!(refused(yaml), [expected], "{yaml}");
    }
}

#[test]
fn every_reserved_key_of_a_file_is_reported_and_the_other_target_is_not_confused() {
    assert_eq!(
        refused(
            "claude:\n  hooks: []\n  mcp_servers: x\n  model: y\ncodex:\n  mcpServers: z\n  hooks: []\n"
        ),
        [
            "error [settings] personal:settings.yaml:2:3: 'claude.hooks' is set through hooks/hooks.yaml",
            "error [settings] personal:settings.yaml:4:3: 'claude.model' is set through models.yaml (default)",
            "error [settings] personal:settings.yaml:7:3: 'codex.hooks' is set through hooks/hooks.yaml",
        ]
    );
}

#[test]
fn a_reserved_name_below_the_top_of_a_target_is_an_ordinary_key() {
    let settings = only(
        "claude:\n  env:\n    permissions: x\n  sandbox:\n    hooks: y\ncodex:\n  other:\n    work:\n      model: z\n",
    );
    assert!(settings.of(TargetName::Claude).is_some());
    assert!(settings.of(TargetName::Codex).is_some());
}

#[test]
fn a_reserved_key_does_not_hide_a_protected_variable_and_the_other_way_round() {
    let found = refused("claude:\n  disableAllHooks: true\n  env:\n    HOME: x\n");
    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .any(|line| line.starts_with("error [settings] "))
    );
    assert!(found.iter().any(|line| {
        line.starts_with(
            "error [protected-variable] personal:settings.yaml: claude.env names HOME;",
        )
    }));
}

#[test]
fn a_protected_variable_in_settings_is_refused_and_the_file_gives_no_settings() {
    assert_eq!(
        refused("claude:\n  env:\n    HOME: x\n"),
        [
            "error [protected-variable] personal:settings.yaml: claude.env names HOME; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"
        ]
    );
    assert_eq!(
        refused(
            "codex:\n  profiles:\n    work:\n      shell_environment_policy:\n        set:\n          codex_home: x\n"
        ),
        [
            "error [settings] personal:settings.yaml:2:3: 'codex.profiles' is not set in settings.yaml: Codex ignores it in a project's config.toml",
            "error [protected-variable] personal:settings.yaml: codex.shell_environment_policy.set names CODEX_HOME; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"
        ]
    );
}

#[test]
fn a_claude_env_is_not_looked_for_under_codex_and_a_policy_is_not_looked_for_under_claude() {
    only(
        "codex:\n  env:\n    HOME: x\nclaude:\n  shell_environment_policy:\n    set:\n      HOME: x\n",
    );
}

proptest! {
    #[test]
    fn a_top_level_key_is_accepted_exactly_when_it_is_a_target(key in "[A-Za-z_][A-Za-z0-9_]{0,12}") {
        let (settings, found) = read(&format!("{key}:\n  a: 1\n"));
        if key == "claude" || key == "codex" {
            prop_assert!(settings.is_some());
            prop_assert_eq!(found, Vec::<String>::new());
        } else {
            prop_assert_eq!(settings, None);
            prop_assert_eq!(found, [format!("error [settings] personal:settings.yaml:1:1: unknown field '{key}'")]);
        }
    }
}

#[test]
fn a_policy_that_is_not_a_mapping_is_a_settings_error_naming_the_key_and_the_file_gives_no_settings()
 {
    assert_eq!(
        refused("codex:\n  shell_environment_policy: HOME\n"),
        [
            "error [settings] personal:settings.yaml: codex.shell_environment_policy must be a mapping"
        ]
    );
    assert_eq!(
        refused("codex:\n  profiles:\n    work:\n      shell_environment_policy: [HOME]\n"),
        [
            "error [settings] personal:settings.yaml:2:3: 'codex.profiles' is not set in settings.yaml: Codex ignores it in a project's config.toml",
            "error [settings] personal:settings.yaml: codex.shell_environment_policy must be a mapping"
        ]
    );
}

#[test]
fn a_codex_key_that_duplicates_a_first_class_item_is_refused_at_any_depth() {
    for (yaml, expected) in [
        (
            "codex:\n  features:\n    hooks: false\n",
            "error [settings] personal:settings.yaml:3:5: 'codex.features.hooks' is set through hooks/hooks.yaml",
        ),
        (
            "codex:\n  features:\n    codex_hooks: false\n",
            "error [settings] personal:settings.yaml:3:5: 'codex.features.codex_hooks' is set through hooks/hooks.yaml",
        ),
        (
            "codex:\n  other:\n    work:\n      features:\n        hooks: false\n",
            "error [settings] personal:settings.yaml:5:9: 'codex.**.features.hooks' is set through hooks/hooks.yaml",
        ),
        (
            "codex:\n  other:\n    - features:\n        codex_hooks: false\n",
            "error [settings] personal:settings.yaml:4:9: 'codex.**.features.codex_hooks' is set through hooks/hooks.yaml",
        ),
    ] {
        assert_eq!(refused(yaml), [expected], "{yaml}");
    }
}

#[test]
fn a_refused_codex_key_never_prints_its_value_or_the_name_of_a_profile() {
    let found =
        refused("codex:\n  profiles:\n    sneaky-name:\n      approval_policy: secret-value\n");
    assert!(found.iter().all(|line| !line.contains("secret-value")));
    assert!(found.iter().all(|line| !line.contains("sneaky-name")));
}

#[test]
fn the_names_of_a_first_class_item_are_ordinary_keys_where_codex_does_not_read_them_as_one() {
    let settings = only(
        "claude:\n  features:\n    hooks: true\n  profiles:\n    work:\n      approval_policy: never\ncodex:\n  features:\n    other: true\n  other:\n    work:\n      model_reasoning_effort: high\n  hooks_note:\n    approval_policy: x\n",
    );
    assert!(settings.of(TargetName::Claude).is_some());
    assert!(settings.of(TargetName::Codex).is_some());
}

// The reserved keys of the table of native keys (§7.9).

/// The yaml of `target` that sets the key path of a row (a wildcard stands for the key `x`, and
/// `**` for nothing) to a scalar.
fn yaml_of(row: &crate::item::NativeKey) -> String {
    let keys: Vec<&str> = row
        .path()
        .split('.')
        .filter(|segment| *segment != "**")
        .map(|segment| if segment == "*" { "x" } else { segment })
        .collect();
    let flow = keys
        .iter()
        .rev()
        .fold("1".to_owned(), |inner, key| format!("{{{key}: {inner}}}"));
    format!("{}: {flow}\n", row.target())
}

#[test]
fn every_key_that_the_table_refuses_is_refused_with_its_path_and_no_value() {
    use crate::item::{KEYS, KeyRule, Step, native_key};
    for row in KEYS {
        let (KeyRule::FirstClass(_) | KeyRule::Reserved(_)) = row.rule() else {
            continue;
        };
        let found = refused(&yaml_of(row));
        // The first row that matches the path is the one that is named.
        let names: Vec<String> = row
            .path()
            .split('.')
            .filter(|segment| *segment != "**")
            .map(|segment| if segment == "*" { "x" } else { segment }.to_owned())
            .collect();
        let steps: Vec<_> = names.iter().map(|name| Step::Key(name)).collect();
        let first = native_key(row.target(), &steps)
            .expect("the table has the row")
            .display_path();
        assert!(
            found.iter().any(
                |line| line.starts_with("error [settings] personal:settings.yaml:")
                    && line.contains(&format!("'{first}'"))
            ),
            "{}: {found:?}",
            row.display_path()
        );
    }
}

#[test]
fn a_reserved_key_says_why_it_is_not_set() {
    assert_eq!(
        refused("claude:\n  apiKeyHelper: ./login.sh\n"),
        [
            "error [settings] personal:settings.yaml:2:3: 'claude.apiKeyHelper' is not set in settings.yaml: it runs a command and carries a login"
        ]
    );
    assert_eq!(
        refused("codex:\n  projects:\n    shop:\n      trust_level: trusted\n"),
        [
            "error [settings] personal:settings.yaml:4:7: 'codex.projects.*.trust_level' is not set in settings.yaml: a layer must not mark a project as trusted"
        ]
    );
    assert_eq!(
        refused("claude:\n  sandbox:\n    credentials:\n      helper: x\n"),
        [
            "error [settings] personal:settings.yaml:3:5: 'claude.sandbox.credentials' is not set in settings.yaml: it carries or pins a login"
        ]
    );
}

#[test]
fn a_floor_key_and_a_key_with_a_similar_name_are_read() {
    let settings = only(
        "claude:\n  sandbox:\n    enabled: true\n    network:\n      allowedDomains: [a.example]\n      httpProxyPortX: 1\n  statusLineX: 1\ncodex:\n  web_search: cached\n  projects:\n    shop:\n      other: 1\n",
    );
    assert!(settings.of(TargetName::Claude).is_some());
    assert!(settings.of(TargetName::Codex).is_some());
}

#[test]
fn a_reserved_key_is_refused_below_a_list_only_where_the_path_says_so() {
    // A mapping inside a list is not at the path of `sandbox.credentials`.
    only(
        "claude:\n  other:\n    - credentials: 1\n  sandbox:\n    list:\n      - credentials: 1\n",
    );
}
