use crate::diagnostic::Where;
use crate::item::{Binding, DefaultMode, PermissionMode, Permissions};
use crate::layer::LayerName;
use crate::merge::algebra::{ListEdit, OrderedSet};
use crate::merge::provenance::Traced;
use crate::merge::testing::{layer, layer_from_yaml, lines, merged};
use crate::stack::ResolvedLayer;
use crate::untrusted_text::Stripped;

use super::{PermissionRule, PermissionState};

// The files of acceptance case 07 (§13.7): a personal layer with a deny list, and a repo layer
// that removes one of its rules or adds to every list.
const CASE_07_PERSONAL_YAML: &str =
    include_str!("../../../../../test/fixtures/07-permission-floor/personal/layer.yaml");
const CASE_07_PERSONAL_PERMISSIONS: &str =
    include_str!("../../../../../test/fixtures/07-permission-floor/personal/permissions.yaml");
const CASE_07_REMOVE_YAML: &str =
    include_str!("../../../../../test/fixtures/07-permission-floor/repo-remove/.layer/layer.yaml");
const CASE_07_REMOVE_PERMISSIONS: &str = include_str!(
    "../../../../../test/fixtures/07-permission-floor/repo-remove/.layer/permissions.yaml"
);
const CASE_07_COMBINE_YAML: &str =
    include_str!("../../../../../test/fixtures/07-permission-floor/repo-combine/.layer/layer.yaml");
const CASE_07_COMBINE_PERMISSIONS: &str = include_str!(
    "../../../../../test/fixtures/07-permission-floor/repo-combine/.layer/permissions.yaml"
);

fn case_07(repo_yaml: &str, repo_permissions: &str) -> Vec<ResolvedLayer> {
    vec![
        layer_from_yaml(
            CASE_07_PERSONAL_YAML,
            &[("permissions.yaml", CASE_07_PERSONAL_PERMISSIONS)],
        ),
        layer_from_yaml(repo_yaml, &[("permissions.yaml", repo_permissions)]),
    ]
}

/// A layer called `name` whose only file is `permissions.yaml`.
pub(super) fn permissions_layer(name: &str, yaml: &str) -> ResolvedLayer {
    layer(name, "client", &[("permissions.yaml", yaml)])
}

/// The rules of a merged list as written.
pub(super) fn written(rules: &[Traced<PermissionRule>]) -> Vec<String> {
    rules.iter().map(|rule| rule.value().to_string()).collect()
}

/// The layers that made each rule, most general first.
fn makers(rules: &[Traced<PermissionRule>]) -> Vec<Vec<&str>> {
    rules
        .iter()
        .map(|rule| {
            rule.provenance()
                .contributions()
                .iter()
                .map(|part| part.layer().as_str())
                .collect()
        })
        .collect()
}

fn stack(files: &[(&str, &str)]) -> Vec<ResolvedLayer> {
    files
        .iter()
        .map(|(name, yaml)| permissions_layer(name, yaml))
        .collect()
}

/// The diagnostics of merging the stack, and the merged permissions when there is no error.
fn run(files: &[(&str, &str)]) -> (Vec<String>, Option<super::MergedPermissions>) {
    let found = merged(stack(files));
    (
        lines(&found),
        found.merged().map(|model| model.permissions().clone()),
    )
}

// Test 07, case 1.

#[test]
fn case_07_a_repo_layer_removing_a_personal_deny_fails_naming_the_layer_and_the_rule() {
    let found = merged(case_07(CASE_07_REMOVE_YAML, CASE_07_REMOVE_PERMISSIONS));
    assert_eq!(
        lines(&found),
        [
            "error [permission-floor] shop-api:permissions.yaml: deny rule 'read:.env*' is set by layer personal and cannot be removed by layer shop-api"
        ]
    );
    assert!(found.merged().is_none());
}

// Test 07, case 2.

#[test]
fn case_07_allow_ask_and_deny_lists_combine_across_layers() {
    let found = merged(case_07(CASE_07_COMBINE_YAML, CASE_07_COMBINE_PERMISSIONS));
    assert_eq!(lines(&found), Vec::<String>::new());
    let permissions = found.merged().expect("no error was found").permissions();
    assert_eq!(
        written(permissions.allow()),
        ["bash:git status", "bash:mise run *"]
    );
    assert_eq!(
        written(permissions.deny()),
        ["read:.env*", "bash:rm -rf *", "bash:git push --force*"]
    );
    assert_eq!(written(permissions.ask()), Vec::<String>::new());
    assert_eq!(
        makers(permissions.deny()),
        [vec!["personal"], vec!["personal"], vec!["shop-api"]]
    );
}

// The floor of deny rules.

#[test]
fn a_replacement_that_drops_a_deny_rule_is_refused_and_the_rule_stays() {
    let layers = stack(&[
        ("personal", "deny: [\"read:.env*\", \"bash:rm -rf *\"]"),
        ("repo", "deny: \"[bash:rm -rf *, bash:ls]\""),
    ]);
    let found = merged(layers);
    assert_eq!(
        lines(&found),
        [
            "error [permission-floor] repo:permissions.yaml: deny rule 'read:.env*' is set by layer personal and cannot be removed by layer repo"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn every_removed_deny_rule_is_reported() {
    let (found, merged) = run(&[
        ("personal", "deny: [\"read:.env*\", \"bash:rm -rf *\"]"),
        ("repo", "deny: \"-[bash:rm -rf *, read:.env*]\""),
    ]);
    assert_eq!(
        found,
        [
            "error [permission-floor] repo:permissions.yaml: deny rule 'read:.env*' is set by layer personal and cannot be removed by layer repo",
            "error [permission-floor] repo:permissions.yaml: deny rule 'bash:rm -rf *' is set by layer personal and cannot be removed by layer repo",
        ]
    );
    assert!(merged.is_none());
}

#[test]
fn the_layer_named_is_the_one_that_set_the_rule_not_the_first_layer() {
    let (found, _) = run(&[
        ("personal", "deny: [\"read:.env*\"]"),
        ("client", "deny: [\"bash:rm -rf *\"]"),
        ("repo", "deny: \"-[bash:rm -rf *]\""),
    ]);
    assert_eq!(
        found,
        [
            "error [permission-floor] repo:permissions.yaml: deny rule 'bash:rm -rf *' is set by layer client and cannot be removed by layer repo"
        ]
    );
}

#[test]
fn a_deny_rule_a_layer_adds_and_removes_itself_is_no_removal() {
    let (found, merged) = run(&[
        ("personal", "deny: [\"read:.env*\"]"),
        ("repo", "deny: \"+[bash:ls] -[bash:ls]\""),
    ]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(written(merged.expect("merged").deny()), ["read:.env*"]);
}

#[test]
fn a_deny_rule_removed_and_added_back_in_one_edit_stays() {
    let (found, merged) = run(&[
        ("personal", "deny: [\"read:.env*\"]"),
        ("repo", "deny: \"-[read:.env*] +[read:.env*]\""),
    ]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(written(merged.expect("merged").deny()), ["read:.env*"]);
}

#[test]
fn allow_and_ask_rules_may_be_removed_by_a_more_specific_layer() {
    let (found, merged) = run(&[
        (
            "personal",
            "allow: [\"bash:git status\", read]\nask: [\"bash:git push *\"]",
        ),
        ("repo", "allow: \"-[read]\"\nask: \"[bash:ls]\""),
    ]);
    assert_eq!(found, Vec::<String>::new());
    let merged = merged.expect("merged");
    assert_eq!(written(merged.allow()), ["bash:git status"]);
    assert_eq!(written(merged.ask()), ["bash:ls"]);
}

#[test]
fn a_rule_listed_twice_is_held_once_by_the_layer_that_set_it_first() {
    let (found, merged) = run(&[
        ("personal", "deny: [\"read:.env*\", \"read:.env*\"]"),
        ("repo", "deny: [\"read:.env*\"]"),
    ]);
    assert_eq!(found, Vec::<String>::new());
    let merged = merged.expect("merged");
    assert_eq!(written(merged.deny()), ["read:.env*"]);
    assert_eq!(makers(merged.deny()), [vec!["personal"]]);
}

#[test]
fn a_layer_without_permissions_yaml_changes_nothing() {
    let layers = vec![
        permissions_layer("personal", "deny: [read]\ndefault_mode: plan\nfloor: true"),
        layer("repo", "repo", &[]),
    ];
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    let permissions = found.merged().expect("merged").permissions();
    assert_eq!(written(permissions.deny()), ["read"]);
    assert_eq!(permissions.floor(), Some(PermissionMode::Plan));
}

#[test]
fn the_removal_leaves_the_deny_list_as_it_was() {
    let name = |text: &str| LayerName::try_from(text).expect("a layer name");
    let at = Where::layer_file(
        Stripped::from_untrusted("repo"),
        Stripped::from_untrusted("permissions.yaml"),
    );
    let rule = |text: &str| PermissionRule::try_from(text).expect("a rule");
    let add = |texts: &[&str]| ListEdit::Adjust {
        remove: OrderedSet::default(),
        add: texts.iter().copied().map(rule).collect(),
    };
    let first = Permissions::new(
        ListEdit::default(),
        ListEdit::default(),
        add(&["read:.env*"]),
        None,
    );
    let drop = Permissions::new(
        ListEdit::default(),
        ListEdit::default(),
        ListEdit::Replace(OrderedSet::default()),
        None,
    );
    let (state, _) = PermissionState::default().apply(&first, &name("personal"), &at);
    let (state, found) = state.apply(&drop, &name("repo"), &at);
    assert_eq!(found.error_count(), 1);
    assert_eq!(written(state.finish().deny()), ["read:.env*"]);
}

// default_mode and the floor.

fn mode_of(files: &[(&str, &str)]) -> (Vec<String>, Option<PermissionMode>) {
    let (found, merged) = run(files);
    (
        found,
        merged.and_then(|merged| merged.default_mode().map(|mode| *mode.value())),
    )
}

#[test]
fn the_most_specific_default_mode_wins() {
    assert_eq!(
        mode_of(&[
            ("personal", "default_mode: plan"),
            ("repo", "default_mode: accept_edits"),
        ]),
        (Vec::new(), Some(PermissionMode::AcceptEdits))
    );
}

#[test]
fn a_layer_without_a_default_mode_keeps_the_inherited_one() {
    assert_eq!(
        mode_of(&[
            ("personal", "default_mode: plan"),
            ("repo", "allow: [read]")
        ]),
        (Vec::new(), Some(PermissionMode::Plan))
    );
}

#[test]
fn a_mode_looser_than_a_floor_is_refused_naming_both_layers() {
    let (found, mode) = mode_of(&[
        ("personal", "default_mode: dont_ask\nfloor: true"),
        ("repo", "default_mode: accept_edits"),
    ]);
    assert_eq!(
        found,
        [
            "error [permission-floor] repo:permissions.yaml: default_mode 'accept_edits' is looser than the floor 'dont_ask' set by layer personal and cannot be set by layer repo"
        ]
    );
    assert_eq!(mode, None);
}

#[test]
fn a_mode_as_strict_as_the_floor_or_stricter_is_kept() {
    for (mode, expected) in [
        ("dont_ask", PermissionMode::DontAsk),
        ("plan", PermissionMode::Plan),
    ] {
        assert_eq!(
            mode_of(&[
                ("personal", "default_mode: dont_ask\nfloor: true"),
                ("repo", &format!("default_mode: {mode}")),
            ]),
            (Vec::new(), Some(expected))
        );
    }
}

#[test]
fn a_mode_without_floor_binds_nobody() {
    assert_eq!(
        mode_of(&[
            ("personal", "default_mode: plan"),
            ("repo", "default_mode: bypass_permissions"),
        ]),
        (Vec::new(), Some(PermissionMode::BypassPermissions))
    );
    assert_eq!(
        mode_of(&[
            ("personal", "default_mode: plan\nfloor: false"),
            ("repo", "default_mode: auto"),
        ]),
        (Vec::new(), Some(PermissionMode::Auto))
    );
}

#[test]
fn a_floor_binds_only_more_specific_layers_never_its_own_layer() {
    // A layer that declares `floor: true` with a mode is not checked against its own floor, and a
    // more general layer's looser mode does not bind it.
    assert_eq!(
        mode_of(&[
            ("personal", "default_mode: bypass_permissions"),
            ("repo", "default_mode: default\nfloor: true"),
        ]),
        (Vec::new(), Some(PermissionMode::Default))
    );
}

#[test]
fn the_strictest_floor_binds_and_the_first_layer_to_declare_it_is_named() {
    let (found, _) = mode_of(&[
        ("personal", "default_mode: default\nfloor: true"),
        ("client", "default_mode: plan\nfloor: true"),
        ("team", "default_mode: plan\nfloor: true"),
        ("repo", "default_mode: dont_ask"),
    ]);
    assert_eq!(
        found,
        [
            "error [permission-floor] repo:permissions.yaml: default_mode 'dont_ask' is looser than the floor 'plan' set by layer client and cannot be set by layer repo"
        ]
    );
}

#[test]
fn a_looser_floor_does_not_lower_the_floor() {
    let (found, _) = mode_of(&[
        ("personal", "default_mode: plan\nfloor: true"),
        ("client", "default_mode: plan"),
        ("repo", "default_mode: default"),
    ]);
    assert_eq!(found.len(), 1);
}

#[test]
fn a_refused_mode_leaves_the_floor_and_the_mode_as_they_were() {
    let layers = stack(&[
        ("personal", "default_mode: plan\nfloor: true"),
        ("client", "default_mode: auto\nfloor: true"),
    ]);
    let found = merged(layers);
    assert_eq!(found.diagnostics().error_count(), 1);
    assert!(found.merged().is_none());
}

#[test]
fn the_floor_is_part_of_the_merged_permissions() {
    let (_, merged) = run(&[
        ("personal", "default_mode: default\nfloor: true"),
        ("repo", "default_mode: plan\nfloor: true"),
    ]);
    let merged = merged.expect("merged");
    assert_eq!(merged.floor(), Some(PermissionMode::Plan));
    let mode = merged.default_mode().expect("a mode");
    assert_eq!(mode.value(), &PermissionMode::Plan);
    assert_eq!(
        mode.provenance()
            .contributions()
            .iter()
            .map(|part| part.layer().as_str())
            .collect::<Vec<_>>(),
        ["personal", "repo"]
    );
}

// Problems of the file.

#[test]
fn a_problem_in_the_file_is_a_permissions_error_and_the_layer_is_not_merged() {
    let (found, merged) = run(&[
        ("personal", "deny: [read]"),
        (
            "repo",
            "deny: [nope]\ndefault_mode: yolo\nfloor: true\nextra: 1",
        ),
    ]);
    assert_eq!(found.len(), 3);
    assert!(
        found
            .iter()
            .all(|line| line.starts_with("error [permissions] repo:permissions.yaml:"))
    );
    assert!(merged.is_none());
}

#[test]
fn merging_twice_gives_equal_results() {
    let files = [
        ("personal", "deny: [read]\ndefault_mode: plan\nfloor: true"),
        ("repo", "deny: [bash]\nallow: [glob]"),
    ];
    assert_eq!(
        merged(stack(&files)).merged().cloned(),
        merged(stack(&files)).merged().cloned()
    );
}

#[test]
fn default_modes_are_a_pair_of_a_mode_and_a_binding() {
    let mode = DefaultMode::new(PermissionMode::Plan, Binding::Floor);
    assert_eq!(mode.mode(), PermissionMode::Plan);
    assert_eq!(mode.binding(), Binding::Floor);
}

#[test]
fn a_rule_that_is_allowed_and_denied_is_a_warning_and_deny_wins() {
    let found = merged(stack(&[
        ("personal", "deny: [\"read:.env*\", \"bash:ls\"]"),
        ("repo", "allow: [\"read:.env*\", \"bash:git status\"]"),
    ]));
    assert_eq!(
        lines(&found),
        ["warning [permission-shadowed] allow rule 'read:.env*' is also denied; deny wins"]
    );
    let permissions = found.merged().expect("a warning is no error").permissions();
    assert_eq!(written(permissions.deny()), ["read:.env*", "bash:ls"]);
    assert_eq!(
        written(permissions.allow()),
        ["read:.env*", "bash:git status"]
    );
}
