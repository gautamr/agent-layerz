use std::collections::BTreeMap;

use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use crate::account::PROTECTED_VARIABLES;
use crate::diagnostic::{Diagnostics, Where};
use crate::item::agent::tree_of;
use crate::lint::parse_document;
use crate::merge::Tree;
use crate::untrusted_text::Stripped;

use super::{ServerField, codex_policies, env_keys, paths, server_envs};
use crate::diagnostic::Code;

fn at() -> Where {
    Where::layer_file(
        Stripped::from_untrusted("personal"),
        Stripped::from_untrusted("settings.yaml"),
    )
}

fn tree(yaml: &str) -> BTreeMap<String, Tree> {
    let Tree::Map(entries) = tree_of(&parse_document(yaml.as_bytes()).expect("valid YAML")) else {
        panic!("a mapping was expected");
    };
    entries
}

fn lines(found: &Diagnostics) -> Vec<String> {
    found.iter().map(ToString::to_string).collect()
}

/// What the settings of the target `claude` say about `env`.
fn claude(yaml: &str) -> Vec<String> {
    let entries = tree(yaml);
    lines(&env_keys(entries.get("env"), paths::CLAUDE_ENV, &at()))
}

fn codex(yaml: &str) -> Vec<String> {
    lines(&codex_policies(&tree(yaml), Code::Settings, &at()))
}

fn names(path: &str, variable: &str) -> String {
    format!(
        "error [protected-variable] personal:settings.yaml: {path} names {variable}; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"
    )
}

/// One form of the acceptance cases of §7.9, written for the variable `name`.
type Form = (&'static str, &'static str, &'static str, fn(&str) -> String);

const POLICY_SET: &str = "codex.shell_environment_policy.set";
const POLICY_EXCLUDE: &str = "codex.shell_environment_policy.exclude";
const POLICY_FILTERS: &str = "codex.shell_environment_policy.filters";

fn forms() -> Vec<Form> {
    vec![
        ("claude env", "claude", "claude.env", |v| {
            format!("env:\n  {v}: x\n")
        }),
        ("codex set", "codex", POLICY_SET, |v| {
            format!("shell_environment_policy:\n  set:\n    {v}: x\n")
        }),
        ("codex set, lower case", "codex", POLICY_SET, |v| {
            format!(
                "shell_environment_policy:\n  set:\n    {}: x\n",
                v.to_lowercase()
            )
        }),
        ("codex exclude, exact", "codex", POLICY_EXCLUDE, |v| {
            format!("shell_environment_policy:\n  exclude: [{v}]\n")
        }),
        (
            "codex exclude, one-character wildcard",
            "codex",
            POLICY_EXCLUDE,
            |v| {
                format!(
                    "shell_environment_policy:\n  exclude: [\"?{}\"]\n",
                    v.get(1..).unwrap_or("")
                )
            },
        ),
        (
            "codex exclude, run wildcard and lower case",
            "codex",
            POLICY_EXCLUDE,
            |v| {
                format!(
                    "shell_environment_policy:\n  exclude: [\"*{}\"]\n",
                    v.get(2..).unwrap_or("").to_lowercase()
                )
            },
        ),
        (
            "codex exclude, *home* style",
            "codex",
            POLICY_EXCLUDE,
            |v| {
                format!(
                    "shell_environment_policy:\n  exclude: [\"*{}*\"]\n",
                    v.get(v.len().saturating_sub(3)..)
                        .unwrap_or("")
                        .to_lowercase()
                )
            },
        ),
        (
            "codex filters entry set to exclude",
            "codex",
            POLICY_FILTERS,
            |v| format!("shell_environment_policy:\n  filters:\n    \"{v}\": exclude\n"),
        ),
        ("codex policy under a profile", "codex", POLICY_SET, |v| {
            format!(
                "profiles:\n  work:\n    shell_environment_policy:\n      set:\n        {v}: x\n"
            )
        }),
    ]
}

#[test]
fn every_form_of_every_protected_variable_is_refused_naming_the_variable() {
    for (label, target, path, form) in forms() {
        for variable in PROTECTED_VARIABLES {
            let text = form(variable);
            let found = if target == "claude" {
                claude(&text)
            } else {
                codex(&text)
            };
            assert!(
                found.contains(&names(path, variable)),
                "{label}, {variable}: {found:?}"
            );
        }
    }
}

#[test]
fn a_lower_case_spelling_is_named_by_the_fixed_name_and_never_printed() {
    let found = codex("shell_environment_policy:\n  set:\n    codex_home: x\n");
    assert_eq!(found, [names(POLICY_SET, "CODEX_HOME")]);
    assert!(found.iter().all(|line| !line.contains("codex_home")));
}

#[test]
fn a_name_that_is_not_a_plain_name_is_refused_without_printing_it() {
    let bad = "CODEX_HOME=/x";
    let message = |path: &str| {
        format!(
            "error [protected-variable] personal:settings.yaml: {path} has a key that is not a plain variable name (letters, digits and '_', not starting with a digit) (\u{a7}7.9)"
        )
    };
    assert_eq!(
        claude(&format!("env:\n  \"{bad}\": x\n")),
        [message("claude.env")]
    );
    assert_eq!(
        codex(&format!(
            "shell_environment_policy:\n  set:\n    \"{bad}\": x\n"
        )),
        [message(POLICY_SET)]
    );
    assert_eq!(
        claude("env:\n  \"CODEX_HOME X\": x\n"),
        [message("claude.env")]
    );
}

#[test]
fn a_list_or_map_of_the_wrong_type_cannot_be_checked_and_is_refused() {
    let unchecked = |path: &str, why: &str| {
        format!(
            "error [protected-variable] personal:settings.yaml: {path} {why}, so it cannot be checked (\u{a7}7.9)"
        )
    };
    assert_eq!(
        claude("env: [HOME]\n"),
        [unchecked("claude.env", "is not a mapping")]
    );
    assert_eq!(
        claude("env: HOME\n"),
        [unchecked("claude.env", "is not a mapping")]
    );
    assert_eq!(
        codex("shell_environment_policy:\n  set: [HOME]\n"),
        [unchecked(POLICY_SET, "is not a mapping")]
    );
    assert_eq!(
        codex("shell_environment_policy:\n  exclude: HOME\n"),
        [unchecked(POLICY_EXCLUDE, "is not a list")]
    );
    assert_eq!(
        codex("shell_environment_policy:\n  exclude: [5]\n"),
        [unchecked(
            POLICY_EXCLUDE,
            "holds a pattern that is not a string"
        )]
    );
    assert_eq!(
        codex("shell_environment_policy:\n  filters: [HOME]\n"),
        [unchecked(POLICY_FILTERS, "is not a mapping")]
    );
}

#[test]
fn a_filters_action_that_is_not_exactly_include_does_not_exempt_the_pattern() {
    assert_eq!(
        codex("shell_environment_policy:\n  filters:\n    HOME: exlude\n"),
        [names(POLICY_FILTERS, "HOME")]
    );
    assert_eq!(
        codex("shell_environment_policy:\n  filters:\n    HOME: Include\n"),
        [names(POLICY_FILTERS, "HOME")]
    );
}

#[test]
fn what_the_policy_allows_is_not_refused() {
    for yaml in [
        "shell_environment_policy:\n  exclude: ['*FOO*']\n",
        "shell_environment_policy:\n  include_only: [HOME, PATH]\n",
        "shell_environment_policy:\n  inherit: core\n",
        "shell_environment_policy:\n  filters:\n    HOME: include\n",
        "shell_environment_policy:\n  set:\n    FOO: x\n",
        "shell_environment_policy:\n  set:\n  exclude:\n  filters:\n",
        "model: x\n",
    ] {
        assert_eq!(codex(yaml), Vec::<String>::new(), "{yaml}");
    }
    assert_eq!(claude("env:\n  FOO: x\n"), Vec::<String>::new());
    assert_eq!(claude("model: x\n"), Vec::<String>::new());
}

#[test]
fn a_policy_in_a_list_is_found_too() {
    assert_eq!(
        codex("items:\n  - shell_environment_policy:\n      set:\n        HOME: x\n"),
        [names(POLICY_SET, "HOME")]
    );
}

fn servers(yaml: &str) -> Vec<String> {
    let entries = tree(yaml);
    lines(&server_envs(
        entries.get("mcpServers"),
        ServerField::Claude,
        Code::Agent,
        &at(),
    ))
}

fn codex_servers(yaml: &str) -> Vec<String> {
    let entries = tree(yaml);
    lines(&server_envs(
        entries.get("mcp_servers"),
        ServerField::Codex,
        Code::Agent,
        &at(),
    ))
}

#[test]
fn a_policy_that_is_not_a_mapping_is_an_error_of_the_file_kind_naming_the_key() {
    for value in ["x", "[HOME]", "true", "5", "1.5"] {
        let yaml = format!(
            "shell_environment_policy: {value}
"
        );
        assert_eq!(
            codex(&yaml),
            [
                "error [settings] personal:settings.yaml: codex.shell_environment_policy must be a mapping"
            ],
            "{value}"
        );
        let found = lines(&codex_policies(&tree(&yaml), Code::Agent, &at()));
        assert_eq!(
            found,
            [
                "error [agent] personal:settings.yaml: codex.shell_environment_policy must be a mapping"
            ],
            "{value}"
        );
    }
    assert_eq!(
        codex(
            "profiles:
  work:
    shell_environment_policy: HOME
"
        ),
        [
            "error [settings] personal:settings.yaml: codex.shell_environment_policy must be a mapping"
        ]
    );
    assert_eq!(
        codex(
            "shell_environment_policy:
"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn an_mcp_servers_value_of_another_type_is_an_error_naming_the_key() {
    for value in ["x", "true", "5"] {
        assert_eq!(
            servers(&format!(
                "mcpServers: {value}
"
            )),
            ["error [agent] personal:settings.yaml: claude.mcpServers must be a mapping or a list"],
            "{value}"
        );
        assert_eq!(
            codex_servers(&format!(
                "mcp_servers: {value}
"
            )),
            ["error [agent] personal:settings.yaml: codex.mcp_servers must be a mapping"],
            "{value}"
        );
    }
    assert_eq!(
        codex_servers(
            "mcp_servers: [a]
"
        ),
        ["error [agent] personal:settings.yaml: codex.mcp_servers must be a mapping"]
    );
    assert_eq!(
        servers(
            "mcpServers: [a]
"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        servers(
            "mcpServers:
"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn the_env_of_a_server_is_checked_in_the_map_shape_and_the_list_shape() {
    let path = paths::CLAUDE_SERVER_ENV;
    assert_eq!(
        servers("mcpServers:\n  s:\n    env:\n      HOME: x\n"),
        [names(path, "HOME")]
    );
    assert_eq!(
        servers("mcpServers:\n  - plain-name\n  - s:\n      env:\n        xdg_state_home: x\n"),
        [names(path, "XDG_STATE_HOME")]
    );
    assert_eq!(
        servers("mcpServers:\n  s:\n    env:\n      FOO: x\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        servers("mcpServers:\n  s:\n    env: [HOME]\n"),
        [format!(
            "error [protected-variable] personal:settings.yaml: {path} is not a mapping, so it cannot be checked (\u{a7}7.9)"
        )]
    );
}

proptest! {
    #[test]
    fn a_protected_name_is_refused_in_any_ascii_case_and_the_text_is_never_printed(
        index in 0..PROTECTED_VARIABLES.len(),
        flips in proptest::collection::vec(proptest::bool::ANY, 16),
        form in 0_usize..3,
    ) {
        let fixed = PROTECTED_VARIABLES.get(index).copied().unwrap_or("HOME");
        let varied: String = fixed
            .chars()
            .zip(flips.iter().cycle())
            .map(|(c, flip)| if *flip { c.to_ascii_lowercase() } else { c })
            .collect();
        let (found, path) = match form {
            0 => (claude(&format!("env:\n  {varied}: x\n")), "claude.env"),
            1 => (
                codex(&format!("shell_environment_policy:\n  set:\n    {varied}: x\n")),
                POLICY_SET,
            ),
            _ => (
                codex(&format!("profiles:\n  p:\n    shell_environment_policy:\n      exclude: [{varied}]\n")),
                POLICY_EXCLUDE,
            ),
        };
        prop_assert!(found.contains(&names(path, fixed)), "{found:?}");
        if varied != fixed {
            prop_assert!(found.iter().all(|line| !line.contains(&varied)));
        }
        prop_assert_eq!(found.iter().filter(|line| line.contains(&format!("names {fixed};"))).count() >= 1, true);
    }
}
