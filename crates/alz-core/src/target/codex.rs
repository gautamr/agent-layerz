//! The native vocabulary of Codex (spec §7.9 **Security-relevant native keys**, §7.2 **Native
//! agent fields**): the key rules of its settings and the keys of its agent map, as a
//! descriptor-only entry: Codex has no adapter yet (§8 **Targets without an adapter**), but its
//! `settings.yaml` section is validated now, so its vocabulary lives here until its adapter
//! exists, and the adapter's descriptor takes it over (TASK-126). The registry hands these rules
//! out for `codex` when no adapter is registered for it.
//!
//! The rows are the owner's approved list (TASK-55.5, 2026-11-23). TASK-76 corrects it. Each list
//! is in the order of §7.9, which is the order the lookup takes.

use alz_adapter::{FirstClassKey, FloorKey, FloorRule, NativeVocabulary, ReservedKey};

const IGNORED_IN_A_PROJECT: &str = "Codex ignores it in a project's config.toml";
const NO_ORDER: &str = "its values have no order, so no layer may tighten it";
const PLUGIN_SOURCE: &str =
    "it brings hooks and servers past the allowlists of mcp.yaml and hooks.yaml";
const ROUTES_TRAFFIC: &str = "it runs a program or routes traffic, and is not a project setting";
const OUTSIDE_THE_PROJECT: &str = "it is read from a user or managed scope, not from a project";

fn first_class(through: &str, paths: &[&str]) -> Vec<FirstClassKey> {
    paths
        .iter()
        .map(|path| FirstClassKey::new(*path, through))
        .collect()
}

fn floor(rule: &FloorRule, path: &str) -> FloorKey {
    FloorKey::new(path, rule.clone())
}

fn reserved(reason: &str, paths: &[&str]) -> Vec<ReservedKey> {
    paths
        .iter()
        .map(|path| ReservedKey::new(*path, reason))
        .collect()
}

fn ranks(ranks: &[&[&str]]) -> FloorRule {
    FloorRule::Ranked(
        ranks
            .iter()
            .map(|rank| rank.iter().map(|value| (*value).to_owned()).collect())
            .collect(),
    )
}

fn first_class_keys() -> Vec<FirstClassKey> {
    [
        first_class("models.yaml", &["model"]),
        first_class("mcp.yaml", &["mcp_servers"]),
        first_class("hooks/hooks.yaml", &["hooks"]),
        first_class("permissions.yaml", &["approval_policy", "sandbox_mode"]),
        first_class(
            "hooks/hooks.yaml",
            &[
                "features.hooks",
                "features.codex_hooks",
                "**.features.hooks",
                "**.features.codex_hooks",
            ],
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn floor_keys() -> Vec<FloorKey> {
    vec![
        floor(
            &FloorRule::FalseIsStricter,
            "sandbox_workspace_write.network_access",
        ),
        floor(
            &FloorRule::NoAdditions,
            "sandbox_workspace_write.writable_roots",
        ),
        floor(
            &FloorRule::TrueIsStricter,
            "sandbox_workspace_write.exclude_tmpdir_env_var",
        ),
        floor(
            &FloorRule::TrueIsStricter,
            "sandbox_workspace_write.exclude_slash_tmp",
        ),
        floor(
            &ranks(&[&["auto_review", "guardian_subagent"], &["user"]]),
            "approvals_reviewer",
        ),
        floor(&FloorRule::FalseIsStricter, "allow_login_shell"),
        floor(
            &ranks(&[&["live"], &["indexed"], &["cached"], &["disabled"]]),
            "web_search",
        ),
        floor(&FloorRule::NoAdditions, "tools.web_search.allowed_domains"),
    ]
}

fn reserved_keys() -> Vec<ReservedKey> {
    [
        reserved("removed by Codex 0.169.0 (use --profile)", &["profile"]),
        reserved(IGNORED_IN_A_PROJECT, &["profiles"]),
        reserved(
            NO_ORDER,
            &["auto_review", "default_permissions", "permissions"],
        ),
        reserved(
            "a layer must not mark a project as trusted",
            &["projects.*.trust_level"],
        ),
        reserved(PLUGIN_SOURCE, &["plugins", "marketplaces"]),
        reserved(NO_ORDER, &["apps"]),
        reserved(OUTSIDE_THE_PROJECT, &["windows"]),
        reserved(
            ROUTES_TRAFFIC,
            &[
                "notify",
                "model_provider",
                "model_providers",
                "openai_base_url",
                "chatgpt_base_url",
                "otel",
            ],
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The keys of an agent's `codex:` map that have a neutral field, and so are refused inside it
/// (§7.2).
const AGENT_NEUTRAL_KEYS: [&str; 4] = ["name", "description", "model", "developer_instructions"];

/// The keys of an agent's `codex:` map that bring servers and hooks past `mcp.yaml` and
/// `hooks.yaml` (§7.2).
fn agent_first_class_keys() -> Vec<FirstClassKey> {
    vec![
        FirstClassKey::new("mcp_servers", "mcp.yaml"),
        FirstClassKey::new("hooks", "hooks/hooks.yaml"),
    ]
}

/// The native vocabulary of Codex: key rules and the keys of its agent map, and no typed keys,
/// until its adapter states the rest.
pub(super) fn vocabulary() -> NativeVocabulary {
    NativeVocabulary::default()
        .with_key_rules(first_class_keys(), reserved_keys(), floor_keys())
        .with_agent_native(
            AGENT_NEUTRAL_KEYS
                .iter()
                .map(|key| (*key).to_owned())
                .collect(),
            agent_first_class_keys(),
            None,
        )
}
