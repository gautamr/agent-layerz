use crate::explain::{ItemType, explain};
use crate::merge::Merged;
use crate::merge::testing::{layer_from_yaml, lines, merged_with};
use crate::untrusted_text::has_control_or_format;

const PERSONAL: &str = "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n";
const CLIENT: &str = "name: acme\nkind: client\nvisibility: private\n";
const REPO: &str = "name: shop-api\nkind: repo\nvisibility: private\n";
const OPTIONS: &str = "options:\n  strict_slots: false\nmcp:\n  enable: [context7]\n";

const AGENT: &str = "---\nname: architect\ndescription: Plans\nskills: [adr]\n---\nPlan.\n";
const OVERLAY: &str = "---\nextends: architect\nskills: +[domain-glossary]\n---\nAcme context.\n";
const SKILL: &str = "---\nname: adr\ndescription: Records decisions\n---\nPersonal text.\n";
const COMMAND: &str = "---\ndescription: Cut a release\n---\nRelease.\n";
const RULE: &str = "---\npaths: [\"**/*.test.ts\"]\n---\nOne behaviour per test.\n";
const EXTENSION: &str = "---\nmode: extend\npaths: [\"e2e/**\"]\n---\nRun e2e.\n";
const BACKEND: &str = "---\npaths: [\"src/**\"]\n---\nBackend.\n";
const HOOKS: &str = "- id: guard\n  event: pre_tool\n  command: ./guard.sh\n";
const MCP: &str = "context7:\n  transport: http\n  url: https://one.example/mcp\nplaywright:\n  transport: stdio\n  command: npx\n";
const MCP_REPLACED: &str = "context7:\n  transport: http\n  url: https://two.example/mcp\n";

fn model() -> Merged {
    let found = merged_with(
        vec![
            layer_from_yaml(
                PERSONAL,
                &[
                    ("agents/architect.md", AGENT),
                    ("skills/adr/SKILL.md", SKILL),
                    ("commands/release.md", COMMAND),
                    ("rules/testing.md", RULE),
                    ("rules/backend.md", BACKEND),
                    ("hooks/hooks.yaml", HOOKS),
                    ("mcp.yaml", MCP),
                    ("instructions/00-core.md", "Core.\n"),
                ],
            ),
            layer_from_yaml(CLIENT, &[("agents/architect.overlay.md", OVERLAY)]),
            layer_from_yaml(
                REPO,
                &[
                    ("rules/testing.md", EXTENSION),
                    ("rules/backend.md", BACKEND),
                    (
                        "skills/adr/SKILL.md",
                        "---\nname: adr\ndescription: Repo ADRs\n---\nRepo text.\n",
                    ),
                    ("mcp.yaml", MCP_REPLACED),
                    ("instructions/50-repo.md", "Repo.\n"),
                ],
            ),
        ],
        OPTIONS,
    );
    assert!(
        lines(&found)
            .iter()
            .all(|line| line.starts_with("warning [slot-missing]")),
        "{:?}",
        lines(&found)
    );
    found.merged().expect("merged").clone()
}

/// The `<source> — <action>` of each contribution.
fn rows(merged: &Merged, item: ItemType, name: &str) -> Vec<String> {
    explain(merged, item, name)
        .expect("the item exists")
        .sources()
        .iter()
        .map(|source| format!("{} — {}", source.source(), source.action()))
        .collect()
}

#[test]
fn an_agent_lists_its_base_then_each_overlay_with_what_it_changed() {
    assert_eq!(
        rows(&model(), ItemType::Agent, "architect"),
        [
            "personal@1.0.0:agents/architect.md — base",
            "acme:agents/architect.overlay.md — extend: skills +[domain-glossary]; body section",
        ]
    );
}

#[test]
fn a_rule_that_another_extends_shows_the_paths_it_adds() {
    assert_eq!(
        rows(&model(), ItemType::Rule, "testing"),
        [
            "personal@1.0.0:rules/testing.md — rule",
            "shop-api:rules/testing.md — extend: paths +[e2e/**]",
        ]
    );
}

#[test]
fn a_rule_that_a_more_specific_layer_replaced_is_marked_overridden() {
    assert_eq!(
        rows(&model(), ItemType::Rule, "backend"),
        [
            "personal@1.0.0:rules/backend.md — rule (overridden)",
            "shop-api:rules/backend.md — rule",
        ]
    );
}

#[test]
fn a_skill_that_a_more_specific_layer_replaced_is_marked_overridden() {
    assert_eq!(
        rows(&model(), ItemType::Skill, "adr"),
        [
            "personal@1.0.0:skills/adr — skill (overridden)",
            "shop-api:skills/adr — skill",
        ]
    );
}

#[test]
fn a_command_is_a_command_and_not_a_skill() {
    let merged = model();
    assert_eq!(
        rows(&merged, ItemType::Command, "release"),
        ["personal@1.0.0:commands/release.md — command"]
    );
    assert!(explain(&merged, ItemType::Skill, "release").is_err());
    assert!(explain(&merged, ItemType::Command, "adr").is_err());
}

#[test]
fn a_slot_stub_has_the_source_slot_stub_and_the_word_stub() {
    assert_eq!(
        rows(&model(), ItemType::Skill, "domain-glossary"),
        ["slot stub — stub"]
    );
}

#[test]
fn a_server_that_a_more_specific_layer_replaced_is_marked_overridden() {
    let merged = model();
    assert_eq!(
        rows(&merged, ItemType::Mcp, "context7"),
        [
            "personal@1.0.0:mcp.yaml — server context7 (overridden)",
            "shop-api:mcp.yaml — server context7",
        ]
    );
    assert!(
        !explain(&merged, ItemType::Mcp, "context7")
            .expect("defined")
            .is_defined_but_not_enabled()
    );
}

#[test]
fn a_server_that_is_defined_and_not_enabled_says_so_first() {
    let explanation = explain(&model(), ItemType::Mcp, "playwright").expect("defined");
    assert!(explanation.is_defined_but_not_enabled());
    assert_eq!(
        explanation.to_string(),
        "mcp playwright is defined but not enabled (add it to mcp.enable in layer.yaml)\n\
         mcp playwright\n\
         sources, general → specific:\n  \
         1. personal@1.0.0:mcp.yaml — server playwright\n"
    );
}

#[test]
fn a_hook_names_its_file_and_id() {
    assert_eq!(
        rows(&model(), ItemType::Hook, "guard"),
        ["personal@1.0.0:hooks/hooks.yaml — hook guard"]
    );
}

#[test]
fn an_instruction_is_found_by_its_file_name_and_each_block_is_a_block() {
    let merged = model();
    assert_eq!(
        rows(&merged, ItemType::Instruction, "00-core.md"),
        ["personal@1.0.0:instructions/00-core.md — block"]
    );
    assert_eq!(
        rows(&merged, ItemType::Instruction, "50-repo.md"),
        ["shop-api:instructions/50-repo.md — block"]
    );
}

#[test]
fn the_text_lists_the_sources_counted_from_one() {
    let explanation = explain(&model(), ItemType::Rule, "testing").expect("exists");
    assert_eq!(
        explanation.to_string(),
        "rule testing\n\
         sources, general → specific:\n  \
         1. personal@1.0.0:rules/testing.md — rule\n  \
         2. shop-api:rules/testing.md — extend: paths +[e2e/**]\n"
    );
}

#[test]
fn each_type_names_the_generated_files_that_hold_the_item() {
    let merged = model();
    let outputs = |item, name| {
        explain(&merged, item, name)
            .expect("exists")
            .outputs()
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        outputs(ItemType::Agent, "architect"),
        [".claude/agents/architect.md"]
    );
    assert_eq!(
        outputs(ItemType::Skill, "adr"),
        [".claude/skills/adr/SKILL.md"]
    );
    assert_eq!(
        outputs(ItemType::Command, "release"),
        [".claude/skills/release/SKILL.md"]
    );
    assert_eq!(
        outputs(ItemType::Rule, "testing"),
        [".claude/rules/testing.md"]
    );
    assert_eq!(outputs(ItemType::Hook, "guard"), [".claude/settings.json"]);
    assert_eq!(outputs(ItemType::Mcp, "context7"), [".mcp.json"]);
    assert_eq!(outputs(ItemType::Instruction, "00-core.md"), ["AGENTS.md"]);
}

#[test]
fn a_name_the_merged_result_does_not_have_is_the_error_explain() {
    let missing = explain(&model(), ItemType::Agent, "nobody").expect_err("no such agent");
    assert_eq!(
        missing.into_diagnostic().to_string(),
        "error [explain] no agent named 'nobody' in the merged result"
    );
}

#[test]
fn a_typed_name_is_stripped_before_it_is_printed() {
    let missing = explain(&model(), ItemType::Rule, "a\u{1b}[2J\nforged \u{202e}line")
        .expect_err("no such rule");
    let line = missing.into_diagnostic().to_string();
    assert_eq!(
        line,
        "error [explain] no rule named 'a[2Jforged line' in the merged result"
    );
    assert!(!has_control_or_format(&line));
}

#[test]
fn a_hook_id_with_a_format_character_is_stripped_in_the_name_and_the_action() {
    let found = merged_with(
        vec![layer_from_yaml(
            PERSONAL,
            &[(
                "hooks/hooks.yaml",
                "- id: \"gu\\u202eard\"\n  event: pre_tool\n  command: ./guard.sh\n",
            )],
        )],
        "",
    );
    let merged = found.merged().expect("merged").clone();
    let explanation = explain(&merged, ItemType::Hook, "gu\u{202e}ard").expect("exists");
    let text = explanation.to_string();
    assert!(!has_control_or_format(&text.replace('\n', "")));
    assert_eq!(
        text,
        "hook guard\nsources, general → specific:\n  1. personal@1.0.0:hooks/hooks.yaml — hook guard\n"
    );
}
