//! Hooks (spec §7.5), with their scripts.

use std::num::NonZeroU32;

use crate::layer_name::LayerName;
use crate::source::Source;
use crate::target_id::TargetId;

/// A neutral hook event of §7.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NeutralEvent {
    /// `session_start`.
    SessionStart,
    /// `pre_tool`.
    PreTool,
    /// `post_tool`.
    PostTool,
    /// `stop`.
    Stop,
    /// `subagent_stop`.
    SubagentStop,
    /// `prompt_submit`.
    PromptSubmit,
}

/// The event of a hook: a neutral event, or a target's own event written `<target>:<Event>`,
/// which runs on that target only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookEvent {
    /// A neutral event, which every target maps to its own.
    Neutral(NeutralEvent),
    /// A target's own event. `event` matches `^[A-Z][A-Za-z]+$`.
    Native {
        /// The target the event belongs to.
        target: TargetId,
        /// The event, as the target names it.
        event: String,
    },
}

/// What a hook does when its command fails (§7.5). The default is `warn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OnFail {
    /// `deny`: the tool use is blocked, where the target can block.
    Deny,
    /// `warn`.
    Warn,
    /// `feedback`.
    Feedback,
}

/// Which tool uses a hook applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookMatch {
    /// No `match` or `matcher`: every use.
    Any,
    /// A neutral `match`: bare neutral tools or `mcp__` tools, and path globs.
    Neutral {
        /// The tools of `match.tools`.
        tools: Vec<String>,
        /// The globs of `match.paths`.
        paths: Vec<String>,
    },
    /// The target's own `matcher` string, for a native event.
    Native(String),
}

/// A script of a layer that a hook runs (`run:`), copied into the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HookScript {
    /// The path below the layer's `hooks/`, as `scripts/block-secrets.sh`.
    pub path: String,
    /// The script, byte for byte.
    pub content: Vec<u8>,
    /// Whether the script is executable.
    pub executable: bool,
}

impl HookScript {
    /// A script.
    #[must_use]
    pub fn new(path: impl Into<String>, content: Vec<u8>, executable: bool) -> Self {
        Self {
            path: path.into(),
            content,
            executable,
        }
    }
}

/// What a hook runs: exactly one of a script or a command (§7.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookRun {
    /// `run:`, a script of the layer.
    Script(HookScript),
    /// `command:`, a command line used as written. `${PROJECT_DIR}` is the repository root, which
    /// each adapter replaces with its own variable.
    Command(String),
}

/// A hook that runs on the target.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hook {
    /// The id, unique across the stack.
    pub id: String,
    /// The event.
    pub event: HookEvent,
    /// Which tool uses it applies to.
    pub matcher: HookMatch,
    /// What it runs.
    pub run: HookRun,
    /// The timeout in seconds, when set.
    pub timeout: Option<NonZeroU32>,
    /// What happens when it fails.
    pub on_fail: OnFail,
    /// Whether a more specific layer may not disable it.
    pub locked: bool,
    /// The layer that defines it.
    pub defined_by: LayerName,
    /// The layer file it came from, for the header.
    pub sources: Vec<Source>,
}

impl Hook {
    /// An unlocked hook that matches every use and runs `run`, with `on_fail: warn`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        event: HookEvent,
        run: HookRun,
        defined_by: LayerName,
    ) -> Self {
        Self {
            id: id.into(),
            event,
            matcher: HookMatch::Any,
            run,
            timeout: None,
            on_fail: OnFail::Warn,
            locked: false,
            defined_by,
            sources: Vec::new(),
        }
    }
}
