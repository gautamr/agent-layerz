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

fn tree_of(target: TargetName, yaml: &str) -> Tree {
    crate::item::SettingsItems::read(&settings_layer("t", yaml))
        .0
        .into_file()
        .and_then(|(_, file)| file.of(target).map(SettingsTree::to_tree))
        .expect("settings of the target")
}

fn tree(yaml: &str) -> Tree {
    tree_of(TargetName::Claude, yaml)
}

#[test]
fn mappings_merge_key_by_key_and_the_more_specific_scalar_wins() {
    let merged = of(
        &[
            (
                "personal",
                "claude:\n  language: a\n  env:\n    X: '1'\n    Y: '2'\n  effortLevel: low\n",
            ),
            (
                "repo",
                "claude:\n  env:\n    Y: '3'\n    Z: '4'\n  effortLevel: high\n  verbose: true\n",
            ),
        ],
        TargetName::Claude,
    );
    assert_eq!(
        merged,
        Some(tree(
            "claude:\n  language: a\n  env:\n    X: '1'\n    Y: '3'\n    Z: '4'\n  effortLevel: high\n  verbose: true\n"
        ))
    );
}

#[test]
fn two_lists_are_concatenated_without_repeats_and_mappings_in_another_key_order_are_one_item() {
    let merged = of(
        &[
            (
                "personal",
                "claude:\n  availableModels: [a, b]\n  spinnerTipsOverride:\n    tips:\n      - {id: a, text: t}\n",
            ),
            (
                "repo",
                "claude:\n  availableModels: [b, c, a]\n  spinnerTipsOverride:\n    tips:\n      - {text: t, id: a}\n      - {id: b, text: u}\n",
            ),
        ],
        TargetName::Claude,
    );
    assert_eq!(
        merged,
        Some(tree(
            "claude:\n  availableModels: [a, b, c]\n  spinnerTipsOverride:\n    tips:\n      - {id: a, text: t}\n      - {id: b, text: u}\n"
        ))
    );
}

#[test]
fn a_change_of_kind_is_replaced_by_the_more_specific_value() {
    // Codex settings have no types here, so a change of kind is a plain replacement; a Claude
    // value of another kind is refused when the file is read.
    let merged = of(
        &[
            ("personal", "codex:\n  a: [x]\n  b: 1\n  c: {k: 1}\n"),
            ("repo", "codex:\n  a: 2\n  b: [y]\n  c: [z]\n"),
        ],
        TargetName::Codex,
    );
    assert_eq!(
        merged,
        Some(tree_of(
            TargetName::Codex,
            "codex:\n  a: 2\n  b: [y]\n  c: [z]\n"
        ))
    );
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
        ("repo", "claude:\n  verbose: true\n"),
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
        ("personal", "claude:\n  verbose: true\n"),
        ("client", "codex:\n  a: 1\n"),
        ("repo", "claude:\n  fastMode: true\n"),
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
            ("personal", "claude:\n  verbose: true\n"),
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
        ("personal", "claude:\n  verbose: true\n"),
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
            "claude:\n  env:\n    FOO: x\n    __proto__: y\n    constructor: z\n    toString: w\ncodex:\n  shell_environment_policy:\n    inherit: core\n    include_only: [HOME, PATH]\n    exclude: ['*FOO*']\n",
        )],
        TargetName::Claude,
    );
    assert!(merged.is_some());
}

#[test]
fn merging_the_same_stack_twice_gives_equal_results() {
    let files = [
        ("personal", "claude:\n  availableModels: [a]\n"),
        ("repo", "claude:\n  availableModels: [b]\n"),
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
                ("personal", "codex:\n  allow_login_shell: false\n"),
                ("repo", &format!("codex:\n  allow_login_shell: {other}\n")),
            ]),
            [floor_error("repo", "codex.allow_login_shell", KIND)],
            "{other}"
        );
    }
}

#[test]
fn a_claude_floor_key_of_another_kind_is_refused_by_its_type_before_the_floor_is_looked_at() {
    assert_eq!(
        error_lines(&[
            ("personal", "claude:\n  sandbox:\n    enabled: true\n"),
            ("repo", "claude:\n  sandbox:\n    enabled: text\n"),
        ]),
        ["error [settings] repo:settings.yaml:3:14: 'claude.sandbox.enabled' must be a boolean"]
    );
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
        [
            "error [settings] repo:settings.yaml:4:17: 'claude.sandbox.filesystem.denyRead' must be a list whose items are text"
        ]
    );
}

#[test]
fn a_file_that_loosens_a_floor_is_not_merged_at_all_and_every_loosening_is_reported() {
    let found = merged(stack(&[
        (
            "personal",
            "claude:\n  sandbox:\n    enabled: true\n  verbose: true\n",
        ),
        (
            "repo",
            "claude:\n  sandbox:\n    enabled: false\n    failIfUnavailable: true\n  disableSkillShellExecution: true\n  fastMode: true\ncodex:\n  allow_login_shell: true\n",
        ),
    ]));
    assert_eq!(
        lines(&found),
        [floor_error("repo", "claude.sandbox.enabled", LOOSER)]
    );
    assert!(found.merged().is_none());
}

/// §7.9 When a floor is broken: one file that loosens two floors, one of each target, gets both
/// reported (in key order of the target), and none of its settings are merged.
#[test]
fn every_loosening_of_a_file_is_reported_not_only_the_first() {
    let found = merged(stack(&[
        (
            "personal",
            "claude:\n  sandbox:\n    enabled: true\n  disableSkillShellExecution: true\ncodex:\n  allow_login_shell: false\n",
        ),
        (
            "repo",
            "claude:\n  sandbox:\n    enabled: false\n  disableSkillShellExecution: false\ncodex:\n  allow_login_shell: true\n",
        ),
    ]));
    assert_eq!(
        lines(&found),
        [
            floor_error("repo", "claude.disableSkillShellExecution", LOOSER),
            floor_error("repo", "claude.sandbox.enabled", LOOSER),
            floor_error("repo", "codex.allow_login_shell", LOOSER),
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn the_first_layer_to_set_a_floor_key_is_free_to_set_any_value() {
    assert_eq!(
        error_lines(&[
            ("personal", "claude:\n  verbose: true\n"),
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
