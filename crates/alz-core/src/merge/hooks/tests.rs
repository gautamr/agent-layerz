use std::collections::BTreeSet;

use proptest::collection::vec;
use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest};

use crate::item::{
    Glob, HookAction, LifecycleEvent, Lock, Matcher, OnFail, Seconds, ToolFilter, ToolMoment,
    Trigger,
};
use crate::merge::testing::{layer, layer_from_yaml, lines, merged};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;

// The files of acceptance case 08 (§13.8): a locked and an unlocked hook of the personal layer,
// disabled by one repo or the other.
const CASE_08_PERSONAL_YAML: &str =
    include_str!("../../../../../test/fixtures/08-locked-hook/personal/layer.yaml");
const CASE_08_PERSONAL_HOOKS: &str =
    include_str!("../../../../../test/fixtures/08-locked-hook/personal/hooks/hooks.yaml");
const CASE_08_BLOCK_SECRETS: &str = include_str!(
    "../../../../../test/fixtures/08-locked-hook/personal/hooks/scripts/block-secrets.sh"
);
const CASE_08_FORMAT: &str =
    include_str!("../../../../../test/fixtures/08-locked-hook/personal/hooks/scripts/format.sh");
const CASE_08_REPO_YAML: &str =
    include_str!("../../../../../test/fixtures/08-locked-hook/repo-locked/.layer/layer.yaml");
const CASE_08_LOCKED_HOOKS: &str =
    include_str!("../../../../../test/fixtures/08-locked-hook/repo-locked/.layer/hooks/hooks.yaml");
const CASE_08_UNLOCKED_HOOKS: &str = include_str!(
    "../../../../../test/fixtures/08-locked-hook/repo-unlocked/.layer/hooks/hooks.yaml"
);

// The files of acceptance case 19 (§13.19): command hooks and a native event, and a hook that has
// both run and command.
const CASE_19_PERSONAL_YAML: &str =
    include_str!("../../../../../test/fixtures/19-hook-commands/personal/layer.yaml");
const CASE_19_REPO_YAML: &str =
    include_str!("../../../../../test/fixtures/19-hook-commands/repo/.layer/layer.yaml");
const CASE_19_REPO_HOOKS: &str =
    include_str!("../../../../../test/fixtures/19-hook-commands/repo/.layer/hooks/hooks.yaml");
const CASE_19_BAD_HOOKS: &str =
    include_str!("../../../../../test/fixtures/19-hook-commands/bad/.layer/hooks/hooks.yaml");

fn case_08(repo_hooks: &str) -> Vec<ResolvedLayer> {
    vec![
        layer_from_yaml(
            CASE_08_PERSONAL_YAML,
            &[
                ("hooks/hooks.yaml", CASE_08_PERSONAL_HOOKS),
                ("hooks/scripts/block-secrets.sh", CASE_08_BLOCK_SECRETS),
                ("hooks/scripts/format.sh", CASE_08_FORMAT),
            ],
        ),
        layer_from_yaml(CASE_08_REPO_YAML, &[("hooks/hooks.yaml", repo_hooks)]),
    ]
}

fn case_19(repo_hooks: &str) -> Vec<ResolvedLayer> {
    vec![
        layer_from_yaml(CASE_19_PERSONAL_YAML, &[]),
        layer_from_yaml(CASE_19_REPO_YAML, &[("hooks/hooks.yaml", repo_hooks)]),
    ]
}

/// A layer called `name` whose only file is `hooks/hooks.yaml`.
fn hooks_layer(name: &str, yaml: &str) -> ResolvedLayer {
    layer(name, "client", &[("hooks/hooks.yaml", yaml)])
}

/// The ids of the merged hooks in order.
fn ids(layers: Vec<ResolvedLayer>) -> Vec<String> {
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    found
        .merged()
        .expect("no error was found")
        .hooks()
        .iter()
        .map(|hook| hook.value().id().as_str().to_owned())
        .collect()
}

#[test]
fn case_08_disabling_a_locked_hook_fails_naming_the_layer() {
    let found = merged(case_08(CASE_08_LOCKED_HOOKS));
    assert_eq!(
        lines(&found),
        [
            "error [hook-locked] shop-api:hooks/hooks.yaml: hook 'block-secrets' is locked by layer personal and cannot be disabled by layer shop-api"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn case_08_disabling_an_unlocked_hook_removes_it_and_keeps_the_locked_one() {
    let found = merged(case_08(CASE_08_UNLOCKED_HOOKS));
    assert_eq!(lines(&found), Vec::<String>::new());
    let model = found.merged().expect("no error was found");
    let [kept] = model.hooks() else {
        panic!("one hook is left");
    };
    let hook = kept.value();
    assert_eq!(hook.id().as_str(), "block-secrets");
    assert_eq!(hook.lock(), Lock::Locked);
    assert_eq!(hook.on_fail(), OnFail::Deny);
    let HookAction::Run(script) = hook.action() else {
        panic!("a script");
    };
    assert_eq!(script.path().as_str(), "hooks/scripts/block-secrets.sh");
    let Trigger::Tool { when, filter } = hook.trigger() else {
        panic!("a tool event");
    };
    assert_eq!(*when, ToolMoment::Before);
    let ToolFilter::Tools { tools, paths } = filter else {
        panic!("a tool filter");
    };
    assert_eq!(tools.len(), 1);
    assert_eq!(
        paths.iter().map(Glob::as_str).collect::<Vec<_>>(),
        [".env*"]
    );
}

#[test]
fn case_19_command_hooks_are_kept_as_written_with_the_project_dir_variable() {
    let found = merged(case_19(CASE_19_REPO_HOOKS));
    assert_eq!(lines(&found), Vec::<String>::new());
    let model = found.merged().expect("no error was found");
    let [guard, _] = model.hooks() else {
        panic!("two hooks");
    };
    let guard = guard.value();
    assert_eq!(guard.id().as_str(), "guard");
    let HookAction::Command(command) = guard.action() else {
        panic!("a command");
    };
    assert_eq!(
        command.as_str(),
        "\"${PROJECT_DIR}\"/.harness/scripts/guard.sh"
    );
    assert_eq!(guard.timeout().map(Seconds::get), Some(10));
    assert_eq!(guard.on_fail(), OnFail::Deny);
    assert!(matches!(
        guard.trigger(),
        Trigger::Tool { when: ToolMoment::Before, filter: ToolFilter::Tools { tools, .. } } if tools.len() == 2
    ));
}

#[test]
fn case_19_a_native_event_keeps_its_matcher_and_runs_on_its_target_only() {
    let found = merged(case_19(CASE_19_REPO_HOOKS));
    let model = found.merged().expect("no error was found");
    let notify = model.hooks().last().expect("a second hook").value();
    assert_eq!(notify.id().as_str(), "notify");
    let Trigger::Native { event, matcher } = notify.trigger() else {
        panic!("a native event");
    };
    assert_eq!(event.target(), TargetName::Claude);
    assert_eq!(event.name().as_str(), "Notification");
    assert_eq!(
        matcher.as_ref().map(Matcher::as_str),
        Some("permission_prompt")
    );
    assert_eq!(
        notify.targets().map(|targets| targets.as_slice().to_vec()),
        Some(vec![TargetName::Claude])
    );
    assert_eq!(notify.on_fail(), OnFail::Warn);
    let HookAction::Command(command) = notify.action() else {
        panic!("a command");
    };
    assert_eq!(
        command.as_str(),
        "\"${PROJECT_DIR}\"/.harness/scripts/notify.sh"
    );
}

#[test]
fn case_19_a_hook_with_both_run_and_command_is_refused() {
    let found = merged(case_19(CASE_19_BAD_HOOKS));
    assert_eq!(
        lines(&found),
        [
            "error [hooks] shop-api:hooks/hooks.yaml:1:3: hook 'both': needs exactly one of 'run' (a script in the layer) or 'command'"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn hooks_run_in_stack_order_then_file_order() {
    let general = hooks_layer(
        "general",
        "- id: c\n  event: stop\n  command: x\n- id: a\n  event: stop\n  command: x\n",
    );
    let specific = hooks_layer(
        "specific",
        "- id: d\n  event: stop\n  command: x\n- id: b\n  event: stop\n  command: x\n",
    );
    assert_eq!(ids(vec![general, specific]), ["c", "a", "d", "b"]);
}

#[test]
fn a_hook_remembers_the_layer_that_defined_it() {
    let found = merged(vec![hooks_layer(
        "general",
        "- id: a\n  event: stop\n  command: x\n",
    )]);
    let model = found.merged().expect("no error was found");
    let [hook] = model.hooks() else {
        panic!("one hook");
    };
    assert_eq!(hook.provenance().contributions().len(), 1);
    assert_eq!(
        hook.provenance()
            .contributions()
            .first()
            .map(|part| part.layer().as_str()),
        Some("general")
    );
    assert!(matches!(
        hook.value().trigger(),
        Trigger::Lifecycle(LifecycleEvent::Stop)
    ));
}

#[test]
fn an_id_that_a_more_general_layer_uses_is_an_error_even_when_that_hook_is_unlocked() {
    let hook = "- id: a\n  event: stop\n  command: x\n";
    let found = merged(vec![
        hooks_layer("general", hook),
        hooks_layer("specific", hook),
    ]);
    assert_eq!(
        lines(&found),
        [
            "error [hooks] specific:hooks/hooks.yaml: hook 'a' is already defined by layer general and cannot be defined again"
        ]
    );
}

#[test]
fn an_id_twice_in_one_layer_is_an_error() {
    let hook = "- id: a\n  event: stop\n  command: x\n";
    let found = merged(vec![hooks_layer("general", &format!("{hook}{hook}"))]);
    assert_eq!(
        lines(&found),
        [
            "error [hooks] general:hooks/hooks.yaml: hook 'a' is already defined by layer general and cannot be defined again"
        ]
    );
}

#[test]
fn a_disabled_id_stays_taken() {
    let found = merged(vec![
        hooks_layer("general", "- id: a\n  event: stop\n  command: x\n"),
        hooks_layer("middle", "- id: a\n  disabled: true\n"),
        hooks_layer("specific", "- id: a\n  event: stop\n  command: y\n"),
    ]);
    assert_eq!(
        lines(&found),
        [
            "error [hooks] specific:hooks/hooks.yaml: hook 'a' was disabled by layer middle and cannot be defined again"
        ]
    );
}

#[test]
fn disabling_an_unknown_hook_is_unresolved() {
    let found = merged(vec![
        hooks_layer("general", "- id: a\n  event: stop\n  command: x\n"),
        hooks_layer("specific", "- id: nope\n  disabled: true\n"),
    ]);
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] specific:hooks/hooks.yaml: cannot disable hook 'nope': no more general layer defines it"
        ]
    );
}

#[test]
fn a_layer_cannot_disable_its_own_hook() {
    let found = merged(vec![hooks_layer(
        "general",
        "- id: a\n  event: stop\n  command: x\n- id: a\n  disabled: true\n",
    )]);
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] general:hooks/hooks.yaml: cannot disable hook 'a': no more general layer defines it"
        ]
    );
}

#[test]
fn a_hook_disabled_once_cannot_be_disabled_again() {
    let found = merged(vec![
        hooks_layer("general", "- id: a\n  event: stop\n  command: x\n"),
        hooks_layer("middle", "- id: a\n  disabled: true\n"),
        hooks_layer("specific", "- id: a\n  disabled: true\n"),
    ]);
    assert_eq!(found.diagnostics().iter().count(), 1);
    assert!(found.merged().is_none());
}

#[test]
fn a_locked_hook_cannot_be_disabled_by_any_later_layer() {
    let found = merged(vec![
        hooks_layer(
            "general",
            "- id: a\n  event: stop\n  command: x\n  locked: true\n",
        ),
        layer("middle", "client", &[]),
        hooks_layer("specific", "- id: a\n  disabled: true\n"),
    ]);
    assert_eq!(
        lines(&found),
        [
            "error [hook-locked] specific:hooks/hooks.yaml: hook 'a' is locked by layer general and cannot be disabled by layer specific"
        ]
    );
}

#[test]
fn a_disable_entry_holds_only_the_id_and_disabled_true() {
    for entry in [
        "- id: a\n  disabled: true\n  command: x\n",
        "- id: a\n  disabled: false\n",
        "- id: a\n  disabled: yes please\n",
    ] {
        let found = merged(vec![
            hooks_layer("general", "- id: a\n  event: stop\n  command: x\n"),
            hooks_layer("specific", entry),
        ]);
        assert_eq!(
            lines(&found),
            [
                "error [hooks] specific:hooks/hooks.yaml:1:3: hook 'a': a disable entry has only 'id' and 'disabled: true'"
            ],
            "{entry}"
        );
    }
}

#[test]
fn every_problem_of_every_layer_is_reported_in_one_run() {
    let found = merged(vec![
        hooks_layer(
            "general",
            "- id: a\n  event: nowhen\n  command: x\n- id: b\n  event: stop\n",
        ),
        hooks_layer("specific", "- id: gone\n  disabled: true\n"),
    ]);
    assert_eq!(found.diagnostics().iter().count(), 3);
}

/// One entry of a generated `hooks.yaml`.
#[derive(Debug, Clone)]
enum Entry {
    Define { id: usize, locked: bool },
    Disable { id: usize },
}

fn entries() -> impl Strategy<Value = Vec<Entry>> {
    let entry = prop_oneof![
        (0..3_usize, any::<bool>()).prop_map(|(id, locked)| Entry::Define { id, locked }),
        (0..3_usize).prop_map(|id| Entry::Disable { id }),
    ];
    vec(entry, 0..4)
}

fn yaml(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| match entry {
            Entry::Define { id, locked } => {
                format!("- id: h{id}\n  event: stop\n  command: x\n  locked: {locked}\n")
            }
            Entry::Disable { id } => format!("- id: h{id}\n  disabled: true\n"),
        })
        .collect()
}

fn stack(layers: &[Vec<Entry>]) -> Vec<ResolvedLayer> {
    layers
        .iter()
        .enumerate()
        .map(|(index, entries)| hooks_layer(&format!("layer-{index}"), &yaml(entries)))
        .collect()
}

/// The hooks the spec says stand after `layers` as `(id, locked)` in order, and whether any entry
/// is an error: the rules of §7.5 written the plain way, with no knowledge of the merge's code.
fn model(layers: &[Vec<Entry>]) -> (Vec<(usize, bool)>, bool) {
    let (standing, _, failed) = layers.iter().enumerate().fold(
        (Vec::<(usize, bool, usize)>::new(), BTreeSet::new(), false),
        |state, (origin, entries)| {
            entries
                .iter()
                .fold(state, |(mut standing, mut disabled, mut failed), entry| {
                    match *entry {
                        Entry::Define { id, locked } => {
                            if standing.iter().any(|held| held.0 == id) || disabled.contains(&id) {
                                failed = true;
                            } else {
                                standing.push((id, locked, origin));
                            }
                        }
                        Entry::Disable { id } => {
                            match standing
                                .iter()
                                .position(|held| held.0 == id && held.2 != origin)
                            {
                                Some(at) if !standing[at].1 => {
                                    standing.remove(at);
                                    disabled.insert(id);
                                }
                                Some(_) | None => failed = true,
                            }
                        }
                    }
                    (standing, disabled, failed)
                })
        },
    );
    (
        standing
            .into_iter()
            .map(|(id, locked, _)| (id, locked))
            .collect(),
        failed,
    )
}

fn merged_hooks(layers: &[Vec<Entry>]) -> (Option<Vec<(String, bool)>>, usize) {
    let found = merged(stack(layers));
    let errors = found.diagnostics().iter().count();
    (
        found.merged().map(|model| {
            model
                .hooks()
                .iter()
                .map(|hook| {
                    (
                        hook.value().id().as_str().to_owned(),
                        hook.value().lock() == Lock::Locked,
                    )
                })
                .collect()
        }),
        errors,
    )
}

proptest! {
    #[test]
    fn the_merge_follows_the_rules_of_the_spec_for_any_stack(
        layers in vec(entries(), 1..5),
    ) {
        let (standing, failed) = model(&layers);
        let (hooks, errors) = merged_hooks(&layers);
        prop_assert_eq!(errors > 0, failed);
        if !failed {
            prop_assert_eq!(
                hooks,
                Some(
                    standing
                        .into_iter()
                        .map(|(id, locked)| (format!("h{id}"), locked))
                        .collect()
                )
            );
        }
    }

    #[test]
    fn no_later_layer_can_disable_a_locked_hook(
        layers in vec(entries(), 1..4),
        further in entries(),
    ) {
        let (before, errors_before) = merged_hooks(&layers);
        let extended: Vec<Vec<Entry>> = layers.iter().cloned().chain([further]).collect();
        let (after, errors_after) = merged_hooks(&extended);
        // A stack that has an error still has it with one more layer.
        if errors_before > 0 {
            prop_assert!(errors_after >= errors_before);
        }
        // Otherwise the merge refuses, or every locked hook is still there, still locked.
        if let (Some(before), Some(after)) = (before, after) {
            for locked in before.iter().filter(|(_, locked)| *locked) {
                prop_assert!(after.contains(locked), "{locked:?} is gone from {after:?}");
            }
            // The hooks that are left keep their order.
            let kept = |from: &[(String, bool)], other: &[(String, bool)]| -> Vec<String> {
                from.iter()
                    .filter(|(id, _)| other.iter().any(|(same, _)| same == id))
                    .map(|(id, _)| id.clone())
                    .collect()
            };
            prop_assert_eq!(kept(&before, &after), kept(&after, &before));
        }
    }

    #[test]
    fn a_layer_that_disables_nothing_keeps_every_hook_in_its_place(
        layers in vec(entries(), 1..4),
        id in 3..6_usize,
    ) {
        let (before, errors) = merged_hooks(&layers);
        let added = vec![Entry::Define { id, locked: false }];
        let extended: Vec<Vec<Entry>> = layers.iter().cloned().chain([added]).collect();
        let (after, _) = merged_hooks(&extended);
        if let (Some(before), true) = (before, errors == 0) {
            let expected: Vec<(String, bool)> = before
                .into_iter()
                .chain([(format!("h{id}"), false)])
                .collect();
            prop_assert_eq!(after, Some(expected));
        }
    }
}
