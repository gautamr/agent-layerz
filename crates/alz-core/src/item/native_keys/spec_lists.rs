//! The lists of spec §7.9 (security-relevant native keys, first-class keys, reserved keys, floor
//! keys), written out here key by key, so that the key rules of the descriptors cannot lose or gain a
//! row, or change a row's rule kind or reason, without a test failing.

use std::collections::BTreeSet;

use super::{KeyRule, NativeKey, Strictness};
use crate::target::Registry;

const CLAUDE: &str = "claude";
const CODEX: &str = "codex";

/// A rule as the spec words it, which the rows of the descriptors are compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    FirstClass(&'static str),
    Reserved(&'static str),
    Flag(bool),
    Ladder(&'static [&'static [&'static str]]),
    NoAdditions,
    Union,
}

/// A rule as a row of a descriptor has it, owned, so that it can be compared with a [`Rule`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    FirstClass(String),
    Reserved(String),
    Flag(bool),
    Ladder(Vec<Vec<String>>),
    NoAdditions,
    Union,
}

impl Rule {
    fn seen(self) -> Seen {
        match self {
            Self::FirstClass(owner) => Seen::FirstClass(owner.to_owned()),
            Self::Reserved(reason) => Seen::Reserved(reason.to_owned()),
            Self::Flag(stricter) => Seen::Flag(stricter),
            Self::Ladder(ranks) => Seen::Ladder(
                ranks
                    .iter()
                    .map(|rank| rank.iter().map(|value| (*value).to_owned()).collect())
                    .collect(),
            ),
            Self::NoAdditions => Seen::NoAdditions,
            Self::Union => Seen::Union,
        }
    }
}

fn seen(rule: KeyRule<'_>) -> Seen {
    match rule {
        KeyRule::FirstClass(owner) => Seen::FirstClass(owner.to_owned()),
        KeyRule::Reserved(reason) => Seen::Reserved(reason.to_owned()),
        KeyRule::Floor(Strictness::Flag(stricter)) => Seen::Flag(stricter),
        KeyRule::Floor(Strictness::Ladder(ranks)) => Seen::Ladder(ranks.to_vec()),
        KeyRule::Floor(Strictness::NoAdditions) => Seen::NoAdditions,
        KeyRule::Floor(Strictness::Union) => Seen::Union,
    }
}

/// The rows of the descriptors of Claude and then Codex, whose key rules the registry hands out.
pub(super) fn descriptor_rows(registry: &Registry) -> Vec<NativeKey<'_>> {
    [crate::target::claude(), crate::target::codex()]
        .into_iter()
        .filter_map(|target| registry.key_rules(target))
        .flat_map(|rules| rules.rows())
        .collect()
}

type Row = (&'static str, &'static str, Rule);

const OUTSIDE: Rule = Rule::Reserved("it is read from a user or managed scope, not from a project");
const COMMAND: Rule = Rule::Reserved("it runs a command outside hooks.yaml and its locks");
const PLUGIN: Rule =
    Rule::Reserved("it brings hooks and servers past the allowlists of mcp.yaml and hooks.yaml");
const TRAFFIC: Rule =
    Rule::Reserved("it runs a program or routes traffic, and is not a project setting");
const NO_ORDER: Rule = Rule::Reserved("its values have no order, so no layer may tighten it");
const PORT: Rule = Rule::Reserved("it routes traffic, and has no order");
const LOWERS_A_CHECK: Rule =
    Rule::Reserved("it turns off a safety check of Claude Code, which no layer may lower");
const SIGN_IN: Rule =
    Rule::Reserved("it steers how the user signs in, which a repository layer may not set");

const TRUE: Rule = Rule::Flag(true);
const FALSE: Rule = Rule::Flag(false);
const NO_ADDITIONS: Rule = Rule::NoAdditions;
const UNION: Rule = Rule::Union;

const MCP_ENABLE: Rule = Rule::FirstClass("mcp.enable in the repo manifest");
const HOOKS: Rule = Rule::FirstClass("hooks/hooks.yaml");
const PERMISSIONS: Rule = Rule::FirstClass("permissions.yaml");

/// "First-class keys" of §7.9, in the spec's order.
const FIRST_CLASS: &[Row] = &[
    (CLAUDE, "permissions", PERMISSIONS),
    (CLAUDE, "hooks", HOOKS),
    (CLAUDE, "model", Rule::FirstClass("models.yaml (default)")),
    (CLAUDE, "enabledMcpjsonServers", MCP_ENABLE),
    (CLAUDE, "disabledMcpjsonServers", MCP_ENABLE),
    (CLAUDE, "enableAllProjectMcpServers", MCP_ENABLE),
    (
        CLAUDE,
        "disableAllHooks",
        Rule::FirstClass("hooks.yaml `disabled` (locked hooks cannot be switched off)"),
    ),
    (CODEX, "model", Rule::FirstClass("models.yaml")),
    (CODEX, "mcp_servers", Rule::FirstClass("mcp.yaml")),
    (CODEX, "hooks", HOOKS),
    (CODEX, "approval_policy", PERMISSIONS),
    (CODEX, "sandbox_mode", PERMISSIONS),
    (CODEX, "features.hooks", HOOKS),
    (CODEX, "features.codex_hooks", HOOKS),
    (CODEX, "**.features.hooks", HOOKS),
    (CODEX, "**.features.codex_hooks", HOOKS),
];

/// "Reserved keys" of §7.9: each bullet of the spec, with the reason that bullet gives.
const RESERVED: &[Row] = &[
    // A key that runs a command or carries a login.
    (
        CLAUDE,
        "apiKeyHelper",
        Rule::Reserved("it runs a command and carries a login"),
    ),
    (CLAUDE, "statusLine", COMMAND),
    (CLAUDE, "fileSuggestion", COMMAND),
    (CLAUDE, "awsAuthRefresh", COMMAND),
    (CLAUDE, "awsCredentialExport", COMMAND),
    (CLAUDE, "gcpAuthRefresh", COMMAND),
    (CLAUDE, "otelHeadersHelper", COMMAND),
    (
        CLAUDE,
        "sandbox.credentials",
        Rule::Reserved("it carries or pins a login"),
    ),
    // `mcpServers`.
    (
        CLAUDE,
        "mcpServers",
        Rule::Reserved(
            "Claude Code documents no such settings key; servers are set through mcp.yaml",
        ),
    ),
    // A key that routes traffic.
    (CLAUDE, "sandbox.network.httpProxyPort", PORT),
    (CLAUDE, "sandbox.network.socksProxyPort", PORT),
    // A plugin source.
    (CLAUDE, "enabledPlugins", PLUGIN),
    (CLAUDE, "extraKnownMarketplaces", PLUGIN),
    // A key for a scope alz does not write.
    (CLAUDE, "sandbox.filesystem.disabled", OUTSIDE),
    (CLAUDE, "sandbox.allowAppleEvents", OUTSIDE),
    (CLAUDE, "sandbox.allowNestedUserNamespaces", OUTSIDE),
    (CLAUDE, "sandbox.network.strictAllowlist", OUTSIDE),
    (CLAUDE, "sandbox.network.allowManagedDomainsOnly", OUTSIDE),
    (CLAUDE, "autoMode", OUTSIDE),
    (CLAUDE, "skipAutoPermissionPrompt", OUTSIDE),
    (CLAUDE, "skipDangerousModePermissionPrompt", OUTSIDE),
    (CLAUDE, "allowManagedHooksOnly", OUTSIDE),
    (CLAUDE, "allowManagedMcpServersOnly", OUTSIDE),
    (CLAUDE, "managedMcpServers", OUTSIDE),
    (CLAUDE, "prependPlugins", OUTSIDE),
    (CLAUDE, "appendPlugins", OUTSIDE),
    (CLAUDE, "pluginConfigs", OUTSIDE),
    (CLAUDE, "strictKnownMarketplaces", OUTSIDE),
    (CLAUDE, "blockedMarketplaces", OUTSIDE),
    (CLAUDE, "strictPluginOnlyCustomization", OUTSIDE),
    (CLAUDE, "syncClaudeAiPlugins", OUTSIDE),
    (CLAUDE, "allowAllClaudeAiMcps", OUTSIDE),
    (
        CLAUDE,
        "sandbox.filesystem.allowManagedReadPathsOnly",
        OUTSIDE,
    ),
    (CLAUDE, "sandbox.ripgrep", OUTSIDE),
    (CLAUDE, "sandbox.bwrapPath", OUTSIDE),
    (CLAUDE, "sandbox.socatPath", OUTSIDE),
    (CLAUDE, "sandbox.network.tlsTerminate", OUTSIDE),
    (CLAUDE, "processWrapper", OUTSIDE),
    (CLAUDE, "policyHelper", OUTSIDE),
    (CLAUDE, "sshHostCommands", OUTSIDE),
    (CLAUDE, "forceLoginGatewayUrl", OUTSIDE),
    (CLAUDE, "allowManagedPermissionRulesOnly", OUTSIDE),
    (CLAUDE, "disableSideloadFlags", OUTSIDE),
    // TASK-79.
    (CLAUDE, "skipWebFetchPreflight", LOWERS_A_CHECK),
    (CLAUDE, "forceLoginMethod", SIGN_IN),
    (CLAUDE, "forceLoginOrgUUID", SIGN_IN),
    // Codex.
    (CODEX, "auto_review", NO_ORDER),
    (CODEX, "default_permissions", NO_ORDER),
    (CODEX, "permissions", NO_ORDER),
    (CODEX, "apps", NO_ORDER),
    (
        CODEX,
        "profile",
        Rule::Reserved("removed by Codex 0.169.0 (use --profile)"),
    ),
    (
        CODEX,
        "profiles",
        Rule::Reserved("Codex ignores it in a project's config.toml"),
    ),
    (CODEX, "windows", OUTSIDE),
    (CODEX, "plugins", PLUGIN),
    (CODEX, "marketplaces", PLUGIN),
    (
        CODEX,
        "projects.*.trust_level",
        Rule::Reserved("a layer must not mark a project as trusted"),
    ),
    (CODEX, "notify", TRAFFIC),
    (CODEX, "model_provider", TRAFFIC),
    (CODEX, "model_providers", TRAFFIC),
    (CODEX, "openai_base_url", TRAFFIC),
    (CODEX, "chatgpt_base_url", TRAFFIC),
    (CODEX, "otel", TRAFFIC),
];

const DISABLE_AUTO_MODE: Rule = Rule::Ladder(&[&["disable"]]);
const APPROVALS_REVIEWER: Rule = Rule::Ladder(&[&["auto_review", "guardian_subagent"], &["user"]]);
const WEB_SEARCH: Rule = Rule::Ladder(&[&["live"], &["indexed"], &["cached"], &["disabled"]]);

/// "Floor keys" of §7.9: booleans, ordered text, "no additions" and "union" lists.
const FLOORS: &[Row] = &[
    (CLAUDE, "sandbox.enabled", TRUE),
    (CLAUDE, "sandbox.failIfUnavailable", TRUE),
    (CLAUDE, "disableSkillShellExecution", TRUE),
    (CLAUDE, "disableClaudeAiConnectors", TRUE),
    (
        CODEX,
        "sandbox_workspace_write.exclude_tmpdir_env_var",
        TRUE,
    ),
    (CODEX, "sandbox_workspace_write.exclude_slash_tmp", TRUE),
    (CLAUDE, "sandbox.autoAllowBashIfSandboxed", FALSE),
    (CLAUDE, "sandbox.allowUnsandboxedCommands", FALSE),
    (CLAUDE, "sandbox.enableWeakerNestedSandbox", FALSE),
    (CLAUDE, "sandbox.enableWeakerNetworkIsolation", FALSE),
    (CLAUDE, "sandbox.network.allowAllUnixSockets", FALSE),
    (CLAUDE, "sandbox.network.allowLocalBinding", FALSE),
    (CODEX, "sandbox_workspace_write.network_access", FALSE),
    (CODEX, "allow_login_shell", FALSE),
    (CLAUDE, "disableAutoMode", DISABLE_AUTO_MODE),
    (CODEX, "approvals_reviewer", APPROVALS_REVIEWER),
    (CODEX, "web_search", WEB_SEARCH),
    (CLAUDE, "sandbox.excludedCommands", NO_ADDITIONS),
    (CLAUDE, "sandbox.filesystem.allowWrite", NO_ADDITIONS),
    (CLAUDE, "sandbox.filesystem.allowRead", NO_ADDITIONS),
    (CLAUDE, "sandbox.ignoreViolations", NO_ADDITIONS),
    (CLAUDE, "sandbox.network.allowedDomains", NO_ADDITIONS),
    (CLAUDE, "sandbox.network.allowUnixSockets", NO_ADDITIONS),
    (CLAUDE, "sandbox.network.allowMachLookup", NO_ADDITIONS),
    (CLAUDE, "allowedHttpHookUrls", NO_ADDITIONS),
    (CLAUDE, "httpHookAllowedEnvVars", NO_ADDITIONS),
    (CLAUDE, "allowedMcpServers", NO_ADDITIONS),
    (
        CODEX,
        "sandbox_workspace_write.writable_roots",
        NO_ADDITIONS,
    ),
    (CODEX, "tools.web_search.allowed_domains", NO_ADDITIONS),
    (CLAUDE, "sandbox.filesystem.denyWrite", UNION),
    (CLAUDE, "sandbox.filesystem.denyRead", UNION),
    (CLAUDE, "sandbox.network.deniedDomains", UNION),
    (CLAUDE, "deniedMcpServers", UNION),
];

fn spec_rows() -> impl Iterator<Item = &'static Row> {
    FIRST_CLASS.iter().chain(RESERVED).chain(FLOORS)
}

/// §7.9 Security-relevant native keys, First-class keys, Reserved keys, Floor keys: the key rules
/// of the descriptors have each key the spec lists, with the rule kind and the reason the spec
/// gives, and no other row.
#[test]
fn the_table_holds_every_key_the_spec_lists_with_its_rule_and_no_other_row() {
    let registry = Registry::builtin();
    let rows = descriptor_rows(&registry);
    let spec_keys: BTreeSet<_> = spec_rows()
        .map(|(target, path, _)| (*target, *path))
        .collect();
    assert_eq!(
        spec_keys.len(),
        spec_rows().count(),
        "a key is listed twice in the spec lists of this test"
    );
    for (target, path, rule) in spec_rows() {
        let found: Vec<_> = rows
            .iter()
            .filter(|row| row.target().as_str() == *target && row.path() == *path)
            .map(|row| seen(row.rule()))
            .collect();
        assert_eq!(found, [rule.seen()], "{target}.{path}");
    }
    let extra: Vec<_> = rows
        .iter()
        .filter(|row| !spec_keys.contains(&(row.target().as_str(), row.path())))
        .map(NativeKey::display_path)
        .collect();
    assert_eq!(
        extra,
        Vec::<String>::new(),
        "rows that the spec does not list"
    );
    assert_eq!(rows.len(), spec_keys.len());
}

/// §7.9 Reserved keys, First-class keys, Floor keys: a key is exactly one of the three kinds, so
/// the three lists of the spec do not overlap and each holds only rows of its own kind.
#[test]
fn a_key_is_first_class_or_reserved_or_a_floor_never_two_of_them() {
    let keys = |rows: &'static [Row]| -> BTreeSet<(&'static str, &'static str)> {
        rows.iter()
            .map(|(target, path, _)| (*target, *path))
            .collect()
    };
    let (first, reserved, floors) = (keys(FIRST_CLASS), keys(RESERVED), keys(FLOORS));
    assert!(first.is_disjoint(&reserved));
    assert!(first.is_disjoint(&floors));
    assert!(reserved.is_disjoint(&floors));
    assert!(
        FIRST_CLASS
            .iter()
            .all(|(_, _, rule)| matches!(rule, Rule::FirstClass(_)))
    );
    assert!(
        RESERVED
            .iter()
            .all(|(_, _, rule)| matches!(rule, Rule::Reserved(_)))
    );
    assert!(
        FLOORS
            .iter()
            .all(|(_, _, rule)| !matches!(rule, Rule::FirstClass(_) | Rule::Reserved(_)))
    );
}
