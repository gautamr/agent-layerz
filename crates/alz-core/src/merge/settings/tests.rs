use crate::item::SettingsTree;
use crate::merge::Tree;
use crate::merge::action::Action;
use crate::merge::testing::{layer, lines, merged};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;

fn settings_layer(name: &str, yaml: &str) -> ResolvedLayer {
    layer(name, "client", &[("settings.yaml", yaml)])
}

fn stack(files: &[(&str, &str)]) -> Vec<ResolvedLayer> {
    files
        .iter()
        .map(|(name, yaml)| settings_layer(name, yaml))
        .collect()
}

/// The merged settings of `target` as a tree, after merging the stack without an error.
fn of(files: &[(&str, &str)], target: TargetName) -> Option<Tree> {
    let found = merged(stack(files));
    assert_eq!(lines(&found), Vec::<String>::new());
    found
        .merged()
        .and_then(|model| model.settings().get(&target))
        .map(|settings| settings.value().to_tree())
}

fn tree(yaml: &str) -> Tree {
    crate::item::SettingsItems::read(&settings_layer("t", yaml))
        .0
        .into_file()
        .and_then(|(_, file)| file.of(TargetName::Claude).map(SettingsTree::to_tree))
        .expect("claude settings")
}

#[test]
fn mappings_merge_key_by_key_and_the_more_specific_scalar_wins() {
    let merged = of(
        &[
            (
                "personal",
                "claude:\n  keep: 1\n  both:\n    x: 1\n    y: 2\n  effort: low\n",
            ),
            (
                "repo",
                "claude:\n  both:\n    y: 3\n    z: 4\n  effort: high\n  new: true\n",
            ),
        ],
        TargetName::Claude,
    );
    assert_eq!(
        merged,
        Some(tree(
            "claude:\n  keep: 1\n  both:\n    x: 1\n    y: 3\n    z: 4\n  effort: high\n  new: true\n"
        ))
    );
}

#[test]
fn two_lists_are_concatenated_without_repeats_and_mappings_in_another_key_order_are_one_item() {
    let merged = of(
        &[
            (
                "personal",
                "claude:\n  allowed: [a, b]\n  items:\n    - {k: 1, j: 2}\n",
            ),
            (
                "repo",
                "claude:\n  allowed: [b, c, a]\n  items:\n    - {j: 2, k: 1}\n    - {k: 3}\n",
            ),
        ],
        TargetName::Claude,
    );
    assert_eq!(
        merged,
        Some(tree(
            "claude:\n  allowed: [a, b, c]\n  items:\n    - {k: 1, j: 2}\n    - {k: 3}\n"
        ))
    );
}

#[test]
fn a_change_of_kind_is_replaced_by_the_more_specific_value() {
    let merged = of(
        &[
            ("personal", "claude:\n  a: [x]\n  b: 1\n  c: {k: 1}\n"),
            ("repo", "claude:\n  a: 2\n  b: [y]\n  c: [z]\n"),
        ],
        TargetName::Claude,
    );
    assert_eq!(merged, Some(tree("claude:\n  a: 2\n  b: [y]\n  c: [z]\n")));
}

#[test]
fn the_merged_settings_are_an_ordered_map_of_the_targets_that_some_layer_set() {
    let found = merged(stack(&[
        ("personal", "codex:\n  a: 1\n"),
        ("repo", "codex:\n  b: 2\n"),
    ]));
    let model = found.merged().expect("no error was found");
    let targets: Vec<TargetName> = model.settings().keys().copied().collect();
    assert_eq!(targets, [TargetName::Codex]);
    let found = merged(stack(&[
        ("personal", "codex:\n  a: 1\n"),
        ("repo", "claude:\n  b: 2\n"),
    ]));
    let targets: Vec<TargetName> = found
        .merged()
        .expect("no error was found")
        .settings()
        .keys()
        .copied()
        .collect();
    assert_eq!(targets, [TargetName::Claude, TargetName::Codex]);
}

#[test]
fn a_target_keeps_the_layers_that_set_it() {
    let found = merged(stack(&[
        ("personal", "claude:\n  a: 1\n"),
        ("client", "codex:\n  a: 1\n"),
        ("repo", "claude:\n  b: 2\n"),
    ]));
    let model = found.merged().expect("no error was found");
    let claude = model.settings().get(&TargetName::Claude).expect("claude");
    let parts: Vec<(&str, Action)> = claude
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action().clone()))
        .collect();
    assert_eq!(
        parts,
        [("personal", Action::Define), ("repo", Action::Extend)]
    );
    let codex = model.settings().get(&TargetName::Codex).expect("codex");
    assert_eq!(codex.provenance().contributions().len(), 1);
}

#[test]
fn a_layer_with_a_refused_top_level_key_is_reported_and_nothing_is_merged() {
    for key in ["__proto__", "constructor", "toString", "Claude", "cursor"] {
        let found = merged(stack(&[
            ("personal", "claude:\n  a: 1\n"),
            ("repo", &format!("{key}:\n  claude:\n    permissions: x\n")),
        ]));
        assert_eq!(
            lines(&found),
            [format!(
                "error [settings] repo:settings.yaml:1:1: unknown field '{key}'"
            )],
            "{key}"
        );
        assert!(found.merged().is_none(), "{key}");
    }
}

#[test]
fn a_reserved_key_is_refused_naming_the_layer_and_the_file_that_sets_it() {
    let found = merged(stack(&[
        ("personal", "claude:\n  a: 1\n"),
        ("repo", "claude:\n  disableAllHooks: true\n"),
    ]));
    assert_eq!(
        lines(&found),
        [
            "error [settings] repo:settings.yaml:2:3: 'claude.disableAllHooks' is set through hooks.yaml `disabled` (locked hooks cannot be switched off)"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn a_protected_variable_is_refused_naming_the_layer_and_the_file() {
    let found = merged(stack(&[(
        "personal",
        "claude:\n  env:\n    HOME: x\ncodex:\n  shell_environment_policy:\n    exclude: ['*ome*']\n",
    )]));
    let found_lines = lines(&found);
    assert_eq!(found_lines.len(), 6, "{found_lines:?}");
    assert!(
        found_lines
            .iter()
            .all(|line| line.starts_with("error [protected-variable] personal:settings.yaml: "))
    );
    assert!(found.merged().is_none());
}

#[test]
fn settings_that_set_other_variables_and_names_that_are_plain_keys_merge() {
    let merged = of(
        &[(
            "personal",
            "claude:\n  env:\n    FOO: x\n  __proto__: y\n  constructor: z\n  toString: w\ncodex:\n  shell_environment_policy:\n    inherit: core\n    include_only: [HOME, PATH]\n    exclude: ['*FOO*']\n",
        )],
        TargetName::Claude,
    );
    assert!(merged.is_some());
}

#[test]
fn merging_the_same_stack_twice_gives_equal_results() {
    let files = [
        ("personal", "claude:\n  a: [1]\n"),
        ("repo", "claude:\n  a: [2]\n"),
    ];
    assert_eq!(merged(stack(&files)), merged(stack(&files)));
}

// The floors of the table of native keys (§7.9).

fn error_lines(files: &[(&str, &str)]) -> Vec<String> {
    lines(&merged(stack(files)))
}

fn floor_error(layer: &str, path: &str, sentence: &str) -> String {
    format!("error [settings] {layer}:settings.yaml: '{path}' is a floor: {sentence}")
}

const LOOSER: &str = "this value is looser than the one a more general layer set";
const ADDED: &str = "it may not gain entries that a more general layer did not set";
const KIND: &str = "its kind differs from the one a more general layer set";

#[test]
fn a_boolean_floor_set_to_the_stricter_value_cannot_be_loosened_by_a_more_specific_layer() {
    assert_eq!(
        error_lines(&[
            ("personal", "claude:\n  sandbox:\n    enabled: true\n"),
            ("repo", "claude:\n  sandbox:\n    enabled: false\n"),
        ]),
        [floor_error("repo", "claude.sandbox.enabled", LOOSER)]
    );
    assert_eq!(
        error_lines(&[
            ("personal", "codex:\n  allow_login_shell: false\n"),
            ("repo", "codex:\n  allow_login_shell: true\n"),
        ]),
        [floor_error("repo", "codex.allow_login_shell", LOOSER)]
    );
}

#[test]
fn a_boolean_floor_may_be_raised_and_repeated() {
    assert_eq!(
        of(
            &[
                ("personal", "claude:\n  sandbox:\n    enabled: false\n"),
                ("client", "claude:\n  sandbox:\n    enabled: true\n"),
                ("repo", "claude:\n  sandbox:\n    enabled: true\n"),
            ],
            TargetName::Claude
        ),
        Some(tree("claude:\n  sandbox:\n    enabled: true\n"))
    );
}

#[test]
fn a_floor_key_that_changes_kind_is_refused_even_to_null() {
    for other in ["null", "[a]", "text", "1"] {
        assert_eq!(
            error_lines(&[
                ("personal", "claude:\n  sandbox:\n    enabled: true\n"),
                (
                    "repo",
                    &format!("claude:\n  sandbox:\n    enabled: {other}\n")
                ),
            ]),
            [floor_error("repo", "claude.sandbox.enabled", KIND)],
            "{other}"
        );
    }
}

#[test]
fn an_ordered_floor_may_only_move_up() {
    assert_eq!(
        error_lines(&[
            ("personal", "codex:\n  web_search: cached\n"),
            ("repo", "codex:\n  web_search: live\n"),
        ]),
        [floor_error("repo", "codex.web_search", LOOSER)]
    );
    assert_eq!(
        of(
            &[
                ("personal", "codex:\n  web_search: cached\n"),
                ("repo", "codex:\n  web_search: disabled\n"),
            ],
            TargetName::Codex
        ),
        Some(Tree::Map(
            [("web_search".to_owned(), Tree::Text("disabled".to_owned()))].into()
        ))
    );
}

#[test]
fn a_profile_key_is_named_by_its_pattern_never_by_the_profile() {
    assert_eq!(
        error_lines(&[
            (
                "personal",
                "codex:\n  profiles:\n    secret-name:\n      web_search: disabled\n"
            ),
            (
                "repo",
                "codex:\n  profiles:\n    secret-name:\n      web_search: live\n"
            ),
        ]),
        [floor_error("repo", "codex.profiles.*.web_search", LOOSER)]
    );
}

#[test]
fn a_no_additions_list_may_not_gain_an_entry_but_may_repeat_entries() {
    assert_eq!(
        error_lines(&[
            (
                "personal",
                "claude:\n  sandbox:\n    network:\n      allowedDomains: [a.example]\n"
            ),
            (
                "repo",
                "claude:\n  sandbox:\n    network:\n      allowedDomains: [a.example, b.example]\n"
            ),
        ]),
        [floor_error(
            "repo",
            "claude.sandbox.network.allowedDomains",
            ADDED
        )]
    );
    assert_eq!(
        of(
            &[
                (
                    "personal",
                    "claude:\n  sandbox:\n    network:\n      allowedDomains: [a.example, b.example]\n"
                ),
                (
                    "repo",
                    "claude:\n  sandbox:\n    network:\n      allowedDomains: [b.example]\n"
                ),
            ],
            TargetName::Claude
        ),
        Some(tree(
            "claude:\n  sandbox:\n    network:\n      allowedDomains: [a.example, b.example]\n"
        ))
    );
}

#[test]
fn a_union_list_grows_and_cannot_change_kind() {
    assert_eq!(
        of(
            &[
                (
                    "personal",
                    "claude:\n  sandbox:\n    filesystem:\n      denyRead: [a]\n"
                ),
                (
                    "repo",
                    "claude:\n  sandbox:\n    filesystem:\n      denyRead: [b]\n"
                ),
            ],
            TargetName::Claude
        ),
        Some(tree(
            "claude:\n  sandbox:\n    filesystem:\n      denyRead: [a, b]\n"
        ))
    );
    assert_eq!(
        error_lines(&[
            (
                "personal",
                "claude:\n  sandbox:\n    filesystem:\n      denyRead: [a]\n"
            ),
            (
                "repo",
                "claude:\n  sandbox:\n    filesystem:\n      denyRead: none\n"
            ),
        ]),
        [floor_error(
            "repo",
            "claude.sandbox.filesystem.denyRead",
            KIND
        )]
    );
}

#[test]
fn a_file_that_loosens_a_floor_is_not_merged_at_all_and_every_loosening_is_reported() {
    let found = merged(stack(&[
        (
            "personal",
            "claude:\n  sandbox:\n    enabled: true\n  a: 1\n",
        ),
        (
            "repo",
            "claude:\n  sandbox:\n    enabled: false\n    failIfUnavailable: true\n  disableSkillShellExecution: true\n  b: 2\ncodex:\n  allow_login_shell: true\n",
        ),
    ]));
    assert_eq!(
        lines(&found),
        [floor_error("repo", "claude.sandbox.enabled", LOOSER)]
    );
    assert!(found.merged().is_none());
}

#[test]
fn the_first_layer_to_set_a_floor_key_is_free_to_set_any_value() {
    assert_eq!(
        error_lines(&[
            ("personal", "claude:\n  a: 1\n"),
            ("repo", "claude:\n  sandbox:\n    enabled: false\n"),
        ]),
        Vec::<String>::new()
    );
}

#[test]
fn a_floor_binds_a_target_only_against_the_same_target() {
    assert_eq!(
        error_lines(&[
            ("personal", "claude:\n  sandbox:\n    enabled: true\n"),
            (
                "repo",
                "codex:\n  sandbox_workspace_write:\n    network_access: true\n"
            ),
        ]),
        Vec::<String>::new()
    );
}
