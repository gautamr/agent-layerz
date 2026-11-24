use crate::item::{
    HookAction, HookEntry, HookItems, LifecycleEvent, Lock, Matcher, OnFail, Seconds, ToolFilter,
    ToolMoment, Trigger,
};
use crate::merge::testing::layer;
use crate::target::TargetName;

/// What the layer `repo` with this `hooks.yaml` and the script `hooks/scripts/ok.sh` gives.
fn read(yaml: &str) -> (Vec<HookEntry>, Vec<String>) {
    let layer = layer(
        "repo",
        "repo",
        &[
            ("hooks/hooks.yaml", yaml),
            ("hooks/scripts/ok.sh", "#!/bin/sh\n"),
            ("hooks/scripts/dir/deep.sh", "#!/bin/sh\n"),
            ("hooks/other.sh", "#!/bin/sh\n"),
        ],
    );
    let (items, found) = HookItems::read(&layer);
    (
        items
            .into_file()
            .map_or_else(Vec::new, |(_, entries)| entries),
        found.iter().map(ToString::to_string).collect(),
    )
}

fn only_hook(yaml: &str) -> crate::item::Hook {
    let (entries, found) = read(yaml);
    assert_eq!(found, Vec::<String>::new());
    let [HookEntry::Define(hook)] = entries.as_slice() else {
        panic!("one defined hook, got {entries:?}");
    };
    hook.clone()
}

fn refused(yaml: &str) -> Vec<String> {
    let (entries, found) = read(yaml);
    assert_eq!(entries, Vec::new(), "{yaml}");
    found
}

#[test]
fn a_layer_without_hooks_yaml_has_no_hooks() {
    let (items, found) = HookItems::read(&layer("repo", "repo", &[("hooks/scripts/ok.sh", "x")]));
    assert!(items.into_file().is_none());
    assert_eq!(found.iter().count(), 0);
}

#[test]
fn an_empty_hooks_yaml_has_no_hooks() {
    for text in ["", "# nothing\n", "[]\n", "~\n"] {
        let (entries, found) = read(text);
        assert_eq!((entries, found), (Vec::new(), Vec::new()), "{text:?}");
    }
}

#[test]
fn hooks_yaml_must_be_a_list() {
    assert_eq!(
        refused("id: a\n"),
        ["error [hooks] repo:hooks/hooks.yaml: hooks.yaml must be a list"]
    );
}

#[test]
fn a_yaml_error_is_reported_with_its_yaml_code() {
    let found = refused("- id: a\n  id: b\n");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].starts_with("error [yaml] repo:hooks/hooks.yaml:"),
        "{found:?}"
    );
}

#[test]
fn a_script_hook_with_a_tool_filter_is_read_whole() {
    let hook = only_hook(
        "- id: block-secrets\n  event: pre_tool\n  match: { tools: [read, mcp__db__query, read], paths: [\".env*\"] }\n  run: scripts/ok.sh\n  on_fail: deny\n  targets: [claude, codex, claude]\n  timeout: 30\n  locked: true\n",
    );
    assert_eq!(hook.id().as_str(), "block-secrets");
    let Trigger::Tool { when, filter } = hook.trigger() else {
        panic!("a tool event");
    };
    assert_eq!(*when, ToolMoment::Before);
    let ToolFilter::Tools { tools, paths } = filter else {
        panic!("a filter");
    };
    assert_eq!((tools.len(), paths.len()), (2, 1));
    let HookAction::Run(script) = hook.action() else {
        panic!("a script");
    };
    assert_eq!(script.path().as_str(), "hooks/scripts/ok.sh");
    assert_eq!(hook.on_fail(), OnFail::Deny);
    assert_eq!(hook.lock(), Lock::Locked);
    assert_eq!(hook.timeout().map(Seconds::get), Some(30));
    assert_eq!(
        hook.targets().map(|targets| targets.as_slice().to_vec()),
        Some(vec![TargetName::Claude, TargetName::Codex])
    );
}

#[test]
fn a_hook_with_only_the_required_fields_warns_on_every_target_and_is_open() {
    let hook = only_hook("- id: a\n  event: post_tool\n  command: echo done\n");
    assert_eq!(hook.on_fail(), OnFail::Warn);
    assert_eq!(hook.lock(), Lock::Open);
    assert_eq!(hook.timeout(), None);
    assert_eq!(hook.targets(), None);
    assert!(matches!(
        hook.trigger(),
        Trigger::Tool {
            when: ToolMoment::After,
            filter: ToolFilter::Any
        }
    ));
}

#[test]
fn every_neutral_event_is_read() {
    for (event, expected) in [
        ("session_start", LifecycleEvent::SessionStart),
        ("stop", LifecycleEvent::Stop),
        ("subagent_stop", LifecycleEvent::SubagentStop),
        ("prompt_submit", LifecycleEvent::PromptSubmit),
    ] {
        let hook = only_hook(&format!("- id: a\n  event: {event}\n  command: x\n"));
        assert_eq!(hook.trigger(), &Trigger::Lifecycle(expected), "{event}");
    }
}

#[test]
fn run_names_a_script_below_hooks_scripts_relative_to_hooks() {
    let hook = only_hook("- id: a\n  event: stop\n  run: scripts/dir/deep.sh\n");
    assert!(matches!(
        hook.action(),
        HookAction::Run(script) if script.path().as_str() == "hooks/scripts/dir/deep.sh"
    ));
    for run in [
        "other.sh",
        "ok.sh",
        "../x.sh",
        "scripts/../other.sh",
        "/etc/passwd",
        "scripts/",
        "",
        "scripts//ok.sh",
    ] {
        let found = refused(&format!("- id: a\n  event: stop\n  run: '{run}'\n"));
        assert_eq!(
            found,
            [
                "error [hooks] repo:hooks/hooks.yaml:1:3: hook 'a': 'run' must name a script below hooks/scripts/, written relative to hooks/"
            ],
            "{run:?}"
        );
    }
}

#[test]
fn a_script_the_layer_does_not_hold_is_an_error_naming_it() {
    assert_eq!(
        refused("- id: a\n  event: stop\n  run: scripts/missing.sh\n"),
        [
            "error [hooks] repo:hooks/hooks.yaml:1:3: hook 'a': script 'hooks/scripts/missing.sh' is not in the layer"
        ]
    );
}

#[test]
fn a_hook_has_exactly_one_of_run_and_command() {
    for fields in ["", "  run: scripts/ok.sh\n  command: x\n"] {
        assert_eq!(
            refused(&format!("- id: a\n  event: stop\n{fields}")),
            [
                "error [hooks] repo:hooks/hooks.yaml:1:3: hook 'a': needs exactly one of 'run' (a script in the layer) or 'command'"
            ],
            "{fields:?}"
        );
    }
}

#[test]
fn a_native_event_takes_a_matcher_and_runs_on_its_target() {
    let hook = only_hook(
        "- id: a\n  event: 'codex:PermissionRequest'\n  matcher: Bash\n  command: x\n  targets: [codex]\n",
    );
    let Trigger::Native { event, matcher } = hook.trigger() else {
        panic!("a native event");
    };
    assert_eq!(
        (event.target(), event.name().as_str()),
        (TargetName::Codex, "PermissionRequest")
    );
    assert_eq!(matcher.as_ref().map(Matcher::as_str), Some("Bash"));
    let bare = only_hook("- id: a\n  event: 'claude:Stop'\n  command: x\n");
    assert!(matches!(
        bare.trigger(),
        Trigger::Native { matcher: None, .. }
    ));
}

#[test]
fn the_fields_of_a_native_event_and_a_neutral_one_do_not_mix() {
    let at = "error [hooks] repo:hooks/hooks.yaml:1:3: hook 'a': ";
    for (fields, message) in [
        (
            "event: 'claude:Notification'\n  match: { tools: [read] }\n  command: x\n",
            "a native event takes the target's own 'matcher', not 'match'",
        ),
        (
            "event: 'claude:Notification'\n  command: x\n  targets: [codex]\n",
            "an event of claude only runs on that target",
        ),
        (
            "event: stop\n  matcher: x\n  command: x\n",
            "'matcher' is only for native events; use 'match'",
        ),
        (
            "event: stop\n  match: { tools: [read] }\n  command: x\n",
            "'match' only applies to the tool events pre_tool and post_tool",
        ),
        (
            "event: pre_tool\n  match: { paths: ['*.md'] }\n  command: x\n",
            "'match.paths' needs 'match.tools'",
        ),
        (
            "event: stop\n  command: x\n  targets: []\n",
            "'targets' must list at least one target",
        ),
    ] {
        assert_eq!(
            refused(&format!("- id: a\n  {fields}")),
            [format!("{at}{message}")],
            "{fields}"
        );
    }
}

#[test]
fn a_field_that_breaks_its_rule_is_reported_at_its_key_path() {
    for (fields, expected) in [
        (
            "event: later\n  command: x\n",
            "[0].event must be session_start, pre_tool, post_tool, stop, subagent_stop or prompt_submit, or <target>:<Event> for claude or codex, such as claude:Notification",
        ),
        (
            "event: 'gemini:Stop'\n  command: x\n",
            "[0].event must be session_start",
        ),
        (
            "event: 'claude:stop'\n  command: x\n",
            "[0].event must be session_start",
        ),
        (
            "event: stop\n  command: '  '\n",
            "[0].command must not be empty",
        ),
        (
            "event: stop\n  command: x\n  on_fail: explode\n",
            "[0].on_fail must be deny, warn or feedback",
        ),
        (
            "event: stop\n  command: x\n  timeout: 0\n",
            "[0].timeout must be a positive whole number of seconds",
        ),
        (
            "event: stop\n  command: x\n  timeout: 1.5\n",
            "[0].timeout must be a whole number",
        ),
        (
            "event: stop\n  command: x\n  timeout: -3\n",
            "[0].timeout must be a positive whole number of seconds",
        ),
        (
            "event: stop\n  command: x\n  locked: maybe\n",
            "[0].locked must be true or false",
        ),
        (
            "event: pre_tool\n  match: { tools: ['read:x'] }\n  command: x\n",
            "[0].match.tools[0] must name a tool, without a scope",
        ),
        (
            "event: pre_tool\n  match: { tools: [read], when: x }\n  command: x\n",
            "unknown field 'when' in [0].match",
        ),
        (
            "event: stop\n  command: x\n  speed: 3\n",
            "unknown field 'speed' in [0]",
        ),
        ("command: x\n", "[0].event is required"),
        (
            "event: stop\n  command: x\n  targets: [vim]\n",
            "[0].targets[0] must be",
        ),
    ] {
        let found = refused(&format!("- id: a\n  {fields}"));
        assert_eq!(found.len(), 1, "{fields}: {found:?}");
        assert!(
            found[0].starts_with("error [hooks] repo:hooks/hooks.yaml:")
                && found[0].contains(expected),
            "{fields}: {found:?}"
        );
    }
}

#[test]
fn an_entry_without_a_usable_id_is_refused() {
    for (yaml, expected) in [
        ("- event: stop\n", "[0].id is required"),
        ("- id: ' '\n", "[0].id must not be empty"),
        ("- id: 3\n", "[0].id must be text"),
        ("- nope\n", "[0] must be a mapping"),
    ] {
        let found = refused(yaml);
        assert_eq!(found.len(), 1, "{yaml}");
        assert!(found[0].contains(expected), "{yaml}: {found:?}");
    }
}

#[test]
fn a_good_entry_is_kept_beside_a_bad_one_and_both_are_reported_in_file_order() {
    let (entries, found) = read(
        "- id: a\n  event: stop\n  command: x\n- id: b\n  event: nowhen\n  command: x\n- id: c\n  disabled: true\n",
    );
    assert_eq!(entries.len(), 2);
    assert!(matches!(&entries[0], HookEntry::Define(hook) if hook.id().as_str() == "a"));
    assert!(matches!(&entries[1], HookEntry::Disable(id) if id.as_str() == "c"));
    assert_eq!(found.len(), 1);
}

#[test]
fn an_id_is_printed_quoted_with_its_quote_doubled() {
    let found = refused("- id: \"it's\"\n  event: stop\n");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].contains("hook 'it''s': needs exactly one"),
        "{found:?}"
    );
}

#[test]
fn a_script_name_with_a_character_a_shell_would_read_is_a_hooks_error_naming_the_file() {
    for run in [
        "scripts/a b.sh",
        "scripts/a\\\"b.sh",
        "scripts/a$b.sh",
        "scripts/a`b`.sh",
        "scripts/a;b.sh",
        "scripts/a'b.sh",
        "scripts/a&b.sh",
        "scripts/é.sh",
    ] {
        let found = refused(&format!("- id: a\n  event: stop\n  run: \"{run}\"\n"));
        assert_eq!(found.len(), 1, "{run:?}: {found:?}");
        assert!(
            found[0].starts_with("error [hooks] repo:hooks/hooks.yaml:1:3: hook 'a': script '"),
            "{run:?}: {found:?}"
        );
        assert!(
            found[0].ends_with("' may only hold ASCII letters, digits, '.', '_', '-' and '/'"),
            "{run:?}: {found:?}"
        );
    }
}
