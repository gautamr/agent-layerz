use std::collections::BTreeMap;

use crate::merge::Tree;
use alz_adapter::TargetId;

use super::spec_lists::descriptor_rows;
use super::{KeyRule, KeyRules, Looseness, Step, Strictness};
use crate::target::Registry;

/// The rules of `target` in the built-in registry: Claude's adapter states them, and Codex has a
/// descriptor-only entry.
fn rules<'a>(registry: &'a Registry, target: &TargetId) -> KeyRules<'a> {
    registry
        .key_rules(target)
        .expect("claude and codex have key rules")
}

/// The key path of the row of `target` that refuses the key at `names`.
fn refused_path(target: &TargetId, names: &[&str]) -> Option<String> {
    let registry = Registry::builtin();
    let owned: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    rules(&registry, target)
        .refusal(&steps(&owned))
        .map(|row| row.path().to_owned())
}

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
    loosened(crate::target::claude(), path, before, after)
}

fn codex(path: &str, before: &Tree, after: &Tree) -> Vec<Looseness> {
    loosened(crate::target::codex(), path, before, after)
}

fn loosened(target: &TargetId, path: &str, before: &Tree, after: &Tree) -> Vec<Looseness> {
    let registry = Registry::builtin();
    rules(&registry, target)
        .loosenings(&at(path, before.clone()), &at(path, after.clone()))
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
    let registry = Registry::builtin();
    for row in descriptor_rows(&registry) {
        assert!(
            row.path().split('.').all(|segment| !segment.is_empty()),
            "{}",
            row.display_path()
        );
        let names = instance(row.path());
        assert!(
            rules(&registry, row.target())
                .native_key(&steps(&names))
                .is_some(),
            "{}",
            row.display_path()
        );
    }
}

#[test]
fn no_row_is_listed_twice() {
    let registry = Registry::builtin();
    let rows = descriptor_rows(&registry);
    let mut paths: Vec<_> = rows.iter().map(|row| (row.target(), row.path())).collect();
    paths.sort_unstable();
    let all = paths.len();
    paths.dedup();
    assert_eq!(paths.len(), all);
}

#[test]
fn a_key_that_is_a_floor_is_not_also_refused() {
    let registry = Registry::builtin();
    for row in descriptor_rows(&registry) {
        if let KeyRule::Floor(_) = row.rule() {
            let names = instance(row.path());
            assert_eq!(
                rules(&registry, row.target()).refusal(&steps(&names)),
                None,
                "{}",
                row.display_path()
            );
        }
    }
}

#[test]
fn the_agent_keys_are_servers_and_hooks_of_each_target() {
    let registry = Registry::builtin();
    let keys: Vec<_> = [crate::target::claude(), crate::target::codex()]
        .into_iter()
        .flat_map(|target| {
            rules(&registry, target)
                .vocabulary()
                .agent_first_class_keys
                .iter()
                .map(|key| (target.as_str(), key.path.as_str(), key.set_through.as_str()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        keys,
        [
            ("claude", "mcpServers", "mcp.yaml"),
            ("claude", "hooks", "hooks/hooks.yaml"),
            ("codex", "mcp_servers", "mcp.yaml"),
            ("codex", "hooks", "hooks/hooks.yaml"),
        ]
    );
}

#[test]
fn wildcards_stand_for_one_key_and_a_run_of_keys() {
    let path = |names: &[&str]| refused_path(crate::target::codex(), names);
    assert_eq!(
        path(&["features", "hooks"]).as_deref(),
        Some("features.hooks")
    );
    assert_eq!(
        path(&["profiles", "dev", "features", "hooks"]).as_deref(),
        Some("**.features.hooks")
    );
    assert_eq!(
        path(&["x", "y", "features", "codex_hooks"]).as_deref(),
        Some("**.features.codex_hooks")
    );
    assert_eq!(path(&["projects", "trust_level"]), None);
    assert_eq!(
        path(&["projects", "/work/shop", "trust_level"]).as_deref(),
        Some("projects.*.trust_level")
    );
    assert_eq!(path(&["features", "other"]), None);
    let registry = Registry::builtin();
    assert_eq!(
        rules(&registry, crate::target::codex()).refusal(&[Step::Key("features"), Step::Unnamed]),
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
fn codex_web_search_is_a_ladder_only_at_the_top_level() {
    // `tools.web_search` is a boolean or a table, never a mode, so it is not a ladder.
    for after in [Tree::Bool(true), text("live"), list(&["x"])] {
        assert_eq!(
            codex("tools.web_search", &Tree::Bool(false), &after),
            NOTHING,
            "{after:?}"
        );
    }
    assert_eq!(
        codex(
            "tools.web_search",
            &Tree::Map([("context_size".to_owned(), text("low"))].into()),
            &Tree::Map([("context_size".to_owned(), text("high"))].into())
        ),
        NOTHING
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
fn a_web_search_domain_filter_may_not_gain_a_domain() {
    assert_eq!(
        codex(
            "tools.web_search.allowed_domains",
            &list(&["a.example"]),
            &list(&["a.example", "b.example"])
        ),
        [Looseness::Added]
    );
    assert_eq!(
        codex(
            "tools.web_search.allowed_domains",
            &list(&["a.example", "b.example"]),
            &list(&["a.example"])
        ),
        NOTHING
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
    let registry = Registry::builtin();
    let none = rules(&registry, crate::target::claude()).loosenings(
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
    let registry = Registry::builtin();
    for row in descriptor_rows(&registry) {
        let KeyRule::Floor(strictness) = row.rule() else {
            continue;
        };
        if row.path().contains('*') {
            continue;
        }
        let (held, looser) = match strictness {
            Strictness::Flag(stricter) => (Tree::Bool(stricter), Tree::Bool(!stricter)),
            Strictness::Ladder(ranks) => {
                let strictest = ranks.last().and_then(|rank| rank.first());
                let loosest = ranks.first().and_then(|rank| rank.first());
                match (strictest, loosest) {
                    (Some(strict), Some(loose)) if ranks.len() > 1 => (text(strict), text(loose)),
                    _ => (text("disable"), text("other")),
                }
            }
            Strictness::NoAdditions => (list(&["held"]), list(&["held", "added"])),
            Strictness::Union => (list(&["held"]), text("scalar")),
        };
        let found = rules(&registry, row.target())
            .loosenings(&at(row.path(), held), &at(row.path(), looser));
        assert_eq!(found.len(), 1, "{}", row.display_path());
        assert_eq!(found[0].key().path(), row.path());
    }
}

#[test]
fn the_error_names_the_layer_and_file_and_the_key_path_without_a_value() {
    use crate::diagnostic::Where;
    use crate::untrusted_text::Stripped;
    let registry = Registry::builtin();
    let found = rules(&registry, crate::target::claude()).loosenings(
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

// The refusals that TASK-76 corrected.

fn reserved_reason(target: &TargetId, names: &[&str]) -> Option<String> {
    let registry = Registry::builtin();
    let owned: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    match rules(&registry, target)
        .refusal(&steps(&owned))
        .map(|row| row.rule())
    {
        Some(KeyRule::Reserved(reason)) => Some(reason.to_owned()),
        Some(KeyRule::FirstClass(_) | KeyRule::Floor(_)) | None => None,
    }
}

#[test]
fn a_key_that_only_a_user_or_managed_scope_reads_is_reserved_for_that_reason() {
    let outside = "it is read from a user or managed scope, not from a project";
    for key in [
        "allowAllClaudeAiMcps",
        "sandbox.filesystem.allowManagedReadPathsOnly",
        "sandbox.ripgrep",
        "sandbox.bwrapPath",
        "sandbox.socatPath",
        "sandbox.network.tlsTerminate",
        "processWrapper",
        "policyHelper",
        "sshHostCommands",
        "forceLoginGatewayUrl",
        "allowManagedPermissionRulesOnly",
        "disableSideloadFlags",
    ] {
        let names: Vec<&str> = key.split('.').collect();
        assert_eq!(
            reserved_reason(crate::target::claude(), &names),
            Some(outside.to_owned()),
            "{key}"
        );
    }
    assert_eq!(
        reserved_reason(crate::target::codex(), &["windows"]),
        Some(outside.to_owned())
    );
}

#[test]
fn a_claude_key_that_runs_a_command_outside_hooks_is_reserved() {
    for key in [
        "awsAuthRefresh",
        "awsCredentialExport",
        "gcpAuthRefresh",
        "otelHeadersHelper",
        "statusLine",
        "fileSuggestion",
    ] {
        assert_eq!(
            reserved_reason(crate::target::claude(), &[key]),
            Some("it runs a command outside hooks.yaml and its locks".to_owned()),
            "{key}"
        );
    }
}

#[test]
fn claude_mcp_servers_is_refused_without_calling_it_a_key_claude_reads() {
    let reason = reserved_reason(crate::target::claude(), &["mcpServers"]).expect("a reserved key");
    assert!(
        reason.contains("documents no such settings key"),
        "{reason}"
    );
}

#[test]
fn codex_profile_is_refused_as_removed_and_profiles_as_ignored_in_a_project() {
    assert_eq!(
        reserved_reason(crate::target::codex(), &["profile"]),
        Some("removed by Codex 0.169.0 (use --profile)".to_owned())
    );
    assert_eq!(
        reserved_reason(crate::target::codex(), &["profiles"]),
        Some("Codex ignores it in a project's config.toml".to_owned())
    );
}

#[test]
fn a_profile_is_refused_as_a_whole_so_no_row_below_it_is_reachable() {
    let registry = Registry::builtin();
    for row in descriptor_rows(&registry) {
        assert!(
            !(row.target() == crate::target::codex() && row.path().starts_with("profiles.")),
            "{}",
            row.display_path()
        );
    }
}
