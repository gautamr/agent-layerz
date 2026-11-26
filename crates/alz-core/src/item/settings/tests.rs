use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use crate::item::{Settings, SettingsItems};
use crate::merge::Tree;
use crate::merge::testing::layer;
use crate::target::Registry;
use alz_adapter::TargetId;

/// What the layer `personal` with this `settings.yaml` gives.
fn read(yaml: &str) -> (Option<Settings>, Vec<String>) {
    let (items, found) = SettingsItems::read(
        &layer("personal", "personal", &[("settings.yaml", yaml)]),
        &Registry::builtin(),
    );
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
    let (items, found) =
        SettingsItems::read(&layer("personal", "personal", &[]), &Registry::builtin());
    assert!(items.into_file().is_none());
    assert_eq!(found.iter().count(), 0);
    assert_eq!(read(""), (None, Vec::new()));
}

#[test]
fn the_settings_of_a_file_are_an_ordered_map_keyed_by_target() {
    let settings = only("codex:\n  b: 1\n  a: [x]\nclaude:\n  verbose: true\n  env:\n    FOO: x\n");
    let targets: Vec<TargetId> = settings.iter().map(|(target, _)| target.clone()).collect();
    assert_eq!(
        targets,
        [
            crate::target::claude().clone(),
            crate::target::codex().clone()
        ]
    );
    let codex = settings.of(crate::target::codex()).expect("codex");
    let keys: Vec<&str> = codex.iter().map(|(key, _)| key).collect();
    assert_eq!(keys, ["a", "b"]);
    assert_eq!(codex.get("b"), Some(&Tree::Int(1)));
    assert!(settings.of(crate::target::claude()).is_some());
}

#[test]
fn a_file_may_name_one_target_only() {
    let settings = only("codex:\n  a: 1\n");
    assert!(settings.of(crate::target::claude()).is_none());
    assert!(settings.of(crate::target::codex()).is_some());
}

#[test]
fn a_top_level_key_that_is_not_a_known_target_is_refused_naming_the_file_whatever_the_key() {
    for key in [
        "__proto__",
        "Constructor",
        "Prototype",
        "toString",
        "hasOwnProperty",
        "Claude",
        "CLAUDE",
        "claude ",
        " codex",
        "claude.env",
        "Codex2",
        "Cursor",
        "Env",
        "Model",
        "_",
        "0",
        "True",
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
    let found = refused("claude:\n  verbose: true\ncodex:\n  b: 2\nOther:\n  c: 3\n");
    assert_eq!(
        found,
        ["error [settings] personal:settings.yaml:5:1: unknown field 'Other'"]
    );
}

#[test]
fn every_unknown_key_is_reported_and_one_that_is_not_text_is_named_by_its_kind() {
    assert_eq!(
        refused("1: x\nFirst: x\nclaude:\n  verbose: true\nSecond: x\n"),
        [
            "error [settings] personal:settings.yaml:1:1: an unknown key that is a number",
            "error [settings] personal:settings.yaml:2:1: unknown field 'First'",
            "error [settings] personal:settings.yaml:5:1: unknown field 'Second'",
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
            "claude:\n  hooks: []\n  verbose: true\n  model: y\ncodex:\n  mcpServers: z\n  hooks: []\n"
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
        "claude:\n  env:\n    permissions: x\n    hooks: y\ncodex:\n  other:\n    work:\n      model: z\n",
    );
    assert!(settings.of(crate::target::claude()).is_some());
    assert!(settings.of(crate::target::codex()).is_some());
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
    only("codex:\n  env:\n    HOME: x\nclaude:\n  env:\n    shell_environment_policy: x\n");
    // Under Claude a policy is no key of the table, so it is refused as such, not as a variable.
    assert_eq!(
        refused("claude:\n  shell_environment_policy:\n    set:\n      HOME: x\n"),
        [
            "error [settings] personal:settings.yaml:2:3: 'claude.shell_environment_policy' is not a Claude Code settings key that alz accepts (docs/target-facts.md section 14)"
        ]
    );
}

proptest! {
    #[test]
    fn a_top_level_key_is_accepted_exactly_when_it_is_a_target(key in "[A-Za-z_][A-Za-z0-9_]{0,12}") {
        let (settings, found) = read(&format!("{key}:\n  verbose: true\n"));
        if alz_adapter::TargetId::try_from(key.as_str()).is_ok() {
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
        "claude:\n  env:\n    features: x\n    profiles: y\ncodex:\n  features:\n    other: true\n  other:\n    work:\n      model_reasoning_effort: high\n  hooks_note:\n    approval_policy: x\n",
    );
    assert!(settings.of(crate::target::claude()).is_some());
    assert!(settings.of(crate::target::codex()).is_some());
}

// The reserved keys of the table of native keys (§7.9).

/// The yaml of `target` that sets the key path of a row (a wildcard stands for the key `x`, and
/// `**` for nothing) to a scalar.
fn yaml_of(row: &crate::item::NativeKey<'_>) -> String {
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
    use crate::item::{KeyRule, Step};
    let registry = Registry::builtin();
    for target in [crate::target::claude(), crate::target::codex()] {
        let rules = registry
            .key_rules(target)
            .expect("claude has an adapter and codex a descriptor-only entry");
        for row in rules.rows() {
            let (KeyRule::FirstClass(_) | KeyRule::Reserved(_)) = row.rule() else {
                continue;
            };
            let found = refused(&yaml_of(&row));
            // The first row that matches the path is the one that is named.
            let names: Vec<String> = row
                .path()
                .split('.')
                .filter(|segment| *segment != "**")
                .map(|segment| if segment == "*" { "x" } else { segment }.to_owned())
                .collect();
            let steps: Vec<_> = names.iter().map(|name| Step::Key(name)).collect();
            let first = rules
                .native_key(&steps)
                .expect("the table has the row")
                .display_path();
            assert!(
                found.iter().any(|line| line
                    .starts_with("error [settings] personal:settings.yaml:")
                    && line.contains(&format!("'{first}'"))),
                "{}: {found:?}",
                row.display_path()
            );
        }
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
        "claude:\n  sandbox:\n    enabled: true\n    network:\n      allowedDomains: [a.example]\n  env:\n    httpProxyPortX: x\n    statusLineX: x\ncodex:\n  web_search: cached\n  projects:\n    shop:\n      other: 1\n",
    );
    assert!(settings.of(crate::target::claude()).is_some());
    assert!(settings.of(crate::target::codex()).is_some());
}

#[test]
fn a_reserved_key_is_refused_below_a_list_only_where_the_path_says_so() {
    // A mapping inside a list is not at the path of `sandbox.credentials`.
    only(
        "codex:\n  other:\n    - trust_level: 1\n  projects:\n    list:\n      - trust_level: 1\n",
    );
}

// The types of Claude's settings keys (§7.9, TASK-79).

const UNLISTED: &str =
    "is not a Claude Code settings key that alz accepts (docs/target-facts.md section 14)";

#[test]
fn a_value_of_the_wrong_type_is_a_settings_error_naming_the_key_path_and_never_the_value() {
    for (yaml, expected) in [
        (
            "claude:\n  verbose: secret-value\n",
            "error [settings] personal:settings.yaml:2:12: 'claude.verbose' must be a boolean",
        ),
        (
            "claude:\n  language: [secret-value]\n",
            "error [settings] personal:settings.yaml:2:13: 'claude.language' must be text",
        ),
        (
            "claude:\n  cleanupPeriodDays: secret-value\n",
            "error [settings] personal:settings.yaml:2:22: 'claude.cleanupPeriodDays' must be a whole number of at least 1",
        ),
        (
            "claude:\n  cleanupPeriodDays: 1.5\n",
            "error [settings] personal:settings.yaml:2:22: 'claude.cleanupPeriodDays' must be a whole number of at least 1",
        ),
        (
            "claude:\n  feedbackSurveyRate: secret-value\n",
            "error [settings] personal:settings.yaml:2:23: 'claude.feedbackSurveyRate' must be a number from 0 to 1",
        ),
        (
            "claude:\n  editorMode: secret-value\n",
            "error [settings] personal:settings.yaml:2:15: 'claude.editorMode' must be one of normal, vim",
        ),
        (
            "claude:\n  availableModels: secret-value\n",
            "error [settings] personal:settings.yaml:2:20: 'claude.availableModels' must be a list whose items are text",
        ),
        (
            "claude:\n  availableModels: [a, {k: secret-value}]\n",
            "error [settings] personal:settings.yaml:2:24: 'claude.availableModels[]' must be text",
        ),
        (
            "claude:\n  env:\n    FOO: 1\n",
            "error [settings] personal:settings.yaml:3:10: 'claude.env.*' must be text",
        ),
        (
            "claude:\n  sandbox: secret-value\n",
            "error [settings] personal:settings.yaml:2:12: 'claude.sandbox' must be a mapping",
        ),
        (
            "claude:\n  sandbox:\n    network:\n      allowLocalBinding: secret-value\n",
            "error [settings] personal:settings.yaml:4:26: 'claude.sandbox.network.allowLocalBinding' must be a boolean",
        ),
        (
            "claude:\n  attribution: true\n",
            "error [settings] personal:settings.yaml:2:16: 'claude.attribution' must be a mapping, or the boolean false",
        ),
        (
            "claude:\n  allowedMcpServers:\n    - {serverName: a, serverUrl: b}\n",
            "error [settings] personal:settings.yaml:3:7: 'claude.allowedMcpServers[]' must be a mapping with exactly one of the keys serverName, serverCommand, serverUrl",
        ),
    ] {
        let found = refused(yaml);
        assert_eq!(found, [expected], "{yaml}");
        assert!(found.iter().all(|line| !line.contains("secret-value")));
    }
}

#[test]
fn an_env_that_is_not_a_mapping_is_a_type_error_and_a_variable_error_without_the_value() {
    let found = refused("claude:\n  env: secret-value\n");
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0],
        "error [settings] personal:settings.yaml:2:8: 'claude.env' must be a mapping whose values are text"
    );
    assert!(found.iter().all(|line| !line.contains("secret-value")));
}

#[test]
fn a_key_that_the_table_does_not_list_is_a_settings_error_at_any_depth() {
    for (yaml, expected) in [
        (
            "claude:\n  bogusKey: 1\n",
            "error [settings] personal:settings.yaml:2:3: 'claude.bogusKey'",
        ),
        (
            "claude:\n  sandbox:\n    bogusKey: 1\n",
            "error [settings] personal:settings.yaml:3:5: 'claude.sandbox.bogusKey'",
        ),
        (
            "claude:\n  sandbox:\n    network:\n      bogusKey: 1\n",
            "error [settings] personal:settings.yaml:4:7: 'claude.sandbox.network.bogusKey'",
        ),
        (
            "claude:\n  modelSettings:\n    opus:\n      bogusKey: 1\n",
            "error [settings] personal:settings.yaml:4:7: 'claude.modelSettings.*.bogusKey'",
        ),
        (
            "claude:\n  spinnerTipsOverride:\n    tips:\n      - {id: a, text: b, bogusKey: 1}\n",
            "error [settings] personal:settings.yaml:4:26: 'claude.spinnerTipsOverride.tips[].bogusKey'",
        ),
    ] {
        assert_eq!(refused(yaml), [format!("{expected} {UNLISTED}")], "{yaml}");
    }
}

#[test]
fn a_key_that_claude_reads_from_a_user_or_managed_scope_only_is_not_accepted() {
    for key in [
        "askUserQuestionTimeout",
        "availableModelsMatch",
        "spellcheck",
    ] {
        assert_eq!(
            refused(&format!("claude:\n  {key}: x\n")),
            [format!(
                "error [settings] personal:settings.yaml:2:3: 'claude.{key}' {UNLISTED}"
            )]
        );
    }
}

#[test]
fn a_refused_key_is_reported_once_by_the_table_that_refuses_it_and_not_as_unlisted() {
    assert_eq!(
        refused("claude:\n  apiKeyHelper: x\n  sandbox:\n    credentials: 1\n"),
        [
            "error [settings] personal:settings.yaml:2:3: 'claude.apiKeyHelper' is not set in settings.yaml: it runs a command and carries a login",
            "error [settings] personal:settings.yaml:4:5: 'claude.sandbox.credentials' is not set in settings.yaml: it carries or pins a login",
        ]
    );
}

#[test]
fn a_typed_error_and_a_reserved_key_of_one_file_are_all_reported() {
    let found = refused("claude:\n  verbose: x\n  disableAllHooks: true\n  bogusKey: 1\n");
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn every_documented_shape_accepts_a_value_of_its_type() {
    only(
        "claude:\n  verbose: true\n  language: german\n  cleanupPeriodDays: 30\n  feedbackSurveyRate: 1\n  skillListingBudgetFraction: 0.5\n  editorMode: vim\n  availableModels: [a, b]\n  env:\n    FOO: x\n  attribution: false\n  allowedMcpServers:\n    - serverName: a\n    - {serverCommand: [a, b]}\n    - {serverUrl: 'https://a.example/*'}\n  sandbox:\n    enabled: true\n    network:\n      allowedDomains: [a.example]\n    ignoreViolations:\n      '*': [/tmp]\n  modelSettings:\n    opus:\n      effortLevel: high\n      autoCompactWindow: auto\n  spinnerTipsOverride:\n    tips: [plain, {id: a, text: b, priority: 1}]\n",
    );
    only("claude:\n  attribution:\n    commit: c\n    pr: p\n    sessionUrl: false\n");
}

#[test]
fn a_null_is_not_a_value_of_any_typed_key() {
    assert_eq!(
        refused("claude:\n  verbose:\n"),
        ["error [settings] personal:settings.yaml:2:10: 'claude.verbose' must be a boolean"]
    );
}

#[test]
fn codex_settings_are_not_typed_here() {
    only("codex:\n  anything: [1, {a: b}]\n  verbose: 3\n");
}

#[test]
fn a_key_that_an_object_requires_is_a_settings_error_when_it_is_missing() {
    assert_eq!(
        refused("claude:\n  spinnerVerbs:\n    verbs: [a]\n"),
        ["error [settings] personal:settings.yaml:3:5: 'claude.spinnerVerbs.mode' is required"]
    );
    assert_eq!(
        refused("claude:\n  spinnerVerbs: {}\n"),
        [
            "error [settings] personal:settings.yaml:2:17: 'claude.spinnerVerbs.verbs' is required",
            "error [settings] personal:settings.yaml:2:17: 'claude.spinnerVerbs.mode' is required",
        ]
    );
    assert_eq!(
        refused("claude:\n  spinnerTipsOverride:\n    tips: [{id: a}]\n"),
        [
            "error [settings] personal:settings.yaml:3:12: 'claude.spinnerTipsOverride.tips[].text' is required"
        ]
    );
}

#[test]
fn a_key_that_lowers_a_safety_check_or_steers_sign_in_is_refused_with_its_reason() {
    let lowers = "it turns off a safety check of Claude Code, which no layer may lower";
    let steers = "it steers how the user signs in, which a repository layer may not set";
    for (key, value, reason) in [
        ("skipWebFetchPreflight", "true", lowers),
        ("forceLoginMethod", "console", steers),
        ("forceLoginOrgUUID", "secret-value", steers),
        ("forceLoginOrgUUID", "[secret-value]", steers),
    ] {
        let found = refused(&format!("claude:\n  {key}: {value}\n"));
        assert_eq!(
            found,
            [format!(
                "error [settings] personal:settings.yaml:2:3: 'claude.{key}' is not set in settings.yaml: {reason}"
            )]
        );
        assert!(found.iter().all(|line| !line.contains("secret-value")));
    }
}

// Numbers: finite, and inside the range Claude Code documents (docs/target-facts.md section 14).

#[test]
fn a_number_that_is_not_finite_is_a_settings_error_naming_the_key_path_and_no_value() {
    for yaml in [
        "claude:\n  feedbackSurveyRate: .nan\n",
        "claude:\n  feedbackSurveyRate: .inf\n",
        "claude:\n  feedbackSurveyRate: -.inf\n",
        "claude:\n  skillListingBudgetFraction: .inf\n",
    ] {
        let found = refused(yaml);
        assert_eq!(found.len(), 1, "{yaml}");
        assert!(found[0].starts_with("error [settings] personal:settings.yaml:2:"));
        assert!(found[0].contains("'claude."), "{found:?}");
        assert!(!found[0].to_lowercase().contains("nan"), "{found:?}");
        assert!(!found[0].contains(".inf"), "{found:?}");
    }
}

#[test]
fn a_whole_number_outside_the_documented_range_is_refused_and_one_inside_is_read() {
    for (key, low, high) in [
        ("cleanupPeriodDays", "0", "1"),
        ("cleanupPeriodDays", "-3", "30"),
        ("autoCompactWindow", "99999", "100000"),
        ("bashOutputMaxChars", "0", "1"),
        ("bashOutputMaxChars", "-1", "999999"),
        ("skillListingMaxDescChars", "0", "1"),
        ("maxProseWidth", "39", "40"),
        ("maxProseWidth", "0", "80"),
    ] {
        let found = refused(&format!("claude:\n  {key}: {low}\n"));
        assert_eq!(found.len(), 1, "{key}: {low}");
        assert!(
            found[0].contains(&format!(
                "'claude.{key}' must be a whole number of at least"
            )),
            "{found:?}"
        );
        only(&format!("claude:\n  {key}: {high}\n"));
    }
}

#[test]
fn a_fraction_outside_zero_to_one_is_refused() {
    for (yaml, ok) in [
        ("claude:\n  skillListingBudgetFraction: 0\n", false),
        ("claude:\n  skillListingBudgetFraction: -0.5\n", false),
        ("claude:\n  skillListingBudgetFraction: 1.5\n", false),
        ("claude:\n  skillListingBudgetFraction: 2\n", false),
        ("claude:\n  skillListingBudgetFraction: 1\n", true),
        ("claude:\n  skillListingBudgetFraction: 0.02\n", true),
        ("claude:\n  feedbackSurveyRate: -0.1\n", false),
        ("claude:\n  feedbackSurveyRate: 1.5\n", false),
        ("claude:\n  feedbackSurveyRate: 0\n", true),
        ("claude:\n  feedbackSurveyRate: 1\n", true),
        ("claude:\n  feedbackSurveyRate: 0.05\n", true),
    ] {
        if ok {
            only(yaml);
        } else {
            assert_eq!(refused(yaml).len(), 1, "{yaml}");
        }
    }
}

#[test]
fn the_ranges_of_a_nested_number_hold() {
    let tip = |field: &str| {
        format!("claude:\n  spinnerTipsOverride:\n    tips:\n      - {{id: a, text: b, {field}}}\n")
    };
    for refusing in [
        "cooldownSessions: 1001",
        "cooldownSessions: -1",
        "priority: 11",
        "priority: -11",
    ] {
        assert_eq!(refused(&tip(refusing)).len(), 1, "{refusing}");
    }
    for fine in [
        "cooldownSessions: 0",
        "cooldownSessions: 1000",
        "priority: 10",
        "priority: -10",
    ] {
        only(&tip(fine));
    }
    assert_eq!(
        refused("claude:\n  modelSettings:\n    opus:\n      autoCompactWindow: 5\n").len(),
        1
    );
    only("claude:\n  modelSettings:\n    opus:\n      autoCompactWindow: 100000\n");
    only("claude:\n  modelSettings:\n    opus:\n      autoCompactWindow: auto\n");
}

#[test]
fn an_allowlist_entry_that_claude_would_skip_is_refused() {
    for yaml in [
        "claude:\n  allowedMcpServers:\n    - {serverName: has space}\n",
        "claude:\n  allowedMcpServers:\n    - {serverName: a.b}\n",
        "claude:\n  allowedMcpServers:\n    - {serverName: ''}\n",
        "claude:\n  allowedMcpServers:\n    - {serverCommand: []}\n",
    ] {
        assert_eq!(refused(yaml).len(), 1, "{yaml}");
    }
    only(
        "claude:\n  allowedMcpServers:\n    - {serverName: ok_name-1}\n    - {serverCommand: [bin]}\n",
    );
    only("claude:\n  deniedMcpServers:\n    - {serverName: has space}\n");
}

// The key rules come from the descriptor of the target (§8.0 **Descriptor.**, TASK-110).

/// What the layer `personal` with this `settings.yaml` gives when `registry` holds the adapters.
fn read_with(registry: &Registry, yaml: &str) -> (Option<Settings>, Vec<String>) {
    let (items, found) = SettingsItems::read(
        &layer("personal", "personal", &[("settings.yaml", yaml)]),
        registry,
    );
    (
        items.into_file().map(|(_, settings)| settings),
        found.iter().map(ToString::to_string).collect(),
    )
}

#[test]
fn a_target_section_is_checked_with_the_key_rules_of_its_descriptor() {
    use alz_adapter::{FirstClassKey, NativeVocabulary, ReservedKey};
    let registry = crate::merge::testing::registry_with_vocabulary(
        "probe",
        NativeVocabulary::default().with_key_rules(
            vec![FirstClassKey::new("model", "models.yaml")],
            vec![ReservedKey::new(
                "deep.secret",
                "the probe keeps it to itself",
            )],
            Vec::new(),
        ),
    );
    let (settings, found) = read_with(
        &registry,
        "probe:\n  model: x\n  deep:\n    secret: 1\n  other: 2\n",
    );
    assert_eq!(settings, None);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("'probe.model' is set through models.yaml"),
        "{found:?}"
    );
    assert!(
        found[1].contains(
            "'probe.deep.secret' is not set in settings.yaml: the probe keeps it to itself"
        ),
        "{found:?}"
    );
}

#[test]
fn a_target_with_neither_an_adapter_nor_a_descriptor_is_read_as_a_mapping_and_nothing_more() {
    // `claude` has no adapter in this registry, and `antigravity` never had one: both are read as
    // a mapping, whatever they hold.
    let registry = Registry::new();
    let (settings, found) = read_with(
        &registry,
        "claude:\n  hooks: {}\nantigravity:\n  permissions: 1\n",
    );
    assert_eq!(found, Vec::<String>::new());
    let settings = settings.expect("a settings file");
    assert!(settings.of(crate::target::claude()).is_some());
    assert!(settings.of(&crate::target::id("antigravity")).is_some());
}

#[test]
fn codex_is_validated_without_an_adapter_by_the_descriptor_only_entry_of_the_core() {
    let (settings, found) = read_with(&Registry::new(), "codex:\n  hooks: {}\n");
    assert_eq!(settings, None);
    assert_eq!(
        found,
        [
            "error [settings] personal:settings.yaml:2:3: 'codex.hooks' is set through hooks/hooks.yaml"
        ]
    );
}
