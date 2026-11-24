//! The rows for Claude Code: the keys of `settings.json` that a project file may carry, and the
//! frontmatter fields of a subagent that no neutral field covers. Names and types are recorded in
//! `docs/target-facts.md` §14 (TASK-79).
//!
//! A key that the security table ([`super::super::native_keys`]) refuses is not here: the table
//! reports it with its reason. A key that Claude Code reads from a user or managed scope only, or
//! that §14.4 leaves out, is not here either, so `settings.yaml` refuses it as unlisted. The keys
//! with a floor are here, because a floor key is accepted: only its value may not get looser.

use super::{Field, Shape};

const fn flag(name: &'static str) -> Field {
    Field::new(name, Shape::Bool)
}

const fn text(name: &'static str) -> Field {
    Field::new(name, Shape::Text)
}

const fn whole(name: &'static str) -> Field {
    Field::new(name, Shape::Whole)
}

const fn number(name: &'static str) -> Field {
    Field::new(name, Shape::Number)
}

const fn choice(name: &'static str, values: &'static [&'static str]) -> Field {
    Field::new(name, Shape::Choice(values))
}

const fn list_of_text(name: &'static str) -> Field {
    Field::new(name, Shape::List(&Shape::Text))
}

const fn object(name: &'static str, fields: &'static [Field]) -> Field {
    Field::new(name, Shape::Object(fields))
}

const EFFORT: &[&str] = &["low", "medium", "high", "xhigh"];
const EFFORT_CAP: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const CACHE_TTL: &[&str] = &["5m", "1h"];
const DISABLE: &[&str] = &["disable"];

/// An entry of an MCP allowlist or denylist: exactly one of three keys.
const SERVER_RULE: Shape = Shape::Single(&[
    text("serverName"),
    list_of_text("serverCommand"),
    text("serverUrl"),
]);

/// What a model name maps to in `modelSettings`.
const MODEL_SETTING: Shape = Shape::Object(&[
    choice("effortLevel", EFFORT),
    choice("maxEffortLevel", EFFORT_CAP),
    Field::new(
        "autoCompactWindow",
        Shape::Either(&[Shape::Whole, Shape::Choice(&["auto"])]),
    ),
]);

/// An entry of `spinnerTipsOverride.tips`: text, or a tip with an id.
const SPINNER_TIP: Shape = Shape::Either(&[
    Shape::Text,
    Shape::Object(&[
        Field::required("id", Shape::Text),
        Field::required("text", Shape::Text),
        whole("cooldownSessions"),
        whole("priority"),
    ]),
]);

/// The keys of a project's `.claude/settings.json` that `settings.yaml` accepts under `claude:`.
pub const CLAUDE_SETTINGS: &[Field] = &[
    // Model and responses.
    text("$schema"),
    text("advisorModel"),
    flag("alwaysThinkingEnabled"),
    list_of_text("availableModels"),
    choice("effortLevel", EFFORT),
    flag("enforceAvailableModels"),
    list_of_text("fallbackModel"),
    flag("fastMode"),
    flag("fastModePerSessionOptIn"),
    text("language"),
    choice("maxEffortLevel", EFFORT_CAP),
    Field::new("modelOverrides", Shape::Map(&Shape::Text)),
    Field::new("modelSettings", Shape::Map(&MODEL_SETTING)),
    text("outputStyle"),
    choice("promptCacheTtl", CACHE_TTL),
    flag("showThinkingSummaries"),
    choice("subagentPromptCacheTtl", CACHE_TTL),
    flag("switchModelsOnFlag"),
    flag("ultracode"),
    // Permissions.
    choice("disableAutoMode", DISABLE),
    // Sandbox.
    object(
        "sandbox",
        &[
            flag("enabled"),
            flag("failIfUnavailable"),
            flag("autoAllowBashIfSandboxed"),
            list_of_text("excludedCommands"),
            flag("allowUnsandboxedCommands"),
            object(
                "filesystem",
                &[
                    list_of_text("allowWrite"),
                    list_of_text("denyWrite"),
                    list_of_text("denyRead"),
                    list_of_text("allowRead"),
                ],
            ),
            Field::new("ignoreViolations", Shape::Map(&Shape::List(&Shape::Text))),
            flag("enableWeakerNestedSandbox"),
            flag("enableWeakerNetworkIsolation"),
            object(
                "network",
                &[
                    list_of_text("allowUnixSockets"),
                    flag("allowAllUnixSockets"),
                    flag("allowLocalBinding"),
                    list_of_text("allowMachLookup"),
                    list_of_text("allowedDomains"),
                    list_of_text("deniedDomains"),
                ],
            ),
        ],
    ),
    // Memory and context.
    flag("autoCompactEnabled"),
    whole("autoCompactWindow"),
    text("autoMemoryDirectory"),
    flag("autoMemoryEnabled"),
    whole("bashOutputMaxChars"),
    list_of_text("claudeMdExcludes"),
    Field::new("env", Shape::Map(&Shape::Text)),
    flag("fileCheckpointingEnabled"),
    flag("idleCompaction"),
    text("plansDirectory"),
    number("skillListingBudgetFraction"),
    whole("skillListingMaxDescChars"),
    // Interface and terminal.
    flag("autoScrollEnabled"),
    list_of_text("companyAnnouncements"),
    choice("defaultShell", &["bash", "powershell"]),
    choice("editorMode", &["normal", "vim"]),
    flag("emojiCompletionEnabled"),
    choice("keybindingFlavor", &["classic", "readline"]),
    whole("maxProseWidth"),
    flag("prefersReducedMotion"),
    flag("promptSuggestionEnabled"),
    flag("respectGitignore"),
    flag("respondToBashCommands"),
    flag("showClearContextOnPlanAccept"),
    flag("showTurnDuration"),
    flag("spinnerTipsEnabled"),
    object(
        "spinnerTipsOverride",
        &[
            Field::new("tips", Shape::List(&SPINNER_TIP)),
            text("tipsFile"),
            text("label"),
            flag("excludeDefault"),
        ],
    ),
    object(
        "spinnerVerbs",
        &[
            Field::required("verbs", Shape::List(&Shape::Text)),
            Field::required("mode", Shape::Choice(&["append", "replace"])),
        ],
    ),
    flag("syntaxHighlightingDisabled"),
    flag("terminalProgressBarEnabled"),
    flag("terminalTitleFromRename"),
    // The documented values, and `custom:<slug>`, are all text.
    text("theme"),
    choice("timeFormat", &["auto", "12-hour", "24-hour", "24-hour-utc"]),
    text("timeZone"),
    choice("tui", &["default", "fullscreen"]),
    flag("verbose"),
    choice("viewMode", &["default", "verbose", "focus"]),
    object(
        "voice",
        &[
            flag("enabled"),
            flag("autoSubmit"),
            choice("mode", &["hold", "tap"]),
        ],
    ),
    flag("voiceEnabled"),
    flag("wheelScrollAccelerationEnabled"),
    // Git and attribution.
    Field::new(
        "attribution",
        Shape::Either(&[
            Shape::Object(&[text("commit"), text("pr"), flag("sessionUrl")]),
            Shape::False,
        ]),
    ),
    flag("includeCoAuthoredBy"),
    flag("includeGitInstructions"),
    text("prUrlTemplate"),
    // Hooks and automation.
    list_of_text("allowedHttpHookUrls"),
    flag("disableWorkflows"),
    flag("enableWorkflows"),
    list_of_text("httpHookAllowedEnvVars"),
    flag("workflowKeywordTriggerEnabled"),
    choice(
        "workflowSizeGuideline",
        &["unrestricted", "small", "medium", "large"],
    ),
    // Skills and MCP.
    flag("disableBundledSkills"),
    flag("disableSkillShellExecution"),
    Field::new(
        "skillOverrides",
        Shape::Map(&Shape::Choice(&[
            "on",
            "name-only",
            "user-invocable-only",
            "off",
        ])),
    ),
    Field::new("allowedMcpServers", Shape::List(&SERVER_RULE)),
    Field::new("deniedMcpServers", Shape::List(&SERVER_RULE)),
    flag("disableClaudeAiConnectors"),
    // Agents, sessions and worktrees.
    text("agent"),
    choice("crossSessionInbound", &["accept", "hold", "refuse"]),
    flag("disableAgentView"),
    flag("isolatePeerMachines"),
    choice("teammateMode", &["in-process", "auto", "tmux", "iterm2"]),
    object(
        "worktree",
        &[
            choice("baseRef", &["fresh", "head"]),
            list_of_text("symlinkDirectories"),
            list_of_text("sparsePaths"),
            choice("bgIsolation", &["worktree", "none"]),
        ],
    ),
    // Remote, desktop and notifications.
    flag("agentPushNotifEnabled"),
    flag("awaySummaryEnabled"),
    flag("disableArtifact"),
    choice("disableDeepLinkRegistration", DISABLE),
    flag("disableRemoteControl"),
    flag("enableArtifact"),
    flag("inputNeededNotifEnabled"),
    choice(
        "preferredNotifChannel",
        &[
            "auto",
            "terminal_bell",
            "iterm2",
            "iterm2_with_bell",
            "kitty",
            "ghostty",
            "notifications_disabled",
        ],
    ),
    object("remote", &[text("defaultEnvironmentId")]),
    flag("remoteControlAtStartup"),
    // Updates and privacy.
    choice("autoUpdatesChannel", &["latest", "stable"]),
    text("minimumVersion"),
    whole("cleanupPeriodDays"),
    number("feedbackSurveyRate"),
];

/// The frontmatter fields of a Claude subagent that have no neutral field (`claude:` in an agent
/// file). `tools`, `model`, `permissionMode`, `skills`, `isolation` and `memory` have one, and
/// `mcpServers` and `hooks` come only through `mcp.yaml` and `hooks.yaml`.
pub const CLAUDE_AGENT_FIELDS: &[Field] = &[
    Field::new(
        "disallowedTools",
        Shape::Either(&[Shape::Text, Shape::List(&Shape::Text)]),
    ),
    whole("maxTurns"),
    whole("autoCompactWindow"),
    flag("background"),
    flag("omitClaudeMd"),
    choice("effort", &["low", "medium", "high", "xhigh", "max"]),
    choice(
        "color",
        &[
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "cyan",
        ],
    ),
    text("initialPrompt"),
    object("experimental", &[choice("cacheTtl", CACHE_TTL)]),
];
