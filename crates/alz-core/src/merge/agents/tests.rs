use crate::item::{AgentName, ModelId, OverlayMode, Role, SkillName, ToolGrant};
use crate::item::{EditSegment, EditSign, WrittenEdit};
use crate::item::{NativeMap, NativeValue};
use crate::merge::action::{Action, AgentChange};
use crate::merge::provenance::Traced;
use crate::merge::testing::{layer, lines_but_slots as lines, merged_without_slots as merged};
use crate::merge::{Merged, MergedAgent};
use crate::stack::ResolvedLayer;
use alz_adapter::TargetId;

// The files of acceptance case 03 (§13.3) and case 18 (§13.18), read from the fixtures themselves.
const BASE_03: &str =
    include_str!("../../../../../fixtures/03-agent-overlay/personal/agents/architect.md");
const CLIENT_03: &str =
    include_str!("../../../../../fixtures/03-agent-overlay/client/agents/architect.overlay.md");
const REPO_03: &str = include_str!(
    "../../../../../fixtures/03-agent-overlay/repo/.layer/agents/architect.overlay.md"
);
const EXPECTED_03: &str =
    include_str!("../../../../../fixtures/03-agent-overlay/expected/.claude/agents/architect.md");
const BASE_18: &str =
    include_str!("../../../../../fixtures/18-native-fields/personal/agents/architect.md");
const OVERLAY_18: &str = include_str!(
    "../../../../../fixtures/18-native-fields/repo/.layer/agents/architect.overlay.md"
);
const BAD_18: &str =
    include_str!("../../../../../fixtures/18-native-fields/bad/.layer/agents/reviewer.md");

fn case_03() -> Vec<ResolvedLayer> {
    vec![
        layer("personal", "personal", &[("agents/architect.md", BASE_03)]),
        layer(
            "acme",
            "client",
            &[("agents/architect.overlay.md", CLIENT_03)],
        ),
        layer(
            "shop-api",
            "repo",
            &[("agents/architect.overlay.md", REPO_03)],
        ),
    ]
}

fn merged_model(layers: Vec<ResolvedLayer>) -> Merged {
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    found.merged().expect("no error was found").clone()
}

fn agent<'a>(model: &'a Merged, name: &str) -> &'a Traced<MergedAgent> {
    model
        .agents()
        .get(&AgentName::try_from(name).expect("an agent name"))
        .expect("the agent exists")
}

fn makers(agent: &Traced<MergedAgent>) -> Vec<(&str, Action)> {
    agent
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action().clone()))
        .collect()
}

/// A mapping in flow style, keys in the order the map holds them.
fn show(map: &NativeMap) -> String {
    let entries: Vec<String> = map
        .iter()
        .map(|(key, value)| format!("{key}: {}", show_value(value)))
        .collect();
    format!("{{{}}}", entries.join(", "))
}

fn show_value(value: &NativeValue) -> String {
    match value {
        NativeValue::Null => "null".to_owned(),
        NativeValue::Bool(flag) => flag.to_string(),
        NativeValue::Int(number) => number.to_string(),
        NativeValue::Float(number) => number.get().to_string(),
        NativeValue::Text(text) => text.clone(),
        NativeValue::List(items) => format!(
            "[{}]",
            items.iter().map(show_value).collect::<Vec<_>>().join(", ")
        ),
        NativeValue::Map(map) => show(map),
    }
}

fn native_of(model: &Merged, name: &str, target: &TargetId) -> String {
    show(
        agent(model, name)
            .value()
            .native()
            .of(target)
            .expect("native fields"),
    )
}

fn names<T>(items: impl IntoIterator<Item = T>, text: impl Fn(&T) -> &str) -> Vec<String> {
    items
        .into_iter()
        .map(|item| text(&item).to_owned())
        .collect()
}

fn skills(agent: &MergedAgent) -> Vec<&str> {
    agent.skills().iter().map(SkillName::as_str).collect()
}

fn tools(agent: &MergedAgent) -> Option<Vec<String>> {
    agent
        .tools()
        .map(|tools| tools.iter().map(ToolGrant::to_string).collect())
}

/// The tool entries as the list a merged agent holds.
fn entries(list: &[&str]) -> Vec<String> {
    list.iter().map(|entry| (*entry).to_owned()).collect()
}

/// A stack of a base agent in `personal` and overlays in `acme`.
fn with_overlay(base: &str, overlay: &str) -> Vec<ResolvedLayer> {
    vec![
        layer("personal", "personal", &[("agents/a.md", base)]),
        layer("acme", "client", &[("agents/a.overlay.md", overlay)]),
    ]
}

const BASE: &str = "---\nname: a\ndescription: Plans\nrole: planning\ntools: [read, \"write:docs/**\"]\nskills: [one]\n---\nbase\n";

// Acceptance case 03, first case: "base + skills overlay + body overlay give one merged agent".
#[test]
fn a_base_a_skills_overlay_and_a_body_overlay_give_one_agent() {
    let model = merged_model(case_03());
    assert_eq!(
        names(model.agents().keys(), |name| name.as_str()),
        ["architect"]
    );
    let architect = agent(&model, "architect");
    let value = architect.value();
    assert_eq!(skills(value), ["arch-standards", "domain-glossary"]);
    assert_eq!(
        value.description().as_str(),
        "Plans system structure and records decisions"
    );
    assert_eq!(value.scalars().role().map(Role::as_str), Some("planning"));
    assert_eq!(
        tools(value),
        Some(entries(&["read", "grep", "glob", "write:docs/**"]))
    );
    // The body of the generated file's expected text is the composed body.
    let composed = value.body().compose();
    assert!(
        EXPECTED_03.ends_with(&format!("---\n\n{composed}\n")),
        "{composed}"
    );
    assert_eq!(value.body().sections().len(), 2);
}

// Acceptance case 03, "explain agent architect lists all three sources in order".
#[test]
fn the_agent_lists_its_three_sources_in_stack_order() {
    let model = merged_model(case_03());
    assert_eq!(
        makers(agent(&model, "architect")),
        [
            ("personal", Action::Define),
            (
                "acme",
                Action::Overlay {
                    mode: OverlayMode::Extend,
                    changes: vec![
                        AgentChange::Skills(WrittenEdit::new(vec![EditSegment::new(
                            EditSign::Add,
                            vec!["domain-glossary".to_owned()]
                        )])),
                        AgentChange::BodySection
                    ]
                }
            ),
            (
                "shop-api",
                Action::Overlay {
                    mode: OverlayMode::Extend,
                    changes: vec![AgentChange::BodySection]
                }
            )
        ]
    );
}

#[test]
fn the_body_has_one_section_per_layer_under_the_layer_kind_and_name() {
    let model = merged_model(case_03());
    let body = agent(&model, "architect").value().body().compose();
    assert_eq!(
        body,
        "You are the architect. Propose structure before code.\n\n\
         ## Client context: acme\n\n\
         Acme context: orders, invoices and payouts are separate bounded contexts.\n\n\
         ## Repo context: shop-api\n\n\
         This repo owns the orders API contract."
    );
}

// Acceptance case 18, first case: "passes claude: fields through, and overlays deep-merge them".
#[test]
fn claude_fields_pass_through_and_overlays_deep_merge_them() {
    let model = merged_model(vec![
        layer("personal", "personal", &[("agents/architect.md", BASE_18)]),
        layer(
            "shop-api",
            "repo",
            &[("agents/architect.overlay.md", OVERLAY_18)],
        ),
    ]);
    let architect = agent(&model, "architect").value();
    let claude = architect
        .native()
        .of(crate::target::claude())
        .expect("claude fields");
    // The order is that of the source: the base's keys, then the key the overlay added.
    assert_eq!(show(claude), "{effort: high, maxTurns: 60, color: blue}");
    assert!(architect.native().of(crate::target::codex()).is_none());
}

// Acceptance case 18, second case: "rejects a claude: field that has a neutral equivalent".
#[test]
fn a_claude_key_with_a_neutral_field_is_refused_naming_the_key() {
    let found = merged(vec![layer(
        "shop-api",
        "repo",
        &[("agents/reviewer.md", BAD_18)],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [agent] shop-api:agents/reviewer.md:5:3: 'claude.tools' has a neutral field; set it outside the target map so every target gets it"
        ]
    );
}

#[test]
fn every_neutral_key_of_each_target_is_refused_and_every_one_is_reported() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[(
            "agents/a.md",
            "---\nname: a\ndescription: d\nclaude:\n  model: x\n  effort: high\n  skills: []\ncodex:\n  developer_instructions: y\n---\n",
        )],
    )]);
    let keys: Vec<String> = lines(&found)
        .iter()
        .filter_map(|line| line.split(" has a neutral field").next())
        .filter_map(|head| head.rsplit(": ").next())
        .map(str::to_owned)
        .collect();
    assert_eq!(
        keys,
        [
            "'claude.model'",
            "'claude.skills'",
            "'codex.developer_instructions'"
        ]
    );
}

#[test]
fn an_overlay_may_not_put_a_neutral_key_in_a_target_map_either() {
    let found = merged(with_overlay(
        BASE,
        "---\nextends: a\nclaude:\n  memory: project\n---\n",
    ));
    assert_eq!(lines(&found).len(), 1);
    assert!(lines(&found)[0].contains("'claude.memory' has a neutral field"));
}

#[test]
fn native_maps_deep_merge_nested_maps_and_lists_and_stay_per_target() {
    let model = merged_model(with_overlay(
        "---\nname: a\ndescription: d\nclaude:\n  effort: low\n  disallowedTools: [Bash]\ncodex:\n  extras:\n    Stop: [one]\n    Start: [x]\n  sandbox: ro\n---\n",
        "---\nextends: a\nclaude:\n  disallowedTools: [Write, Bash]\ncodex:\n  extras:\n    Stop: [two, one]\n  sandbox: rw\n  extra: 1\n---\n",
    ));
    // `Stop` keeps its place and gets `two` after `one`; `Start` stays after it.
    assert_eq!(
        native_of(&model, "a", crate::target::claude()),
        "{effort: low, disallowedTools: [Bash, Write]}"
    );
    assert_eq!(
        native_of(&model, "a", crate::target::codex()),
        "{extras: {Stop: [one, two], Start: [x]}, sandbox: rw, extra: 1}"
    );
}

// TASK-66: native fields keep source order, and a whole-number key is a key like any other.
#[test]
fn native_keys_keep_source_order_and_a_whole_number_key_is_not_hoisted() {
    let model = merged_model(vec![layer(
        "personal",
        "personal",
        &[(
            "agents/a.md",
            "---\nname: a\ndescription: d\ncodex:\n  zeta: 1\n  10: ten\n  9: nine\n  alpha: a\n---\n",
        )],
    )]);
    assert_eq!(
        native_of(&model, "a", crate::target::codex()),
        "{zeta: 1, 10: ten, 9: nine, alpha: a}"
    );
}

#[test]
fn a_key_a_later_layer_adds_comes_after_the_existing_ones_and_a_replaced_value_stays_in_place() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[(
                "agents/a.md",
                "---\nname: a\ndescription: d\ncodex:\n  zeta: 1\n  mid: {b: 1, a: 2}\n  alpha: 3\n---\n",
            )],
        ),
        layer(
            "acme",
            "client",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\ncodex:\n  10: ten\n  alpha: changed\n  mid: {c: 3, a: 9}\n  aaa: new\n---\n",
            )],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\ncodex:\n  9: nine\n  zeta: 2\n---\n",
            )],
        ),
    ]);
    assert_eq!(
        native_of(&model, "a", crate::target::codex()),
        "{zeta: 2, mid: {b: 1, a: 9, c: 3}, alpha: changed, 10: ten, aaa: new, 9: nine}"
    );
}

#[test]
fn list_items_that_are_mappings_equal_in_another_key_order_are_one_item() {
    let model = merged_model(with_overlay(
        "---\nname: a\ndescription: d\ncodex:\n  l: [{x: 1, y: 2}, 5]\n---\n",
        "---\nextends: a\ncodex:\n  l: [{y: 2, x: 1}, 6, 5]\n---\n",
    ));
    assert_eq!(
        native_of(&model, "a", crate::target::codex()),
        "{l: [{x: 1, y: 2}, 5, 6]}"
    );
}

#[test]
fn two_base_agents_with_one_name_are_an_error_naming_the_first_layer() {
    let found = merged(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer("acme", "client", &[("agents/a.md", BASE)]),
    ]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [agent-duplicate] acme:agents/a.md: agent a is already defined by personal; use an overlay or mode: replace"
        ]
    );
}

#[test]
fn an_overlay_without_a_base_is_unresolved() {
    let found = merged(vec![layer(
        "acme",
        "client",
        &[("agents/a.overlay.md", "---\nextends: a\n---\ntext\n")],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] acme:agents/a.overlay.md: extends a, but no more general layer defines it"
        ]
    );
}

#[test]
fn an_overlay_whose_extends_differs_from_its_file_name_is_an_error() {
    let found = merged(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer(
            "acme",
            "client",
            &[("agents/a.overlay.md", "---\nextends: b\n---\n")],
        ),
    ]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [agent-overlay] acme:agents/a.overlay.md: extends must be a to match the file name"
        ]
    );
}

#[test]
fn an_overlay_without_extends_is_an_agent_overlay_error() {
    let found = merged(with_overlay(BASE, "---\ndescription: x\n---\n"));
    assert_eq!(
        lines(&found),
        ["error [agent-overlay] acme:agents/a.overlay.md:1:1: extends is required"]
    );
}

#[test]
fn an_overlay_mode_other_than_extend_or_replace_is_an_agent_overlay_error() {
    let found = merged(with_overlay(BASE, "---\nextends: a\nmode: append\n---\n"));
    assert_eq!(
        lines(&found),
        ["error [agent-overlay] acme:agents/a.overlay.md:3:7: mode must be extend or replace"]
    );
}

#[test]
fn a_replace_overlay_replaces_the_whole_body_and_warns() {
    let found = merged(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer(
            "acme",
            "client",
            &[("agents/a.overlay.md", "---\nextends: a\n---\nacme\n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\nmode: replace\n---\nrepo\n",
            )],
        ),
    ]);
    assert_eq!(
        lines(&found),
        [
            "warning [agent-replace] shop-api:agents/a.overlay.md: replaces the whole body of agent a"
        ]
    );
    let model = found.merged().expect("a warning is no error");
    let merged_agent = agent(model, "a");
    assert_eq!(merged_agent.value().body().compose(), "repo");
    assert_eq!(merged_agent.value().body().sections(), []);
    assert_eq!(
        makers(merged_agent),
        [
            ("personal", Action::Define),
            (
                "acme",
                Action::Overlay {
                    mode: OverlayMode::Extend,
                    changes: vec![AgentChange::BodySection]
                }
            ),
            (
                "shop-api",
                Action::Overlay {
                    mode: OverlayMode::Replace,
                    changes: vec![AgentChange::BodyReplaced]
                }
            )
        ]
    );
}

#[test]
fn a_replace_overlay_with_no_base_defines_a_new_agent_without_a_warning() {
    let model = merged_model(vec![layer(
        "acme",
        "client",
        &[(
            "agents/b.overlay.md",
            "---\nextends: b\nmode: replace\ndescription: New\nskills: +[x, y]\n---\nbody\n",
        )],
    )]);
    let created = agent(&model, "b");
    assert_eq!(makers(created), [("acme", Action::Define)]);
    assert_eq!(created.value().description().as_str(), "New");
    assert_eq!(skills(created.value()), ["x", "y"]);
    assert_eq!(created.value().body().compose(), "body");
    assert_eq!(tools(created.value()), None);
}

#[test]
fn a_replace_overlay_with_no_base_needs_a_description() {
    let found = merged(vec![layer(
        "acme",
        "client",
        &[(
            "agents/b.overlay.md",
            "---\nextends: b\nmode: replace\n---\nbody\n",
        )],
    )]);
    assert_eq!(
        lines(&found),
        ["error [agent] acme:agents/b.overlay.md: description is required to define a new agent"]
    );
}

#[test]
fn a_scalar_is_won_by_the_most_specific_layer_and_a_null_keeps_the_inherited_value() {
    let model = merged_model(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer(
            "acme",
            "client",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\ndescription: Acme plans\nrole: ~\nmodel: opus\nmemory: user\n---\n",
            )],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\ndescription: ~\nmodel: sonnet\nisolation: worktree\npermission_mode: plan\n---\n",
            )],
        ),
    ]);
    let value = agent(&model, "a").value();
    assert_eq!(value.description().as_str(), "Acme plans");
    let scalars = value.scalars();
    assert_eq!(scalars.role().map(Role::as_str), Some("planning"));
    assert_eq!(scalars.model().map(ModelId::as_str), Some("sonnet"));
    assert_eq!(scalars.memory().copied(), Some(crate::item::Memory::User));
    assert_eq!(
        scalars.isolation().copied(),
        Some(crate::item::Isolation::Worktree)
    );
    assert_eq!(
        scalars.permission_mode().copied(),
        Some(crate::item::PermissionMode::Plan)
    );
}

#[test]
fn list_edits_apply_in_stack_order_and_keep_each_item_once_in_first_seen_order() {
    let model = merged_model(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer(
            "acme",
            "client",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\nskills: +[two, one, three]\ntools: -[\"write:docs/**\"] +[\"write:docs/adr/**\"]\n---\n",
            )],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "agents/a.overlay.md",
                "---\nextends: a\nskills: -[two] +[four]\n---\n",
            )],
        ),
    ]);
    let value = agent(&model, "a").value();
    assert_eq!(skills(value), ["one", "three", "four"]);
    assert_eq!(tools(value), Some(entries(&["read", "write:docs/adr/**"])));
}

#[test]
fn a_yaml_list_in_an_overlay_replaces_the_inherited_list() {
    let model = merged_model(with_overlay(
        BASE,
        "---\nextends: a\nskills: [x, y, x]\ntargets: [codex]\n---\n",
    ));
    let value = agent(&model, "a").value();
    assert_eq!(skills(value), ["x", "y"]);
    assert_eq!(
        value
            .targets()
            .map(|targets| targets.iter().cloned().collect::<Vec<_>>()),
        Some(vec![crate::target::codex().clone()])
    );
}

#[test]
fn removing_a_tool_from_an_agent_that_has_no_tool_list_is_an_error() {
    let found = merged(with_overlay(
        "---\nname: a\ndescription: d\n---\n",
        "---\nextends: a\ntools: -[read]\n---\n",
    ));
    assert_eq!(
        lines(&found),
        [
            "error [agent-overlay] acme:agents/a.overlay.md: agent a inherits all tools; give it an explicit tools list before removing one"
        ]
    );
}

#[test]
fn an_extension_without_text_adds_no_section_and_an_empty_base_gets_no_blank_lines() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("agents/a.md", "---\nname: a\ndescription: d\n---\n")],
        ),
        layer(
            "acme",
            "client",
            &[("agents/a.overlay.md", "---\nextends: a\n---\n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[("agents/a.overlay.md", "---\nextends: a\n---\nrepo text\n")],
        ),
    ]);
    let merged_agent = agent(&model, "a");
    assert_eq!(
        merged_agent.value().body().compose(),
        "## Repo context: shop-api\n\nrepo text"
    );
    assert_eq!(makers(merged_agent).len(), 3);
}

#[test]
fn a_layer_kind_prints_stripped_in_the_heading_of_an_overlay_section() {
    use crate::merge::testing::layer_from_yaml;
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("agents/a.md", "---\nname: a\ndescription: d\n---\nBase\n")],
        ),
        layer_from_yaml(
            "name: acme\nkind: \"te\\u202Eam\"\nvisibility: private\n",
            &[("agents/a.overlay.md", "---\nextends: a\n---\nTeam text\n")],
        ),
    ]);
    assert_eq!(
        agent(&model, "a").value().body().compose(),
        "Base\n\n## Team context: acme\n\nTeam text"
    );
}

#[test]
fn an_overlay_refines_an_agent_that_its_own_layer_defines() {
    let model = merged_model(vec![layer(
        "personal",
        "personal",
        &[
            ("agents/a.md", BASE),
            (
                "agents/a.overlay.md",
                "---\nextends: a\nskills: +[two]\n---\n",
            ),
        ],
    )]);
    assert_eq!(skills(agent(&model, "a").value()), ["one", "two"]);
}

#[test]
fn a_file_that_breaks_its_schema_is_reported_whole_and_gives_no_agent() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[(
            "agents/a.md",
            "---\nname: b\ndescription: ~\ncolour: red\nskills: [Bad]\n---\n",
        )],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [agent] personal:agents/a.md:1:1: description is required",
            "error [agent] personal:agents/a.md:4:1: unknown field 'colour'",
            "error [agent] personal:agents/a.md:5:10: skills[0] must be kebab-case: lower-case letters and digits in groups joined by '-'",
        ]
    );
}

#[test]
fn a_name_that_differs_from_the_file_name_is_an_error() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[("agents/a.md", "---\nname: b\ndescription: d\n---\n")],
    )]);
    assert_eq!(
        lines(&found),
        ["error [agent] personal:agents/a.md: name must match the file name"]
    );
}

// Acceptance case 03, explain: the words of each contribution of the architect agent.
#[test]
fn the_contributions_of_an_agent_give_the_words_of_alz_explain() {
    use crate::render::action_word::ActionWord;

    let model = merged_model(case_03());
    let words: Vec<String> = agent(&model, "architect")
        .provenance()
        .contributions()
        .iter()
        .map(|part| ActionWord::new(part.action(), "base").to_string())
        .collect();
    assert_eq!(
        words,
        [
            "base",
            "extend: skills +[domain-glossary]; body section",
            "extend: body section"
        ]
    );
}

// The edits of a list read as the layer wrote them, in the order written and with an empty edit
// as it is: the strings are the spec's description of each operation.
#[test]
fn the_edits_of_a_list_are_shown_in_the_order_the_layer_wrote_them() {
    use crate::render::action_word::ActionWord;

    let overlay = |skills: &str, targets: &str| {
        format!("---\nextends: a\nskills: \"{skills}\"\ntargets: \"{targets}\"\n---\n")
    };
    let model = merged_model(vec![
        layer("personal", "personal", &[("agents/a.md", BASE)]),
        layer(
            "acme",
            "client",
            &[("agents/a.overlay.md", &overlay("+[b, c] -[a]", "[claude]"))],
        ),
        layer(
            "shop-api",
            "repo",
            &[("agents/a.overlay.md", &overlay("-[x] +[y] -[z]", ""))],
        ),
    ]);
    let words: Vec<String> = agent(&model, "a")
        .provenance()
        .contributions()
        .iter()
        .map(|part| ActionWord::new(part.action(), "base").to_string())
        .collect();
    assert_eq!(
        words,
        [
            "base",
            "extend: skills +[b, c] -[a]; targets [claude]",
            "extend: skills -[x] +[y] -[z]; targets "
        ]
    );
}
