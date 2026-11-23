use std::collections::BTreeMap;

use crate::merge::Tree;
use crate::target::TargetName;

use super::{
    AGENT_KEYS, KEYS, KeyRule, Looseness, NativeKey, Step, Strictness, loosenings, native_key,
    refusal,
};

/// No loosening.
const NOTHING: [Looseness; 0] = [];

/// The tree with `value` at the dotted `path`.
fn at(path: &str, value: Tree) -> Tree {
    path.rsplit('.').fold(value, |inner, key| {
        Tree::Map(BTreeMap::from([(key.to_owned(), inner)]))
    })
}

fn text(value: &str) -> Tree {
    Tree::Text(value.to_owned())
}

fn list(values: &[&str]) -> Tree {
    Tree::List(values.iter().copied().map(text).collect())
}

/// How `after` loosens `before` at `path` of Claude's settings.
fn claude(path: &str, before: &Tree, after: &Tree) -> Vec<Looseness> {
    loosenings(
        TargetName::Claude,
        &at(path, before.clone()),
        &at(path, after.clone()),
    )
    .into_iter()
    .map(|found| found.how())
    .collect()
}

fn codex(path: &str, before: &Tree, after: &Tree) -> Vec<Looseness> {
    loosenings(
        TargetName::Codex,
        &at(path, before.clone()),
        &at(path, after.clone()),
    )
    .into_iter()
    .map(|found| found.how())
    .collect()
}

/// The steps of a key path, with `*` as a key and `**` as nothing.
fn instance(path: &str) -> Vec<String> {
    path.split('.')
        .filter(|segment| *segment != "**")
        .map(|segment| if segment == "*" { "x" } else { segment }.to_owned())
        .collect()
}

fn steps(names: &[String]) -> Vec<Step<'_>> {
    names.iter().map(|name| Step::Key(name)).collect()
}

// The table.

#[test]
fn every_row_has_non_empty_segments_and_matches_its_own_instance() {
    for row in KEYS {
        assert!(
            row.path().split('.').all(|segment| !segment.is_empty()),
            "{}",
            row.display_path()
        );
        let names = instance(row.path());
        assert!(
            native_key(row.target(), &steps(&names)).is_some(),
            "{}",
            row.display_path()
        );
    }
}

#[test]
fn no_row_is_listed_twice() {
    let mut paths: Vec<_> = KEYS.iter().map(|row| (row.target(), row.path())).collect();
    paths.sort();
    let all = paths.len();
    paths.dedup();
    assert_eq!(paths.len(), all);
}

#[test]
fn a_key_that_is_a_floor_is_not_also_refused() {
    for row in KEYS {
        if let KeyRule::Floor(_) = row.rule() {
            let names = instance(row.path());
            assert_eq!(
                refusal(row.target(), &steps(&names)),
                None,
                "{}",
                row.display_path()
            );
        }
    }
}

#[test]
fn the_agent_keys_are_servers_and_hooks_of_each_target() {
    let keys: Vec<_> = AGENT_KEYS
        .iter()
        .map(|key| (key.target(), key.key(), key.owner()))
        .collect();
    assert_eq!(
        keys,
        [
            (TargetName::Claude, "mcpServers", "mcp.yaml"),
            (TargetName::Claude, "hooks", "hooks/hooks.yaml"),
            (TargetName::Codex, "mcp_servers", "mcp.yaml"),
            (TargetName::Codex, "hooks", "hooks/hooks.yaml"),
        ]
    );
}

#[test]
fn wildcards_stand_for_one_key_and_a_run_of_keys() {
    let path = |names: &[&str]| -> Option<&'static str> {
        let owned: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        refusal(TargetName::Codex, &steps(&owned)).map(NativeKey::path)
    };
    assert_eq!(path(&["features", "hooks"]), Some("features.hooks"));
    assert_eq!(
        path(&["profiles", "dev", "features", "hooks"]),
        Some("profiles.*.features.hooks")
    );
    assert_eq!(
        path(&["x", "y", "features", "codex_hooks"]),
        Some("**.features.codex_hooks")
    );
    assert_eq!(path(&["profiles", "approval_policy"]), None);
    assert_eq!(
        path(&["profiles", "dev", "approval_policy"]),
        Some("profiles.*.approval_policy")
    );
    assert_eq!(path(&["features", "other"]), None);
    assert_eq!(
        refusal(TargetName::Codex, &[Step::Key("features"), Step::Unnamed]),
        None
    );
}

// Boolean rows.

#[test]
fn a_boolean_floor_may_not_go_from_the_stricter_value_to_the_other() {
    assert_eq!(
        claude("sandbox.enabled", &Tree::Bool(true), &Tree::Bool(false)),
        [Looseness::Looser]
    );
    assert_eq!(
        claude("sandbox.enabled", &Tree::Bool(false), &Tree::Bool(true)),
        NOTHING
    );
    assert_eq!(
        claude("sandbox.enabled", &Tree::Bool(true), &Tree::Bool(true)),
        NOTHING
    );
    assert_eq!(
        claude("sandbox.enabled", &Tree::Bool(false), &Tree::Bool(false)),
        NOTHING
    );
}

#[test]
fn a_boolean_floor_where_false_is_stricter_goes_the_other_way() {
    assert_eq!(
        claude(
            "sandbox.autoAllowBashIfSandboxed",
            &Tree::Bool(false),
            &Tree::Bool(true)
        ),
        [Looseness::Looser]
    );
    assert_eq!(
        claude(
            "sandbox.autoAllowBashIfSandboxed",
            &Tree::Bool(true),
            &Tree::Bool(false)
        ),
        NOTHING
    );
    assert_eq!(
        codex(
            "sandbox_workspace_write.network_access",
            &Tree::Bool(false),
            &Tree::Bool(true)
        ),
        [Looseness::Looser]
    );
}

#[test]
fn a_boolean_floor_refuses_a_change_of_kind_in_either_direction() {
    for other in [Tree::Null, text("true"), Tree::Int(1), list(&["x"])] {
        assert_eq!(
            claude("sandbox.enabled", &Tree::Bool(false), &other),
            [Looseness::Kind],
            "{other:?}"
        );
    }
    assert_eq!(
        claude("sandbox.enabled", &text("yes"), &Tree::Bool(true)),
        [Looseness::Kind]
    );
}

// Ordered enum rows.

#[test]
fn an_ordered_floor_may_only_move_up_its_ranks() {
    assert_eq!(
        codex("web_search", &text("cached"), &text("live")),
        [Looseness::Looser]
    );
    assert_eq!(
        codex("web_search", &text("disabled"), &text("indexed")),
        [Looseness::Looser]
    );
    assert_eq!(
        codex("web_search", &text("live"), &text("disabled")),
        NOTHING
    );
    assert_eq!(
        codex("web_search", &text("cached"), &text("cached")),
        NOTHING
    );
}

#[test]
fn values_of_one_rank_are_the_same_strictness() {
    assert_eq!(
        codex(
            "approvals_reviewer",
            &text("auto_review"),
            &text("guardian_subagent")
        ),
        NOTHING
    );
    assert_eq!(
        codex("approvals_reviewer", &text("auto_review"), &text("user")),
        NOTHING
    );
    assert_eq!(
        codex("approvals_reviewer", &text("user"), &text("auto_review")),
        [Looseness::Looser]
    );
}

#[test]
fn a_value_outside_the_ranks_may_only_stay_as_it_was() {
    assert_eq!(codex("web_search", &text("novel"), &text("novel")), NOTHING);
    assert_eq!(
        codex("web_search", &text("novel"), &text("disabled")),
        [Looseness::Looser]
    );
    assert_eq!(
        codex("web_search", &text("cached"), &text("novel")),
        [Looseness::Looser]
    );
    assert_eq!(
        claude("disableAutoMode", &text("disable"), &text("other")),
        [Looseness::Looser]
    );
}

#[test]
fn an_ordered_floor_applies_in_profiles_too() {
    assert_eq!(
        codex("profiles.dev.web_search", &text("disabled"), &text("live")),
        [Looseness::Looser]
    );
}

// "No additions" rows.

#[test]
fn a_no_additions_list_may_not_gain_an_entry() {
    assert_eq!(
        claude(
            "sandbox.network.allowedDomains",
            &list(&["a.example"]),
            &list(&["a.example", "b.example"])
        ),
        [Looseness::Added]
    );
    assert_eq!(
        claude(
            "sandbox.network.allowedDomains",
            &list(&["a.example"]),
            &list(&["b.example"])
        ),
        [Looseness::Added]
    );
}

#[test]
fn a_no_additions_list_may_repeat_what_was_set() {
    assert_eq!(
        claude(
            "sandbox.network.allowedDomains",
            &list(&["a.example", "b.example"]),
            &list(&["b.example"])
        ),
        NOTHING
    );
    assert_eq!(
        claude("sandbox.excludedCommands", &list(&[]), &list(&[])),
        NOTHING
    );
}

#[test]
fn a_no_additions_list_refuses_a_change_of_kind() {
    for other in [Tree::Null, text("x"), Tree::Bool(true)] {
        assert_eq!(
            claude("sandbox.excludedCommands", &list(&["git"]), &other),
            [Looseness::Kind],
            "{other:?}"
        );
    }
}

#[test]
fn a_no_additions_mapping_may_not_gain_a_key_or_an_entry_below_one() {
    let held = at("*", list(&["a"]));
    assert_eq!(
        claude(
            "sandbox.ignoreViolations",
            &held,
            &at("other", list(&["a"]))
        ),
        [Looseness::Added]
    );
    assert_eq!(
        claude(
            "sandbox.ignoreViolations",
            &held,
            &at("*", list(&["a", "b"]))
        ),
        [Looseness::Added]
    );
    assert_eq!(
        claude("sandbox.ignoreViolations", &held, &at("*", text("a"))),
        [Looseness::Added]
    );
    assert_eq!(claude("sandbox.ignoreViolations", &held, &held), NOTHING);
}

// "Union" rows.

#[test]
fn a_union_list_may_grow_and_may_not_change_kind() {
    assert_eq!(
        claude("sandbox.filesystem.denyRead", &list(&["a"]), &list(&["b"])),
        NOTHING
    );
    for other in [Tree::Null, text("a"), Tree::Bool(false)] {
        assert_eq!(
            claude("sandbox.filesystem.denyRead", &list(&["a"]), &other),
            [Looseness::Kind],
            "{other:?}"
        );
    }
}

// What is free.

#[test]
fn a_key_nobody_set_before_or_set_to_null_is_free() {
    let none = loosenings(
        TargetName::Claude,
        &Tree::Map(BTreeMap::new()),
        &at("sandbox.enabled", Tree::Bool(false)),
    );
    assert_eq!(none.len(), 0);
    assert_eq!(
        claude("sandbox.enabled", &Tree::Null, &Tree::Bool(false)),
        NOTHING
    );
}

#[test]
fn a_key_that_is_not_a_floor_may_change_freely() {
    assert_eq!(claude("theme", &text("dark"), &Tree::Bool(true)), NOTHING);
    assert_eq!(claude("sandbox.other", &text("a"), &text("b")), NOTHING);
}

#[test]
fn every_floor_row_is_checked_by_its_strictness() {
    // A looser or differently-kinded value for each row is refused.
    for row in KEYS {
        let KeyRule::Floor(strictness) = row.rule() else {
            continue;
        };
        if row.path().contains('*') {
            continue;
        }
        let (held, looser) = match strictness {
            Strictness::Flag(stricter) => (Tree::Bool(stricter), Tree::Bool(!stricter)),
            Strictness::Ladder(ranks) => {
                let strictest = ranks.last().and_then(|rank| rank.first()).copied();
                let loosest = ranks.first().and_then(|rank| rank.first()).copied();
                match (strictest, loosest) {
                    (Some(strict), Some(loose)) if ranks.len() > 1 => (text(strict), text(loose)),
                    _ => (text("disable"), text("other")),
                }
            }
            Strictness::NoAdditions => (list(&["held"]), list(&["held", "added"])),
            Strictness::Union => (list(&["held"]), text("scalar")),
        };
        let found = loosenings(row.target(), &at(row.path(), held), &at(row.path(), looser));
        assert_eq!(found.len(), 1, "{}", row.display_path());
        assert_eq!(found[0].key().path(), row.path());
    }
}

#[test]
fn the_error_names_the_layer_and_file_and_the_key_path_without_a_value() {
    use crate::diagnostic::Where;
    use crate::untrusted_text::Stripped;
    let found = loosenings(
        TargetName::Claude,
        &at("sandbox.enabled", Tree::Bool(true)),
        &at("sandbox.enabled", Tree::Bool(false)),
    );
    let at = Where::layer_file(
        Stripped::from_untrusted("repo"),
        Stripped::from_untrusted("settings.yaml"),
    );
    assert_eq!(
        found[0].diagnostic(&at).to_string(),
        "error [settings] repo:settings.yaml: 'claude.sandbox.enabled' is a floor: this value is looser than the one a more general layer set"
    );
}
