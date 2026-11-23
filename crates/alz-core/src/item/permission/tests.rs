use crate::item::{
    Binding, DefaultMode, FloorFlag, PermissionItems, PermissionMode, PermissionRefusal,
    PermissionRule, Permissions,
};
use crate::merge::testing::layer;
use crate::merge::{ListEdit, OrderedSet};

/// What the layer `repo` with this `permissions.yaml` gives.
fn read(yaml: &str) -> (Option<Permissions>, Vec<String>) {
    let (items, found) =
        PermissionItems::read(&layer("repo", "repo", &[("permissions.yaml", yaml)]));
    (
        items.into_file().map(|(_, permissions)| permissions),
        found.iter().map(ToString::to_string).collect(),
    )
}

fn rule(text: &str) -> PermissionRule {
    PermissionRule::try_from(text).expect("a permission rule")
}

fn rules(texts: &[&str]) -> OrderedSet<PermissionRule> {
    texts.iter().copied().map(rule).collect()
}

fn adds(texts: &[&str]) -> ListEdit<PermissionRule> {
    ListEdit::Adjust {
        remove: OrderedSet::default(),
        add: rules(texts),
    }
}

fn only(yaml: &str) -> Permissions {
    let (permissions, found) = read(yaml);
    assert_eq!(found, Vec::<String>::new());
    permissions.expect("a permissions file")
}

fn refused(yaml: &str) -> Vec<String> {
    let (permissions, found) = read(yaml);
    assert_eq!(permissions, None, "{yaml}");
    found
}

#[test]
fn a_layer_without_permissions_yaml_has_no_permissions() {
    let (items, found) = PermissionItems::read(&layer("repo", "repo", &[]));
    assert!(items.into_file().is_none());
    assert_eq!(found.iter().count(), 0);
}

#[test]
fn an_empty_file_says_nothing() {
    assert_eq!(read(""), (None, Vec::new()));
    let permissions = only("allow:\n");
    assert_eq!(permissions.allow(), &ListEdit::default());
    assert_eq!(permissions.default_mode(), None);
}

#[test]
fn a_yaml_list_adds_its_rules_to_the_inherited_list() {
    let permissions =
        only("allow: [\"bash:git status\"]\ndeny: [\"read:.env*\", \"bash:rm -rf *\"]\n");
    assert_eq!(permissions.allow(), &adds(&["bash:git status"]));
    assert_eq!(permissions.deny(), &adds(&["read:.env*", "bash:rm -rf *"]));
    assert_eq!(permissions.ask(), &ListEdit::default());
    assert_eq!(permissions.default_mode(), None);
}

#[test]
fn a_string_edits_the_inherited_list_as_written() {
    let permissions =
        only("deny: \"-[read:.env*]\"\nask: \"-[bash:ls] +[bash:cat *]\"\nallow: \"[read]\"\n");
    assert_eq!(
        permissions.deny(),
        &ListEdit::Adjust {
            remove: rules(&["read:.env*"]),
            add: OrderedSet::default()
        }
    );
    assert_eq!(
        permissions.ask(),
        &ListEdit::Adjust {
            remove: rules(&["bash:ls"]),
            add: rules(&["bash:cat *"])
        }
    );
    assert_eq!(permissions.allow(), &ListEdit::Replace(rules(&["read"])));
}

#[test]
fn a_rule_is_a_tool_with_an_optional_pattern() {
    assert_eq!(rule("bash:echo a:b").to_string(), "bash:echo a:b");
    assert_eq!(rule("mcp__docs__search").to_string(), "mcp__docs__search");
    assert_eq!(
        PermissionRule::try_from("Bash:ls"),
        Err(PermissionRefusal::UnknownTool)
    );
    assert_eq!(
        PermissionRule::try_from("  "),
        Err(PermissionRefusal::Blank)
    );
    assert_eq!(
        PermissionRule::try_from("read:"),
        Err(PermissionRefusal::EmptyPattern)
    );
}

#[test]
fn a_rule_with_an_unknown_tool_or_an_empty_pattern_is_a_permissions_error() {
    assert_eq!(
        refused("deny: [\"nope:x\"]\n"),
        [
            "error [permissions] repo:permissions.yaml:1:8: deny[0] must name a tool: read, write, edit, grep, glob, bash, web_fetch, web_search, agent, notebook_edit, todo_write, skill or an mcp__ tool, then an optional :pattern"
        ]
    );
    assert_eq!(
        refused("allow: \"+[read:]\"\n"),
        ["error [permissions] repo:permissions.yaml:1:8: allow must have a pattern after the ':'"]
    );
}

#[test]
fn an_unknown_field_is_a_permissions_error() {
    assert_eq!(
        refused("allow: []\nforbid: [read]\n"),
        ["error [permissions] repo:permissions.yaml:2:1: unknown field 'forbid'"]
    );
}

#[test]
fn an_unknown_mode_is_a_permissions_error() {
    assert_eq!(
        refused("default_mode: yolo\n"),
        [
            "error [permissions] repo:permissions.yaml:1:15: default_mode must be bypass_permissions, auto, accept_edits, default, dont_ask or plan"
        ]
    );
}

#[test]
fn floor_without_a_default_mode_is_a_permissions_error() {
    assert_eq!(
        refused("floor: true\n"),
        ["error [permissions] repo:permissions.yaml:1:1: floor needs a default_mode"]
    );
    assert_eq!(
        refused("allow: [read]\nfloor: false\n"),
        ["error [permissions] repo:permissions.yaml:2:1: floor needs a default_mode"]
    );
}

#[test]
fn a_floor_that_is_not_a_boolean_is_a_permissions_error() {
    assert_eq!(refused("default_mode: plan\nfloor: yes please\n").len(), 1);
}

#[test]
fn a_default_mode_is_free_unless_floor_is_true() {
    let free = only("default_mode: plan\n");
    assert_eq!(
        free.default_mode(),
        Some(DefaultMode::new(PermissionMode::Plan, Binding::Free))
    );
    let free = only("default_mode: plan\nfloor: false\n");
    assert_eq!(
        free.default_mode().map(DefaultMode::binding),
        Some(Binding::Free)
    );
    let bound = only("default_mode: dont_ask\nfloor: true\n");
    assert_eq!(
        bound.default_mode(),
        Some(DefaultMode::new(PermissionMode::DontAsk, Binding::Floor))
    );
}

#[test]
fn a_file_that_is_not_a_mapping_is_a_permissions_error() {
    assert_eq!(
        refused("- read\n"),
        ["error [permissions] repo:permissions.yaml:1:1: the document must be a mapping"]
    );
}

#[test]
fn every_problem_of_the_file_is_reported() {
    assert_eq!(
        refused("deny: [nope]\nask: [also-nope]\ndefault_mode: yolo\n").len(),
        3
    );
}

#[test]
fn the_modes_are_ordered_from_loosest_to_strictest() {
    use PermissionMode::{AcceptEdits, Auto, BypassPermissions, Default, DontAsk, Plan};
    let mut modes = vec![Plan, Default, BypassPermissions, DontAsk, Auto, AcceptEdits];
    modes.sort();
    assert_eq!(
        modes,
        [BypassPermissions, Auto, AcceptEdits, Default, DontAsk, Plan]
    );
}

// The flags that only move to true (§7.6).

#[test]
fn the_flags_are_read_from_permissions_yaml() {
    let permissions =
        only("disable_bypass_mode: true\nblock_reads_outside: false\nblock_writes_outside: true\n");
    assert_eq!(permissions.flag(FloorFlag::DisableBypassMode), Some(true));
    assert_eq!(permissions.flag(FloorFlag::BlockReadsOutside), Some(false));
    assert_eq!(permissions.flag(FloorFlag::BlockWritesOutside), Some(true));
}

#[test]
fn a_flag_that_is_not_a_boolean_is_a_permissions_error() {
    assert_eq!(
        refused("block_reads_outside: yes please\n").len(),
        1,
        "text is not a boolean"
    );
}

#[test]
fn a_file_without_flags_sets_none() {
    let permissions = only("allow: [bash:ls]\n");
    assert!(
        FloorFlag::ALL
            .iter()
            .all(|flag| permissions.flag(*flag).is_none())
    );
}
