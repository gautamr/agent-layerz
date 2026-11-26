//! The key rules of Claude's native settings (spec §7.9 **Security-relevant native keys**): the
//! first-class, reserved and floor keys that the Claude descriptor states. The core applies them
//! (`item::native_keys`); they are Claude's facts, not the core's.
//!
//! The rows are the owner's approved list (TASK-55.5, 2026-11-23). TASK-76 corrects it. Each list
//! is in the order of §7.9, which is the order the lookup takes.

use alz_adapter::{FirstClassKey, FloorKey, FloorRule, ReservedKey};

/// Keys set in a user, local or managed scope only, which Claude ignores in a project.
const OUTSIDE_THE_PROJECT: &str = "it is read from a user or managed scope, not from a project";
const RUNS_A_COMMAND: &str = "it runs a command outside hooks.yaml and its locks";
const PLUGIN_SOURCE: &str =
    "it brings hooks and servers past the allowlists of mcp.yaml and hooks.yaml";
/// Keys that lower a safety check of the target.
const LOWERS_A_CHECK: &str = "it turns off a safety check of Claude Code, which no layer may lower";
/// Keys that steer how the user signs in.
const STEERS_SIGN_IN: &str =
    "it steers how the user signs in, which a repository layer may not set";
const ROUTES_TRAFFIC_NO_ORDER: &str = "it routes traffic, and has no order";

const MCP_ENABLE: &str = "mcp.enable in the repo manifest";

fn floors(rule: &FloorRule, paths: &[&str]) -> Vec<FloorKey> {
    paths
        .iter()
        .map(|path| FloorKey::new(*path, rule.clone()))
        .collect()
}

fn reserved(reason: &str, paths: &[&str]) -> Vec<ReservedKey> {
    paths
        .iter()
        .map(|path| ReservedKey::new(*path, reason))
        .collect()
}

/// The first-class keys of Claude's settings.
pub(super) fn first_class_keys() -> Vec<FirstClassKey> {
    vec![
        FirstClassKey::new("permissions", "permissions.yaml"),
        FirstClassKey::new("hooks", "hooks/hooks.yaml"),
        FirstClassKey::new("model", "models.yaml (default)"),
        FirstClassKey::new("enabledMcpjsonServers", MCP_ENABLE),
        FirstClassKey::new("disabledMcpjsonServers", MCP_ENABLE),
        FirstClassKey::new("enableAllProjectMcpServers", MCP_ENABLE),
        FirstClassKey::new(
            "disableAllHooks",
            "hooks.yaml `disabled` (locked hooks cannot be switched off)",
        ),
    ]
}

/// The floor keys of Claude's settings.
pub(super) fn floor_keys() -> Vec<FloorKey> {
    let rows = [
        floors(
            &FloorRule::TrueIsStricter,
            &["sandbox.enabled", "sandbox.failIfUnavailable"],
        ),
        floors(
            &FloorRule::FalseIsStricter,
            &[
                "sandbox.autoAllowBashIfSandboxed",
                "sandbox.allowUnsandboxedCommands",
            ],
        ),
        floors(
            &FloorRule::NoAdditions,
            &[
                "sandbox.excludedCommands",
                "sandbox.filesystem.allowWrite",
                "sandbox.filesystem.allowRead",
            ],
        ),
        floors(
            &FloorRule::Union,
            &[
                "sandbox.filesystem.denyWrite",
                "sandbox.filesystem.denyRead",
            ],
        ),
        floors(&FloorRule::NoAdditions, &["sandbox.ignoreViolations"]),
        floors(
            &FloorRule::FalseIsStricter,
            &[
                "sandbox.enableWeakerNestedSandbox",
                "sandbox.enableWeakerNetworkIsolation",
            ],
        ),
        floors(
            &FloorRule::NoAdditions,
            &[
                "sandbox.network.allowedDomains",
                "sandbox.network.allowUnixSockets",
                "sandbox.network.allowMachLookup",
            ],
        ),
        floors(
            &FloorRule::FalseIsStricter,
            &[
                "sandbox.network.allowAllUnixSockets",
                "sandbox.network.allowLocalBinding",
            ],
        ),
        floors(&FloorRule::Union, &["sandbox.network.deniedDomains"]),
        floors(
            &FloorRule::Ranked(vec![vec!["disable".to_owned()]]),
            &["disableAutoMode"],
        ),
        floors(
            &FloorRule::NoAdditions,
            &["allowedHttpHookUrls", "httpHookAllowedEnvVars"],
        ),
        floors(&FloorRule::TrueIsStricter, &["disableSkillShellExecution"]),
        floors(&FloorRule::NoAdditions, &["allowedMcpServers"]),
        floors(&FloorRule::Union, &["deniedMcpServers"]),
        floors(&FloorRule::TrueIsStricter, &["disableClaudeAiConnectors"]),
    ];
    rows.into_iter().flatten().collect()
}

/// The reserved keys of Claude's settings.
pub(super) fn reserved_keys() -> Vec<ReservedKey> {
    let rows = [
        reserved(
            "Claude Code documents no such settings key; servers are set through mcp.yaml",
            &["mcpServers"],
        ),
        reserved(
            ROUTES_TRAFFIC_NO_ORDER,
            &[
                "sandbox.network.httpProxyPort",
                "sandbox.network.socksProxyPort",
            ],
        ),
        reserved(
            OUTSIDE_THE_PROJECT,
            &[
                "sandbox.filesystem.disabled",
                "sandbox.allowAppleEvents",
                "sandbox.allowNestedUserNamespaces",
                "sandbox.network.strictAllowlist",
                "sandbox.network.allowManagedDomainsOnly",
            ],
        ),
        reserved("it carries or pins a login", &["sandbox.credentials"]),
        reserved("it runs a command and carries a login", &["apiKeyHelper"]),
        reserved(
            OUTSIDE_THE_PROJECT,
            &[
                "autoMode",
                "skipAutoPermissionPrompt",
                "skipDangerousModePermissionPrompt",
                "allowManagedHooksOnly",
            ],
        ),
        reserved(
            RUNS_A_COMMAND,
            &[
                "statusLine",
                "fileSuggestion",
                "awsAuthRefresh",
                "awsCredentialExport",
                "gcpAuthRefresh",
                "otelHeadersHelper",
            ],
        ),
        reserved(
            OUTSIDE_THE_PROJECT,
            &[
                "allowManagedMcpServersOnly",
                "managedMcpServers",
                "allowAllClaudeAiMcps",
            ],
        ),
        reserved(PLUGIN_SOURCE, &["enabledPlugins", "extraKnownMarketplaces"]),
        reserved(
            OUTSIDE_THE_PROJECT,
            &[
                "prependPlugins",
                "appendPlugins",
                "pluginConfigs",
                "strictKnownMarketplaces",
                "blockedMarketplaces",
                "strictPluginOnlyCustomization",
                "syncClaudeAiPlugins",
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
            ],
        ),
        reserved(LOWERS_A_CHECK, &["skipWebFetchPreflight"]),
        reserved(STEERS_SIGN_IN, &["forceLoginMethod", "forceLoginOrgUUID"]),
        reserved(OUTSIDE_THE_PROJECT, &["disableSideloadFlags"]),
    ];
    rows.into_iter().flatten().collect()
}
