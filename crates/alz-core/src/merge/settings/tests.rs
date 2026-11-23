use crate::item::SettingsTree;
use crate::merge::Tree;
use crate::merge::provenance::Action;
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
        .map(|part| (part.layer().as_str(), part.action()))
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
