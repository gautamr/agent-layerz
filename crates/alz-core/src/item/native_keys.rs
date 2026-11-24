//! The native keys with a security meaning (spec §7.9, §7.2): the one table that says which keys of
//! a target's native settings and native agent fields alz refuses or only lets get stricter.
//!
//! Every row of [`KEYS`] is a [`NativeKey`]: a target, a key path and a [`KeyRule`].
//!
//! - [`KeyRule::FirstClass`]: the key duplicates a first-class item, so `settings.yaml` refuses it
//!   and names the place that sets it.
//! - [`KeyRule::Reserved`]: the key is refused for the reason given: it runs a command, widens
//!   what a project may do, or is meant for a scope that alz does not write.
//! - [`KeyRule::Floor`]: the key may be set, but a more specific layer may only move it up its
//!   [`Strictness`]: a looser value, a list that gains an entry where none may, or a change of
//!   kind is a `settings` error that names the layer, the file and the key path.
//!
//! A key path is the key names from the target's mapping down, joined by `.`. A segment `*` is any
//! one key, and `**` is any run of keys or list items, so `**.features.hooks` is `hooks` in a
//! `features` mapping wherever it stands. A path is never the text of a layer: the errors print the
//! path of the row, so no value and no profile name of the layer reaches the output.
//!
//! The rows are the owner's approved list (TASK-55.5, 2026-11-23). TASK-76 corrects it. Adapters
//! may rely on a key only once `docs/target-facts.md` records it.
//!
//! [`loosenings`] is the check of the floors: pure, over the merged tree of the more general
//! layers and the tree of the more specific one.

use std::mem::discriminant;

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::merge::Tree;
use crate::target::TargetName;
use crate::untrusted_text::QuotedKey;

/// How a key that has a floor gets stricter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// A boolean; the payload is the stricter value, and once a layer sets it, a more specific
    /// layer may not set the other.
    Flag(bool),
    /// Text in ranks, loosest rank first; a value of a rank may move to its own or a later rank.
    /// A value that is in no rank may only stay as it was.
    Ladder(&'static [&'static [&'static str]]),
    /// A list or mapping that may not gain an entry (list item or mapping key, at any depth) that
    /// a more general layer did not set. Settings have no removal syntax, so it cannot shrink
    /// either.
    NoAdditions,
    /// A list that may only grow.
    Union,
}

/// What the table says about a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRule {
    /// The key duplicates a first-class item, set through the place named.
    FirstClass(&'static str),
    /// The key is refused, for the reason given.
    Reserved(&'static str),
    /// The key may only get stricter.
    Floor(Strictness),
}

/// A row of the table: a key path of a target and what is said about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeKey {
    target: TargetName,
    path: &'static str,
    rule: KeyRule,
}

impl NativeKey {
    /// The target whose native settings hold the key.
    #[must_use]
    pub const fn target(&self) -> TargetName {
        self.target
    }

    /// The key path, with `*` and `**` as wildcards.
    #[must_use]
    pub const fn path(&self) -> &'static str {
        self.path
    }

    /// What the table says about the key.
    #[must_use]
    pub const fn rule(&self) -> KeyRule {
        self.rule
    }

    /// The path as the errors print it: the target, then the key path.
    #[must_use]
    pub fn display_path(&self) -> String {
        format!("{}.{}", self.target, self.path)
    }

    /// Whether the key path of the row stands for `steps`.
    fn matches(&self, steps: &[Step<'_>]) -> bool {
        pattern_matches(&self.path.split('.').collect::<Vec<_>>(), steps)
    }
}

/// One step down a tree: a mapping key, or anything that no key path names (a list item, a key
/// that is not text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step<'a> {
    /// A mapping key.
    Key(&'a str),
    /// A list item or a key that is not text.
    Unnamed,
}

fn pattern_matches(pattern: &[&str], steps: &[Step<'_>]) -> bool {
    match (pattern.split_first(), steps.split_first()) {
        (None, None) => true,
        (Some((&"**", rest)), _) => {
            pattern_matches(rest, steps)
                || steps
                    .split_first()
                    .is_some_and(|(_, below)| pattern_matches(pattern, below))
        }
        (Some((&"*", rest)), Some((Step::Key(_), below))) => pattern_matches(rest, below),
        (Some((&name, rest)), Some((&Step::Key(key), below))) => {
            name == key && pattern_matches(rest, below)
        }
        (None, Some(_)) | (Some(_), None | Some((Step::Unnamed, _))) => false,
    }
}

const fn row(target: TargetName, path: &'static str, rule: KeyRule) -> NativeKey {
    NativeKey { target, path, rule }
}

const fn claude(path: &'static str, rule: KeyRule) -> NativeKey {
    row(TargetName::Claude, path, rule)
}

const fn codex(path: &'static str, rule: KeyRule) -> NativeKey {
    row(TargetName::Codex, path, rule)
}

const fn floor(strictness: Strictness) -> KeyRule {
    KeyRule::Floor(strictness)
}

const TRUE_IS_STRICTER: KeyRule = floor(Strictness::Flag(true));
const FALSE_IS_STRICTER: KeyRule = floor(Strictness::Flag(false));
const NO_ADDITIONS: KeyRule = floor(Strictness::NoAdditions);
const UNION: KeyRule = floor(Strictness::Union);

/// Keys set in a user, local or managed scope only, which Claude ignores in a project.
const OUTSIDE_THE_PROJECT: KeyRule =
    KeyRule::Reserved("it is read from a user or managed scope, not from a project");
const RUNS_A_COMMAND: KeyRule =
    KeyRule::Reserved("it runs a command outside hooks.yaml and its locks");
const PLUGIN_SOURCE: KeyRule =
    KeyRule::Reserved("it brings hooks and servers past the allowlists of mcp.yaml and hooks.yaml");
const ROUTES_TRAFFIC: KeyRule =
    KeyRule::Reserved("it runs a program or routes traffic, and is not a project setting");
const NO_ORDER: KeyRule = KeyRule::Reserved("its values have no order, so no layer may tighten it");
/// Keys that lower a safety check of the target.
const LOWERS_A_CHECK: KeyRule =
    KeyRule::Reserved("it turns off a safety check of Claude Code, which no layer may lower");
/// Keys that steer how the user signs in.
const STEERS_SIGN_IN: KeyRule =
    KeyRule::Reserved("it steers how the user signs in, which a repository layer may not set");
/// Codex ignores these keys in a project's `config.toml`, so they would do nothing there.
const IGNORED_IN_A_PROJECT: KeyRule =
    KeyRule::Reserved("Codex ignores it in a project's config.toml");

/// The table: the first row that matches a key path is the one that applies.
pub const KEYS: &[NativeKey] = &[
    // Claude: first-class items.
    claude("permissions", KeyRule::FirstClass("permissions.yaml")),
    claude("hooks", KeyRule::FirstClass("hooks/hooks.yaml")),
    claude("model", KeyRule::FirstClass("models.yaml (default)")),
    claude(
        "enabledMcpjsonServers",
        KeyRule::FirstClass("mcp.enable in the repo manifest"),
    ),
    claude(
        "disabledMcpjsonServers",
        KeyRule::FirstClass("mcp.enable in the repo manifest"),
    ),
    claude(
        "enableAllProjectMcpServers",
        KeyRule::FirstClass("mcp.enable in the repo manifest"),
    ),
    claude(
        "disableAllHooks",
        KeyRule::FirstClass("hooks.yaml `disabled` (locked hooks cannot be switched off)"),
    ),
    // Claude: floors.
    claude("sandbox.enabled", TRUE_IS_STRICTER),
    claude("sandbox.failIfUnavailable", TRUE_IS_STRICTER),
    claude("sandbox.autoAllowBashIfSandboxed", FALSE_IS_STRICTER),
    claude("sandbox.allowUnsandboxedCommands", FALSE_IS_STRICTER),
    claude("sandbox.excludedCommands", NO_ADDITIONS),
    claude("sandbox.filesystem.allowWrite", NO_ADDITIONS),
    claude("sandbox.filesystem.allowRead", NO_ADDITIONS),
    claude("sandbox.filesystem.denyWrite", UNION),
    claude("sandbox.filesystem.denyRead", UNION),
    claude("sandbox.ignoreViolations", NO_ADDITIONS),
    claude("sandbox.enableWeakerNestedSandbox", FALSE_IS_STRICTER),
    claude("sandbox.enableWeakerNetworkIsolation", FALSE_IS_STRICTER),
    claude("sandbox.network.allowedDomains", NO_ADDITIONS),
    claude("sandbox.network.allowUnixSockets", NO_ADDITIONS),
    claude("sandbox.network.allowMachLookup", NO_ADDITIONS),
    claude("sandbox.network.allowAllUnixSockets", FALSE_IS_STRICTER),
    claude("sandbox.network.allowLocalBinding", FALSE_IS_STRICTER),
    claude("sandbox.network.deniedDomains", UNION),
    claude(
        "disableAutoMode",
        floor(Strictness::Ladder(&[&["disable"]])),
    ),
    claude("allowedHttpHookUrls", NO_ADDITIONS),
    claude("httpHookAllowedEnvVars", NO_ADDITIONS),
    claude("disableSkillShellExecution", TRUE_IS_STRICTER),
    claude("allowedMcpServers", NO_ADDITIONS),
    claude("deniedMcpServers", UNION),
    claude("disableClaudeAiConnectors", TRUE_IS_STRICTER),
    // Claude: reserved.
    claude(
        "mcpServers",
        KeyRule::Reserved(
            "Claude Code documents no such settings key; servers are set through mcp.yaml",
        ),
    ),
    claude(
        "sandbox.network.httpProxyPort",
        KeyRule::Reserved("it routes traffic, and has no order"),
    ),
    claude(
        "sandbox.network.socksProxyPort",
        KeyRule::Reserved("it routes traffic, and has no order"),
    ),
    claude("sandbox.filesystem.disabled", OUTSIDE_THE_PROJECT),
    claude("sandbox.allowAppleEvents", OUTSIDE_THE_PROJECT),
    claude("sandbox.allowNestedUserNamespaces", OUTSIDE_THE_PROJECT),
    claude("sandbox.network.strictAllowlist", OUTSIDE_THE_PROJECT),
    claude(
        "sandbox.network.allowManagedDomainsOnly",
        OUTSIDE_THE_PROJECT,
    ),
    claude(
        "sandbox.credentials",
        KeyRule::Reserved("it carries or pins a login"),
    ),
    claude(
        "apiKeyHelper",
        KeyRule::Reserved("it runs a command and carries a login"),
    ),
    claude("autoMode", OUTSIDE_THE_PROJECT),
    claude("skipAutoPermissionPrompt", OUTSIDE_THE_PROJECT),
    claude("skipDangerousModePermissionPrompt", OUTSIDE_THE_PROJECT),
    claude("allowManagedHooksOnly", OUTSIDE_THE_PROJECT),
    claude("statusLine", RUNS_A_COMMAND),
    claude("fileSuggestion", RUNS_A_COMMAND),
    claude("awsAuthRefresh", RUNS_A_COMMAND),
    claude("awsCredentialExport", RUNS_A_COMMAND),
    claude("gcpAuthRefresh", RUNS_A_COMMAND),
    claude("otelHeadersHelper", RUNS_A_COMMAND),
    claude("allowManagedMcpServersOnly", OUTSIDE_THE_PROJECT),
    claude("managedMcpServers", OUTSIDE_THE_PROJECT),
    claude("allowAllClaudeAiMcps", OUTSIDE_THE_PROJECT),
    claude("enabledPlugins", PLUGIN_SOURCE),
    claude("extraKnownMarketplaces", PLUGIN_SOURCE),
    claude("prependPlugins", OUTSIDE_THE_PROJECT),
    claude("appendPlugins", OUTSIDE_THE_PROJECT),
    claude("pluginConfigs", OUTSIDE_THE_PROJECT),
    claude("strictKnownMarketplaces", OUTSIDE_THE_PROJECT),
    claude("blockedMarketplaces", OUTSIDE_THE_PROJECT),
    claude("strictPluginOnlyCustomization", OUTSIDE_THE_PROJECT),
    claude("syncClaudeAiPlugins", OUTSIDE_THE_PROJECT),
    claude(
        "sandbox.filesystem.allowManagedReadPathsOnly",
        OUTSIDE_THE_PROJECT,
    ),
    claude("sandbox.ripgrep", OUTSIDE_THE_PROJECT),
    claude("sandbox.bwrapPath", OUTSIDE_THE_PROJECT),
    claude("sandbox.socatPath", OUTSIDE_THE_PROJECT),
    claude("sandbox.network.tlsTerminate", OUTSIDE_THE_PROJECT),
    claude("processWrapper", OUTSIDE_THE_PROJECT),
    claude("policyHelper", OUTSIDE_THE_PROJECT),
    claude("sshHostCommands", OUTSIDE_THE_PROJECT),
    claude("forceLoginGatewayUrl", OUTSIDE_THE_PROJECT),
    claude("allowManagedPermissionRulesOnly", OUTSIDE_THE_PROJECT),
    claude("skipWebFetchPreflight", LOWERS_A_CHECK),
    claude("forceLoginMethod", STEERS_SIGN_IN),
    claude("forceLoginOrgUUID", STEERS_SIGN_IN),
    claude("disableSideloadFlags", OUTSIDE_THE_PROJECT),
    // Codex: first-class items.
    codex("model", KeyRule::FirstClass("models.yaml")),
    codex("mcp_servers", KeyRule::FirstClass("mcp.yaml")),
    codex("hooks", KeyRule::FirstClass("hooks/hooks.yaml")),
    codex("approval_policy", KeyRule::FirstClass("permissions.yaml")),
    codex("sandbox_mode", KeyRule::FirstClass("permissions.yaml")),
    codex("features.hooks", KeyRule::FirstClass("hooks/hooks.yaml")),
    codex(
        "features.codex_hooks",
        KeyRule::FirstClass("hooks/hooks.yaml"),
    ),
    codex("**.features.hooks", KeyRule::FirstClass("hooks/hooks.yaml")),
    codex(
        "**.features.codex_hooks",
        KeyRule::FirstClass("hooks/hooks.yaml"),
    ),
    // Codex: floors.
    codex("sandbox_workspace_write.network_access", FALSE_IS_STRICTER),
    codex("sandbox_workspace_write.writable_roots", NO_ADDITIONS),
    codex(
        "sandbox_workspace_write.exclude_tmpdir_env_var",
        TRUE_IS_STRICTER,
    ),
    codex(
        "sandbox_workspace_write.exclude_slash_tmp",
        TRUE_IS_STRICTER,
    ),
    codex("approvals_reviewer", APPROVALS_REVIEWER),
    codex("allow_login_shell", FALSE_IS_STRICTER),
    codex("web_search", WEB_SEARCH),
    codex("tools.web_search.allowed_domains", NO_ADDITIONS),
    // Codex: reserved.
    codex(
        "profile",
        KeyRule::Reserved("removed by Codex 0.169.0 (use --profile)"),
    ),
    codex("profiles", IGNORED_IN_A_PROJECT),
    codex("auto_review", NO_ORDER),
    codex("default_permissions", NO_ORDER),
    codex("permissions", NO_ORDER),
    codex(
        "projects.*.trust_level",
        KeyRule::Reserved("a layer must not mark a project as trusted"),
    ),
    codex("plugins", PLUGIN_SOURCE),
    codex("marketplaces", PLUGIN_SOURCE),
    codex("apps", NO_ORDER),
    codex("windows", OUTSIDE_THE_PROJECT),
    codex("notify", ROUTES_TRAFFIC),
    codex("model_provider", ROUTES_TRAFFIC),
    codex("model_providers", ROUTES_TRAFFIC),
    codex("openai_base_url", ROUTES_TRAFFIC),
    codex("chatgpt_base_url", ROUTES_TRAFFIC),
    codex("otel", ROUTES_TRAFFIC),
];

const APPROVALS_REVIEWER: KeyRule = floor(Strictness::Ladder(&[
    &["auto_review", "guardian_subagent"],
    &["user"],
]));
const WEB_SEARCH: KeyRule = floor(Strictness::Ladder(&[
    &["live"],
    &["indexed"],
    &["cached"],
    &["disabled"],
]));

/// A key of an agent's native fields that is refused: servers and hooks come only through
/// `mcp.yaml` and `hooks.yaml` (§7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentKey {
    target: TargetName,
    key: &'static str,
    owner: &'static str,
}

impl AgentKey {
    /// The target whose native fields hold the key.
    #[must_use]
    pub const fn target(&self) -> TargetName {
        self.target
    }

    /// The key, at the top of the target's mapping.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// The place that sets what the key would set.
    #[must_use]
    pub const fn owner(&self) -> &'static str {
        self.owner
    }
}

/// The keys of an agent's native fields that duplicate a first-class item.
pub const AGENT_KEYS: &[AgentKey] = &[
    AgentKey {
        target: TargetName::Claude,
        key: "mcpServers",
        owner: "mcp.yaml",
    },
    AgentKey {
        target: TargetName::Claude,
        key: "hooks",
        owner: "hooks/hooks.yaml",
    },
    AgentKey {
        target: TargetName::Codex,
        key: "mcp_servers",
        owner: "mcp.yaml",
    },
    AgentKey {
        target: TargetName::Codex,
        key: "hooks",
        owner: "hooks/hooks.yaml",
    },
];

/// The row of the table for `steps` below the mapping of `target`, when there is one.
#[must_use]
pub fn native_key(target: TargetName, steps: &[Step<'_>]) -> Option<&'static NativeKey> {
    KEYS.iter()
        .find(|key| key.target == target && key.matches(steps))
}

/// The row of `target` that refuses the key at `steps`, when one does: a first-class item or a
/// reserved key.
#[must_use]
pub fn refusal(target: TargetName, steps: &[Step<'_>]) -> Option<&'static NativeKey> {
    native_key(target, steps).filter(|key| match key.rule {
        KeyRule::FirstClass(_) | KeyRule::Reserved(_) => true,
        KeyRule::Floor(_) => false,
    })
}

/// The error for the key `key` that the table refuses, at `at`.
#[must_use]
pub fn refusal_error(key: &NativeKey, at: &Where) -> Option<Diagnostic> {
    let named = Message::new(QuotedKey::from_untrusted(&key.display_path()));
    let message = match key.rule {
        KeyRule::FirstClass(owner) => named.then(" is set through ").then(owner),
        KeyRule::Reserved(reason) => named.then(" is not set in settings.yaml: ").then(reason),
        KeyRule::Floor(_) => return None,
    };
    Some(Diagnostic::error(Code::Settings, Some(at.clone()), message))
}

/// Why a value is not as strict as the one a more general layer set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Looseness {
    /// The value is looser on the order of the key.
    Looser,
    /// A list or mapping gains an entry where none may.
    Added,
    /// The value is not of the kind the more general layer set.
    Kind,
}

impl Looseness {
    const fn sentence(self) -> &'static str {
        match self {
            Self::Looser => {
                " is a floor: this value is looser than the one a more general layer set"
            }
            Self::Added => {
                " is a floor: it may not gain entries that a more general layer did not set"
            }
            Self::Kind => " is a floor: its kind differs from the one a more general layer set",
        }
    }
}

/// A floor key that a more specific layer loosens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loosening {
    key: &'static NativeKey,
    how: Looseness,
}

impl Loosening {
    /// The row of the key.
    #[must_use]
    pub const fn key(&self) -> &'static NativeKey {
        self.key
    }

    /// How the value is looser.
    #[must_use]
    pub const fn how(&self) -> Looseness {
        self.how
    }

    /// The `settings` error at `at`, which names the layer and the file; the key path is the one
    /// of the row.
    #[must_use]
    pub fn diagnostic(&self, at: &Where) -> Diagnostic {
        Diagnostic::error(
            Code::Settings,
            Some(at.clone()),
            Message::new(QuotedKey::from_untrusted(&self.key.display_path()))
                .then(self.how.sentence()),
        )
    }
}

impl Strictness {
    /// Whether `after`, from a more specific layer, is at least as strict as `before`.
    fn admits(self, before: &Tree, after: &Tree) -> Result<(), Looseness> {
        match (self, before, after) {
            (Self::Flag(stricter), Tree::Bool(held), Tree::Bool(next)) => {
                if *held == stricter && *next != stricter {
                    Err(Looseness::Looser)
                } else {
                    Ok(())
                }
            }
            (Self::Ladder(ranks), Tree::Text(held), Tree::Text(next)) => {
                match (rank(ranks, held), rank(ranks, next)) {
                    (Some(held), Some(next)) if next < held => Err(Looseness::Looser),
                    (Some(_), Some(_)) => Ok(()),
                    (None, _) | (_, None) if held == next => Ok(()),
                    (None, _) | (_, None) => Err(Looseness::Looser),
                }
            }
            (Self::NoAdditions, Tree::List(_), Tree::List(_))
            | (Self::NoAdditions, Tree::Map(_), Tree::Map(_)) => {
                if adds(before, after) {
                    Err(Looseness::Added)
                } else {
                    Ok(())
                }
            }
            (Self::Union, Tree::List(_), Tree::List(_)) => Ok(()),
            (
                Self::Flag(_) | Self::Ladder(_) | Self::NoAdditions | Self::Union,
                Tree::Null
                | Tree::Bool(_)
                | Tree::Int(_)
                | Tree::Float(_)
                | Tree::Text(_)
                | Tree::List(_)
                | Tree::Map(_),
                _,
            ) => Err(Looseness::Kind),
        }
    }
}

/// The rank of `value`, loosest first.
fn rank(ranks: &[&[&str]], value: &str) -> Option<usize> {
    ranks.iter().position(|names| names.contains(&value))
}

/// Whether `after` has an entry, a list item or a mapping key at any depth, that `before` has
/// not, or a value of another kind where `before` has a list or mapping.
fn adds(before: &Tree, after: &Tree) -> bool {
    match (before, after) {
        (Tree::List(held), Tree::List(next)) => next.iter().any(|item| !held.contains(item)),
        (Tree::Map(held), Tree::Map(next)) => next
            .iter()
            .any(|(key, value)| held.get(key).is_none_or(|before| adds(before, value))),
        (held, next) => discriminant(held) != discriminant(next),
    }
}

/// The floor keys of `target` that `after`, the settings of a more specific layer, loosens
/// against `before`, the settings merged from the more general ones. A key that `before` does not
/// set, or sets to `null`, is free.
#[must_use]
pub fn loosenings(target: TargetName, before: &Tree, after: &Tree) -> Vec<Loosening> {
    descend(target, &[], Some(before), after)
}

fn descend(
    target: TargetName,
    steps: &[Step<'_>],
    before: Option<&Tree>,
    after: &Tree,
) -> Vec<Loosening> {
    let floor = native_key(target, steps).and_then(|key| match key.rule {
        KeyRule::Floor(strictness) => Some((key, strictness)),
        KeyRule::FirstClass(_) | KeyRule::Reserved(_) => None,
    });
    match (floor, before) {
        (Some((key, strictness)), Some(held)) if *held != Tree::Null => strictness
            .admits(held, after)
            .err()
            .map(|how| Loosening { key, how })
            .into_iter()
            .collect(),
        (Some(_), _) => Vec::new(),
        (None, _) => match after {
            Tree::Map(entries) => entries
                .iter()
                .flat_map(|(key, value)| {
                    let below = [steps, &[Step::Key(key)]].concat();
                    let held = match before {
                        Some(Tree::Map(held)) => held.get(key),
                        Some(_) | None => None,
                    };
                    descend(target, &below, held, value)
                })
                .collect(),
            Tree::List(items) => items
                .iter()
                .flat_map(|item| descend(target, &[steps, &[Step::Unnamed]].concat(), None, item))
                .collect(),
            Tree::Null | Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) => {
                Vec::new()
            }
        },
    }
}

#[cfg(test)]
mod tests;
