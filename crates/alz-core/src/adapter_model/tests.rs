//! The conversion of a merged stack into the model of the adapter contract (spec §8.0 **Model.**):
//! one test for each kind of item of §7 that the model holds, showing that each neutral field
//! reaches the model, and one for each of the four kinds of security item with the layers that
//! set it.

use alz_adapter::{
    AgentTool, DenyRule, FloorField, HookEvent, HookMatch, HookRun, Isolation, JsonValue,
    McpTransport, Memory, Mode, Model, ModelProblem, NeutralEvent, OnFail, SecurityItemId,
    TargetId,
};

use super::model_for;
use crate::layer::LayerName;
use crate::merge::testing::{layer, layer_executable, layer_from_yaml, merged_with};

const ONLY: &str = "name: only\nkind: repo\nvisibility: private\n";

/// The model for `target` of the stack of `layers`, merged under a manifest with `options`.
fn model_of(layers: Vec<crate::stack::ResolvedLayer>, options: &str, target: &str) -> Model {
    let found = merged_with(layers, options);
    assert!(!found.diagnostics().has_errors(), "{}", found.diagnostics());
    model_for(
        found.merged().expect("no error was found"),
        &TargetId::try_from(target).expect("a target id"),
    )
}

/// The model for `claude` of one layer named `only`.
fn one_layer(files: &[(&str, &str)]) -> Model {
    model_of(
        vec![layer_from_yaml(ONLY, files)],
        "options:\n  strict_slots: false\n",
        "claude",
    )
}

fn names<'a>(layers: impl Iterator<Item = &'a alz_adapter::LayerName>) -> Vec<&'a str> {
    layers.map(alz_adapter::LayerName::as_str).collect()
}

fn layer_set(items: &[&str]) -> Vec<String> {
    items.iter().map(|text| (*text).to_owned()).collect()
}

// 7.2 Agents

#[test]
fn an_agent_reaches_the_model_with_each_neutral_field_and_the_native_fields_of_its_target() {
    let model = one_layer(&[
        (
            "agents/a.md",
            "---\nname: a\ndescription: Plans\nrole: planning\ntools: [read, \"write:docs/**\", mcp__fs__read]\nskills: [s1, s2]\nisolation: worktree\npermission_mode: plan\nmemory: project\nclaude:\n  maxTurns: 9\n  color: blue\ncodex:\n  sandbox: x\n---\nBody\n",
        ),
        (
            "models.yaml",
            "aliases:\n  strong: { claude: opus, codex: other }\nroles:\n  planning: strong\n",
        ),
    ]);
    let [agent] = model.agents.as_slice() else {
        panic!("one agent: {:?}", model.agents);
    };
    assert_eq!(agent.name, "a");
    assert_eq!(agent.description, "Plans");
    assert_eq!(
        agent.tools,
        Some(vec![
            AgentTool {
                name: "read".to_owned(),
                scope: None
            },
            AgentTool {
                name: "write".to_owned(),
                scope: Some("docs/**".to_owned())
            },
            AgentTool {
                name: "mcp__fs__read".to_owned(),
                scope: None
            },
        ])
    );
    assert_eq!(agent.model.as_deref(), Some("opus"));
    assert_eq!(agent.model_problem, None);
    assert_eq!(agent.permission_mode, Some(Mode::Plan));
    assert_eq!(agent.skills, layer_set(&["s1", "s2"]));
    assert_eq!(agent.isolation, Isolation::Worktree);
    assert_eq!(agent.memory, Memory::Project);
    assert_eq!(
        agent.native,
        vec![
            ("maxTurns".to_owned(), JsonValue::Int(9)),
            ("color".to_owned(), JsonValue::Text("blue".to_owned())),
        ]
    );
    assert_eq!(agent.body, "Body");
    let [source] = agent.sources.as_slice() else {
        panic!("one source: {:?}", agent.sources);
    };
    assert_eq!(
        (source.layer.as_str(), source.path.as_str()),
        ("only", "agents/a.md")
    );
}

#[test]
fn an_agent_for_another_target_is_not_in_the_model_and_the_native_fields_are_those_of_the_target() {
    let files = [
        (
            "agents/a.md",
            "---\nname: a\ndescription: d\ntargets: [codex]\n---\nBody\n",
        ),
        (
            "agents/b.md",
            "---\nname: b\ndescription: d\nclaude:\n  color: blue\ncodex:\n  sandbox_mode: read-only\n---\nBody\n",
        ),
    ];
    let claude = model_of(
        vec![layer_from_yaml(ONLY, &files)],
        "targets: [claude]\n",
        "claude",
    );
    assert_eq!(
        claude
            .agents
            .iter()
            .map(|agent| agent.name.as_str())
            .collect::<Vec<_>>(),
        ["b"]
    );
    let codex = model_of(
        vec![layer_from_yaml(ONLY, &files)],
        "targets: [claude]\n",
        "codex",
    );
    assert_eq!(
        codex
            .agents
            .iter()
            .map(|agent| agent.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(
        codex.agents[1]
            .native
            .iter()
            .all(|(key, _)| key == "sandbox_mode")
    );
}

#[test]
fn an_agent_that_asks_for_an_unknown_alias_or_an_alias_without_a_model_has_a_model_problem() {
    let model = one_layer(&[
        (
            "agents/unknown.md",
            "---\nname: unknown\ndescription: d\nmodel: nope\n---\nB\n",
        ),
        (
            "agents/codex-only.md",
            "---\nname: codex-only\ndescription: d\nmodel: other\n---\nB\n",
        ),
        (
            "agents/inherits.md",
            "---\nname: inherits\ndescription: d\nmodel: inherit\n---\nB\n",
        ),
        ("models.yaml", "aliases:\n  other: { codex: gpt-x }\n"),
    ]);
    let problem = |name: &str| {
        model
            .agents
            .iter()
            .find(|agent| agent.name == name)
            .map(|agent| (agent.model.clone(), agent.model_problem.clone()))
    };
    assert_eq!(
        problem("unknown"),
        Some((None, Some(ModelProblem::NotAnAlias("nope".to_owned()))))
    );
    assert_eq!(
        problem("codex-only"),
        Some((
            None,
            Some(ModelProblem::NoModelForTarget("other".to_owned()))
        ))
    );
    assert_eq!(problem("inherits"), Some((None, None)));
}

// 7.3 Skills, 7.4 Commands, slot stubs

#[test]
fn a_skill_a_command_and_a_slot_stub_reach_the_model() {
    let model = model_of(
        vec![layer_executable(
            ONLY,
            &[
                (
                    "skills/s/SKILL.md",
                    "---\nname: s\ndescription: S\n---\nS body\n",
                ),
                ("skills/s/scripts/run.sh", "#!/bin/sh\n"),
                (
                    "commands/go.md",
                    "---\ndescription: Go\nargument_hint: <x>\n---\nDo it\n",
                ),
                (
                    "agents/a.md",
                    "---\nname: a\ndescription: d\nskills: [ghost]\n---\nB\n",
                ),
            ],
            None,
            &["skills/s/scripts/run.sh"],
        )],
        "options:\n  strict_slots: false\n",
        "claude",
    );
    let [skill] = model.skills.as_slice() else {
        panic!("one skill: {:?}", model.skills);
    };
    assert_eq!(skill.name, "s");
    assert_eq!(
        skill
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.executable))
            .collect::<Vec<_>>(),
        [("SKILL.md", false), ("scripts/run.sh", true)]
    );
    assert_eq!(skill.files[1].content, b"#!/bin/sh\n");
    assert_eq!(skill.sources[0].path, "skills/s");
    let [command] = model.commands.as_slice() else {
        panic!("one command: {:?}", model.commands);
    };
    assert_eq!(command.name, "go");
    assert_eq!(command.description, "Go");
    assert_eq!(command.argument_hint.as_deref(), Some("<x>"));
    assert_eq!(command.body, "Do it");
    assert_eq!(command.sources[0].path, "commands/go.md");
    assert_eq!(
        model
            .slot_stubs
            .iter()
            .map(|stub| stub.name.as_str())
            .collect::<Vec<_>>(),
        ["ghost"]
    );
}

#[test]
fn the_sources_of_a_skill_are_the_definition_that_stands() {
    let model = model_of(
        vec![
            layer(
                "first",
                "personal",
                &[(
                    "skills/s/SKILL.md",
                    "---\nname: s\ndescription: one\n---\nA\n",
                )],
            ),
            layer(
                "second",
                "repo",
                &[(
                    "skills/s/SKILL.md",
                    "---\nname: s\ndescription: two\n---\nB\n",
                )],
            ),
        ],
        "",
        "claude",
    );
    let sources: Vec<&str> = model.skills[0]
        .sources
        .iter()
        .map(|source| source.layer.as_str())
        .collect();
    assert_eq!(sources, ["second"]);
}

// 7.10 Rules

#[test]
fn a_rule_reaches_the_model_with_its_globs_its_extensions_and_its_sources() {
    let model = model_of(
        vec![
            layer(
                "first",
                "personal",
                &[(
                    "rules/testing.md",
                    "---\npaths: [\"**/*.test.ts\"]\n---\nONE\n",
                )],
            ),
            layer(
                "second",
                "repo",
                &[(
                    "rules/testing.md",
                    "---\nmode: extend\npaths: [\"e2e/**\"]\n---\nTWO\n",
                )],
            ),
        ],
        "",
        "claude",
    );
    let [rule] = model.rules.as_slice() else {
        panic!("one rule: {:?}", model.rules);
    };
    assert_eq!(rule.name, "testing");
    assert_eq!(rule.paths, layer_set(&["**/*.test.ts", "e2e/**"]));
    assert!(rule.body.starts_with("ONE"));
    assert!(rule.body.contains("TWO"));
    assert_eq!(
        rule.sources
            .iter()
            .map(|source| source.layer.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
}

// 7.5 Hooks

#[test]
fn hooks_reach_the_model_with_their_events_matchers_scripts_and_settings() {
    let model = model_of(
        vec![layer_executable(
            ONLY,
            &[
                (
                    "hooks/hooks.yaml",
                    "- id: guard\n  event: pre_tool\n  match: { tools: [read, mcp__fs__write], paths: [\".env*\"] }\n  run: scripts/guard.sh\n  timeout: 5\n  on_fail: deny\n  locked: true\n- id: say\n  event: stop\n  command: echo ${PROJECT_DIR}\n- id: note\n  event: claude:Notification\n  matcher: idle_prompt\n  command: ./notify.sh\n  on_fail: feedback\n- id: elsewhere\n  event: stop\n  command: ./x.sh\n  targets: [codex]\n- id: theirs\n  event: codex:Other\n  command: ./y.sh\n",
                ),
                ("hooks/scripts/guard.sh", "#!/bin/sh\nexit 0\n"),
            ],
            None,
            &["hooks/scripts/guard.sh"],
        )],
        "",
        "claude",
    );
    let ids: Vec<&str> = model.hooks.iter().map(|hook| hook.id.as_str()).collect();
    assert_eq!(ids, ["guard", "say", "note"]);
    let guard = &model.hooks[0];
    assert_eq!(guard.event, HookEvent::Neutral(NeutralEvent::PreTool));
    assert_eq!(
        guard.matcher,
        HookMatch::Neutral {
            tools: layer_set(&["read", "mcp__fs__write"]),
            paths: layer_set(&[".env*"]),
        }
    );
    let HookRun::Script(script) = &guard.run else {
        panic!("a script: {:?}", guard.run);
    };
    assert_eq!(script.path, "scripts/guard.sh");
    assert_eq!(script.content, b"#!/bin/sh\nexit 0\n");
    assert!(script.executable);
    assert_eq!(guard.timeout.map(std::num::NonZeroU32::get), Some(5));
    assert_eq!(guard.on_fail, OnFail::Deny);
    assert!(guard.locked);
    assert_eq!(guard.defined_by.as_str(), "only");
    assert_eq!(guard.sources[0].path, "hooks/hooks.yaml");
    let say = &model.hooks[1];
    assert_eq!(say.event, HookEvent::Neutral(NeutralEvent::Stop));
    assert_eq!(say.matcher, HookMatch::Any);
    assert_eq!(say.run, HookRun::Command("echo ${PROJECT_DIR}".to_owned()));
    assert_eq!(say.on_fail, OnFail::Warn);
    assert!(!say.locked);
    let note = &model.hooks[2];
    assert_eq!(
        note.event,
        HookEvent::Native {
            target: TargetId::try_from("claude").expect("an id"),
            event: "Notification".to_owned()
        }
    );
    assert_eq!(note.matcher, HookMatch::Native("idle_prompt".to_owned()));
    assert_eq!(note.on_fail, OnFail::Feedback);
}

// 7.6 Permissions

#[test]
fn the_permissions_reach_the_model_with_the_default_mode_and_the_floor_fields() {
    let model = one_layer(&[(
        "permissions.yaml",
        "allow: [\"read\", \"bash:mise run *\"]\nask: [\"bash:git push *\"]\ndeny: [\"read:.env*\"]\ndefault_mode: accept_edits\ndisable_bypass_mode: true\nblock_writes_outside: true\n",
    )]);
    let permissions = &model.permissions;
    assert_eq!(permissions.allow, layer_set(&["read", "bash:mise run *"]));
    assert_eq!(permissions.ask, layer_set(&["bash:git push *"]));
    assert_eq!(permissions.deny, layer_set(&["read:.env*"]));
    assert_eq!(permissions.default_mode, Some(Mode::AcceptEdits));
    assert_eq!(
        permissions.floor_fields,
        [
            FloorField::DisableBypassMode,
            FloorField::BlockWritesOutside
        ]
    );
}

// 7.7 MCP servers

#[test]
fn only_the_enabled_mcp_servers_reach_the_model() {
    let model = model_of(
        vec![layer_from_yaml(
            ONLY,
            &[(
                "mcp.yaml",
                "zeta:\n  transport: stdio\n  command: npx\n  args: [\"-y\"]\n  env_from: [ZETA_TOKEN]\nbare:\n  transport: stdio\n  command: ./bare\nempty:\n  transport: stdio\n  command: ./empty\n  args: []\nweb:\n  transport: http\n  url: https://web.invalid/mcp\nlive:\n  transport: sse\n  url: https://live.invalid/sse\nunused:\n  transport: http\n  url: https://unused.invalid/mcp\n",
            )],
        )],
        "mcp:\n  enable: [zeta, bare, empty, web, live]\n",
        "claude",
    );
    let found: Vec<(&str, &McpTransport)> = model
        .mcp_servers
        .iter()
        .map(|server| (server.id.as_str(), &server.transport))
        .collect();
    assert_eq!(found.len(), 5, "{found:?}");
    let transport = |id: &str| {
        found
            .iter()
            .find(|(found_id, _)| *found_id == id)
            .map(|(_, transport)| (*transport).clone())
    };
    assert_eq!(
        transport("zeta"),
        Some(McpTransport::Stdio {
            command: "npx".to_owned(),
            args: Some(layer_set(&["-y"])),
            env_from: layer_set(&["ZETA_TOKEN"]),
        })
    );
    assert_eq!(
        transport("bare"),
        Some(McpTransport::Stdio {
            command: "./bare".to_owned(),
            args: None,
            env_from: Vec::new(),
        })
    );
    assert_eq!(
        transport("empty"),
        Some(McpTransport::Stdio {
            command: "./empty".to_owned(),
            args: Some(Vec::new()),
            env_from: Vec::new(),
        })
    );
    assert_eq!(
        transport("web"),
        Some(McpTransport::Http {
            url: "https://web.invalid/mcp".to_owned()
        })
    );
    assert_eq!(
        transport("live"),
        Some(McpTransport::Sse {
            url: "https://live.invalid/sse".to_owned()
        })
    );
    assert_eq!(transport("unused"), None);
}

// 7.8 Model routing

#[test]
fn the_routing_is_resolved_for_the_target_of_the_model() {
    let files = [(
        "models.yaml",
        "aliases:\n  strong: { claude: opus, codex: big }\n  fast: { claude: sonnet }\n  cheap: { codex: small }\nroles:\n  planning: strong\n  research: cheap\ndefault: strong\n",
    )];
    let claude = model_of(vec![layer_from_yaml(ONLY, &files)], "", "claude");
    assert_eq!(
        claude
            .models
            .aliases
            .iter()
            .map(|(a, m)| (a.as_str(), m.as_str()))
            .collect::<Vec<_>>(),
        [("fast", "sonnet"), ("strong", "opus")]
    );
    assert_eq!(
        claude
            .models
            .roles
            .iter()
            .map(|(r, a)| (r.as_str(), a.as_str()))
            .collect::<Vec<_>>(),
        [("planning", "strong"), ("research", "cheap")]
    );
    assert_eq!(claude.models.default.as_deref(), Some("opus"));
    let codex = model_of(vec![layer_from_yaml(ONLY, &files)], "", "codex");
    assert_eq!(codex.models.default.as_deref(), Some("big"));
    assert_eq!(codex.models.aliases.len(), 2);
}

// 7.9 Other settings

#[test]
fn the_native_settings_of_the_target_reach_the_model_and_those_of_other_targets_do_not() {
    let files = [(
        "settings.yaml",
        "claude:\n  cleanupPeriodDays: 14\n  attribution: { pr: p, commit: c }\nother-tool:\n  key: 1\n",
    )];
    let claude = model_of(vec![layer_from_yaml(ONLY, &files)], "", "claude");
    assert_eq!(
        claude.settings.get("cleanupPeriodDays"),
        Some(&JsonValue::Int(14))
    );
    assert!(matches!(
        claude.settings.get("attribution"),
        Some(JsonValue::Object(_))
    ));
    assert_eq!(claude.settings.get("key"), None);
    let none = model_of(vec![layer_from_yaml(ONLY, &[])], "", "claude");
    assert!(none.settings.is_empty());
}

// 8.0 The four kinds of security item

fn layers_of(layers: &alz_adapter::SettingLayers) -> Vec<&str> {
    names(layers.iter())
}

fn security_stack() -> Model {
    model_of(
        vec![
            layer(
                "personal",
                "personal",
                &[
                    (
                        "permissions.yaml",
                        "deny: [\"read:.env*\"]\ndefault_mode: default\nfloor: true\nblock_reads_outside: true\n",
                    ),
                    (
                        "hooks/hooks.yaml",
                        "- id: guard\n  event: stop\n  command: ./guard.sh\n  on_fail: deny\n  locked: true\n- id: free\n  event: stop\n  command: ./free.sh\n",
                    ),
                ],
            ),
            layer(
                "client",
                "client",
                &[(
                    "permissions.yaml",
                    "deny: [\"read:.env*\", \"bash:rm -rf *\"]\ndefault_mode: dont_ask\nfloor: true\nblock_reads_outside: true\nblock_writes_outside: true\n",
                )],
            ),
            layer(
                "repo",
                "repo",
                &[(
                    "permissions.yaml",
                    "deny: [\"web_fetch\"]\ndefault_mode: plan\n",
                )],
            ),
        ],
        "",
        "claude",
    )
}

#[test]
fn a_deny_rule_is_a_security_item_set_by_each_layer_that_declares_it() {
    let items = security_stack().security;
    let found: Vec<(&str, Vec<&str>)> = items
        .deny_rules
        .iter()
        .map(|item: &DenyRule| (item.rule.as_str(), layers_of(&item.set_by)))
        .collect();
    assert_eq!(
        found,
        [
            ("read:.env*", vec!["personal", "client"]),
            ("bash:rm -rf *", vec!["client"]),
            ("web_fetch", vec!["repo"]),
        ]
    );
}

#[test]
fn a_floor_field_is_a_security_item_set_by_each_layer_that_sets_it_to_true() {
    let items = security_stack().security;
    let found: Vec<(FloorField, Vec<&str>)> = items
        .floor_fields
        .iter()
        .map(|item| (item.field, layers_of(&item.set_by)))
        .collect();
    assert_eq!(
        found,
        [
            (FloorField::BlockReadsOutside, vec!["personal", "client"]),
            (FloorField::BlockWritesOutside, vec!["client"]),
        ]
    );
}

#[test]
fn the_default_mode_floor_is_the_strictest_floor_set_by_each_layer_whose_floor_is_in_force() {
    let items = security_stack().security;
    let floor = items.default_mode_floor.expect("a floor");
    assert_eq!(floor.mode, Mode::DontAsk);
    assert_eq!(layers_of(&floor.set_by), ["personal", "client"]);
    let none = model_of(
        vec![layer(
            "only",
            "repo",
            &[("permissions.yaml", "default_mode: plan\n")],
        )],
        "",
        "claude",
    );
    assert_eq!(none.security.default_mode_floor, None);
}

#[test]
fn a_locked_hook_is_a_security_item_with_its_event_its_on_fail_and_its_layer() {
    let items = security_stack().security;
    let [hook] = items.locked_hooks.as_slice() else {
        panic!("one locked hook: {:?}", items.locked_hooks);
    };
    assert_eq!(hook.id, "guard");
    assert_eq!(hook.event, HookEvent::Neutral(NeutralEvent::Stop));
    assert_eq!(hook.on_fail, OnFail::Deny);
    assert_eq!(hook.defined_by.as_str(), "personal");
}

#[test]
fn a_locked_hook_of_another_target_is_not_a_security_item_of_this_one() {
    let stack = |target: &str| {
        model_of(
            vec![layer(
                "personal",
                "personal",
                &[(
                    "hooks/hooks.yaml",
                    "- id: theirs\n  event: codex:Stop\n  command: ./x.sh\n  locked: true\n- id: ours\n  event: claude:Stop\n  command: ./y.sh\n  locked: true\n",
                )],
            )],
            "",
            target,
        )
        .security
        .ids()
        .collect::<Vec<_>>()
    };
    assert_eq!(
        stack("claude"),
        [SecurityItemId::LockedHook("ours".to_owned())]
    );
    assert_eq!(
        stack("codex"),
        [SecurityItemId::LockedHook("theirs".to_owned())]
    );
}

#[test]
fn the_model_of_one_stack_is_the_same_every_time() {
    assert_eq!(security_stack(), security_stack());
}

#[test]
fn a_layer_name_of_the_core_is_the_name_of_the_contract() {
    let name = LayerName::try_from("org.team-1").expect("a name");
    assert_eq!(name.to_contract().as_str(), "org.team-1");
}
