use crate::listing::{Decider, Listing, Section};
use crate::merge::Merged;
use crate::merge::testing::{layer_from_yaml, lines, merged_with};
use crate::untrusted_text::has_control_or_format;

const PERSONAL: &str = "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\nrequires_slots: [domain-glossary]\n";
const REPO: &str = "name: shop-api\nkind: repo\nvisibility: private\n";
const OPTIONS: &str = "options:\n  strict_slots: false\nmcp:\n  enable: [playwright, context7]\n";

const AGENT: &str = "---\nname: architect\ndescription: Plans\n---\nPlan.\n";
const SKILL: &str = "---\nname: adr\ndescription: Records decisions\n---\nText.\n";
const COMMAND: &str = "---\ndescription: Cut a release\n---\nRelease.\n";
const RULE: &str = "---\npaths: [\"src/**\"]\n---\nBackend.\n";
const HOOKS: &str = "- id: zeta\n  event: pre_tool\n  command: ./z.sh\n- id: alpha\n  event: stop\n  command: ./a.sh\n";
const MCP: &str = "context7:\n  transport: http\n  url: https://one.example/mcp\nplaywright:\n  transport: stdio\n  command: npx\nunused:\n  transport: stdio\n  command: npx\n";

fn model() -> Merged {
    let found = merged_with(
        vec![
            layer_from_yaml(
                PERSONAL,
                &[
                    ("agents/architect.md", AGENT),
                    (
                        "agents/builder.md",
                        "---\nname: builder\ndescription: Builds\n---\nB.\n",
                    ),
                    ("skills/adr/SKILL.md", SKILL),
                    ("commands/release.md", COMMAND),
                    ("rules/backend.md", RULE),
                    ("hooks/hooks.yaml", HOOKS),
                    ("mcp.yaml", MCP),
                ],
            ),
            layer_from_yaml(
                REPO,
                &[
                    (
                        "agents/builder.overlay.md",
                        "---\nextends: builder\ndescription: Repo builder\n---\n",
                    ),
                    (
                        "skills/adr/SKILL.md",
                        "---\nname: adr\ndescription: Repo ADRs\n---\nRepo.\n",
                    ),
                ],
            ),
        ],
        OPTIONS,
    );
    let found_lines = lines(&found);
    assert!(
        found_lines
            .iter()
            .all(|line| line.starts_with("warning [slot-missing]")),
        "{found_lines:?}"
    );
    found.merged().expect("merged").clone()
}

fn items(listing: &Listing, wanted: Section) -> Vec<String> {
    listing
        .sections()
        .into_iter()
        .filter(|(section, _)| *section == wanted)
        .flat_map(|(_, items)| items.iter())
        .map(|item| format!("{} {}", item.name(), item.decider()))
        .collect()
}

#[test]
fn an_item_is_decided_by_the_most_specific_layer_that_contributed() {
    let listing = Listing::of(&model());
    assert_eq!(
        items(&listing, Section::Agents),
        ["architect personal", "builder shop-api"]
    );
    assert_eq!(items(&listing, Section::Rules), ["backend personal"]);
}

#[test]
fn skills_and_commands_are_separate_sections_and_a_stub_is_a_skill_of_no_layer() {
    let listing = Listing::of(&model());
    assert_eq!(
        items(&listing, Section::Skills),
        ["adr shop-api", "domain-glossary slot stub"]
    );
    assert_eq!(items(&listing, Section::Commands), ["release personal"]);
}

#[test]
fn hooks_keep_file_order_and_servers_the_order_of_the_allowlist() {
    let listing = Listing::of(&model());
    assert_eq!(
        items(&listing, Section::Hooks),
        ["zeta personal", "alpha personal"]
    );
    assert_eq!(
        items(&listing, Section::Mcp),
        ["playwright personal", "context7 personal"]
    );
}

#[test]
fn the_text_has_six_sections_with_counts_and_names_padded_to_28() {
    let text = Listing::of(&model()).to_string();
    assert_eq!(
        text,
        "agents (2)\n  architect                    personal\n  builder                      shop-api\n\
         skills (2)\n  adr                          shop-api\n  domain-glossary              slot stub\n\
         commands (1)\n  release                      personal\n\
         rules (1)\n  backend                      personal\n\
         hooks (2)\n  zeta                         personal\n  alpha                        personal\n\
         mcp (2)\n  playwright                   personal\n  context7                     personal\n"
    );
}

#[test]
fn a_section_without_items_is_printed_with_the_count_zero() {
    let found = merged_with(vec![layer_from_yaml(REPO, &[])], "");
    let text = Listing::of(found.merged().expect("merged")).to_string();
    assert_eq!(
        text,
        "agents (0)\nskills (0)\ncommands (0)\nrules (0)\nhooks (0)\nmcp (0)\n"
    );
}

#[test]
fn a_hook_id_with_a_format_character_is_printed_stripped() {
    let found = merged_with(
        vec![layer_from_yaml(
            PERSONAL,
            &[(
                "hooks/hooks.yaml",
                "- id: \"gu\\u202eard\"\n  event: pre_tool\n  command: ./guard.sh\n",
            )],
        )],
        OPTIONS.split("mcp:").next().unwrap_or_default(),
    );
    let text = Listing::of(found.merged().expect("merged")).to_string();
    assert!(!has_control_or_format(&text.replace('\n', "")));
    assert!(
        text.contains(&format!("  {:<28} personal\n", "guard")),
        "{text}"
    );
}

#[test]
fn a_decider_is_a_layer_or_a_slot_stub() {
    let listing = Listing::of(&model());
    let stub = listing
        .sections()
        .into_iter()
        .flat_map(|(_, items)| items.iter())
        .find(|item| item.name() == "domain-glossary")
        .expect("the stub is listed");
    assert_eq!(stub.decider(), &Decider::SlotStub);
}
