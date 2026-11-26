//! An agent overlay may only make an agent stricter (spec §7.2): its `permission_mode` is no looser
//! than the mode of the agent it refines nor than the permissions floor of the more general layers,
//! and `claude.disallowedTools` only grows down the stack.

use proptest::prelude::{Just, Strategy, prop_assert, prop_assert_eq, prop_oneof, proptest};

use crate::item::{AgentName, PermissionMode};
use crate::merge::provenance::Traced;
use crate::merge::testing::{layer, lines, merged};
use crate::merge::{MergedAgent, Merging};
use crate::stack::ResolvedLayer;

const MODES: [PermissionMode; 6] = [
    PermissionMode::BypassPermissions,
    PermissionMode::Auto,
    PermissionMode::AcceptEdits,
    PermissionMode::Default,
    PermissionMode::DontAsk,
    PermissionMode::Plan,
];

/// `personal` holds the agent `a` with `base_front` and, when given, a floor at `floor`; `shop-api`
/// holds an overlay with `overlay_front`.
fn stack(
    base_front: &str,
    floor: Option<PermissionMode>,
    overlay_front: &str,
) -> Vec<ResolvedLayer> {
    let agent = format!("---\nname: a\ndescription: Plans\n{base_front}\n---\nbase\n");
    let permissions = floor.map_or_else(String::new, |mode| {
        format!("default_mode: {}\nfloor: true\n", mode.as_str())
    });
    let overlay = format!("---\nextends: a\n{overlay_front}\n---\n");
    vec![
        layer(
            "personal",
            "personal",
            &[
                ("agents/a.md", agent.as_str()),
                ("permissions.yaml", permissions.as_str()),
            ],
        ),
        layer(
            "shop-api",
            "repo",
            &[("agents/a.overlay.md", overlay.as_str())],
        ),
    ]
}

fn agent_of(found: &Merging) -> Option<&MergedAgent> {
    let name = AgentName::try_from("a").expect("an agent name");
    found
        .merged()
        .and_then(|model| model.agents().get(&name))
        .map(Traced::value)
}

fn mode_line(front: &str) -> String {
    format!("permission_mode: {front}")
}

#[test]
fn an_overlay_may_not_set_a_permission_mode_looser_than_the_base_agents() {
    let found = merged(stack(&mode_line("plan"), None, &mode_line("accept_edits")));
    assert_eq!(
        lines(&found),
        [
            "error [agent-overlay] shop-api:agents/a.overlay.md: overlay on agent a sets permission_mode 'accept_edits', which is looser than 'plan' of the agent it refines; an overlay may only make an agent stricter"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn an_overlay_may_not_set_a_permission_mode_looser_than_the_permissions_floor() {
    let found = merged(stack("", Some(PermissionMode::Default), &mode_line("auto")));
    assert_eq!(
        lines(&found),
        [
            "error [agent-overlay] shop-api:agents/a.overlay.md: overlay on agent a sets permission_mode 'auto', which is looser than the permissions floor 'default' set by layer personal; an overlay may only make an agent stricter"
        ]
    );
}

#[test]
fn an_overlay_may_set_a_permission_mode_as_strict_as_the_base_and_the_floor() {
    let found = merged(stack(
        &mode_line("accept_edits"),
        Some(PermissionMode::Default),
        &mode_line("dont_ask"),
    ));
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(
        agent_of(&found).and_then(|agent| agent.scalars().permission_mode().copied()),
        Some(PermissionMode::DontAsk)
    );
}

#[test]
fn an_overlay_that_defines_a_new_agent_is_held_to_the_floor_too() {
    let layers = vec![
        layer(
            "personal",
            "personal",
            &[("permissions.yaml", "default_mode: default\nfloor: true\n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "agents/b.overlay.md",
                "---\nextends: b\nmode: replace\ndescription: New\npermission_mode: auto\n---\n",
            )],
        ),
    ];
    assert_eq!(
        lines(&merged(layers)),
        [
            "error [agent-overlay] shop-api:agents/b.overlay.md: overlay on agent b sets permission_mode 'auto', which is looser than the permissions floor 'default' set by layer personal; an overlay may only make an agent stricter"
        ]
    );
}

const DISALLOWED: &str = "claude:\n  disallowedTools:\n    - Bash\n    - WebFetch";

#[test]
fn an_overlay_may_add_to_claude_disallowed_tools_and_keeps_the_earlier_entries() {
    let found = merged(stack(
        DISALLOWED,
        None,
        "claude:\n  disallowedTools: [Write, Bash]",
    ));
    assert_eq!(lines(&found), Vec::<String>::new());
    let claude = agent_of(&found)
        .and_then(|agent| agent.native().of(crate::target::claude()))
        .and_then(|native| native.get("disallowedTools"))
        .map(|value| format!("{:?}", value.to_tree()));
    let shown = claude.expect("the field");
    assert!(
        ["Bash", "WebFetch", "Write"]
            .iter()
            .all(|tool| shown.contains(tool))
    );
}

#[test]
fn an_overlay_may_not_remove_or_replace_what_a_more_general_layer_disallowed() {
    // A value of another type, `~` or a mapping, is refused when the overlay is read (TASK-79).
    let removal = "claude:\n  disallowedTools: Write";
    let found = merged(stack(DISALLOWED, None, removal));
    assert_eq!(
        lines(&found),
        [
            "error [agent-overlay] shop-api:agents/a.overlay.md: overlay on agent a removes entries of claude.disallowedTools that a more general layer set; the list only grows down the stack"
        ],
        "{removal}"
    );
    assert!(found.merged().is_none());
}

#[test]
fn an_overlay_may_set_claude_disallowed_tools_when_no_more_general_layer_did() {
    let found = merged(stack("", None, "claude:\n  disallowedTools: [Bash]"));
    assert_eq!(lines(&found), Vec::<String>::new());
}

fn modes() -> impl Strategy<Value = Option<PermissionMode>> {
    prop_oneof![
        Just(None),
        (0..MODES.len()).prop_map(|index| MODES.get(index).copied()),
    ]
}

proptest! {
    /// Whatever the base, the floor and the overlay say, the merge either refuses or gives an
    /// agent whose mode is no looser than the base's and the floor's, and it refuses exactly when
    /// the overlay's mode is looser than either.
    #[test]
    fn an_overlay_never_loosens_the_permission_mode(
        base in modes(),
        floor in modes(),
        overlay in modes(),
    ) {
        let found = merged(stack(
            &base.map_or_else(String::new, |mode| mode_line(mode.as_str())),
            floor,
            &overlay.map_or_else(String::new, |mode| mode_line(mode.as_str())),
        ));
        let tried_looser = overlay.is_some_and(|mode| {
            base.is_some_and(|base| mode < base) || floor.is_some_and(|floor| mode < floor)
        });
        prop_assert_eq!(found.diagnostics().has_errors(), tried_looser);
        if let Some(agent) = agent_of(&found) {
            let result = agent.scalars().permission_mode().copied();
            prop_assert!(result >= base || base.is_none());
            prop_assert!(result.is_none() || result >= floor || overlay.is_none());
            prop_assert_eq!(result, overlay.or(base));
        }
    }
}
