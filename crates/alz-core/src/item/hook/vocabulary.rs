//! The values of a hook's fields (spec §7.5), each parsed once from the text of a file.

use std::num::NonZeroU32;

use crate::item::ToolName;
use crate::target::TargetName;

/// Text that is not a hook's value for a field: the rule that was broken. The error holds no
/// input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookRefusal {
    /// A blank `id`.
    Id,
    /// An `event` that is neither neutral nor `<target>:<Event>` for a known target.
    Event,
    /// A `match.tools` entry that is not a tool, or has a scope.
    Tool,
    /// An `on_fail` that is not `deny`, `warn` or `feedback`.
    OnFail,
    /// A blank `command`.
    Command,
    /// A `timeout` that is not a positive whole number of seconds.
    Timeout,
}

impl HookRefusal {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Event => {
                "must be session_start, pre_tool, post_tool, stop, subagent_stop or prompt_submit, or <target>:<Event> for claude or codex, such as claude:Notification"
            }
            Self::Tool => {
                "must name a tool, without a scope: read, write, edit, grep, glob, bash, web_fetch, web_search, agent, notebook_edit, todo_write, skill or an mcp__ tool"
            }
            Self::Id | Self::Command => "must not be empty",
            Self::OnFail => "must be deny, warn or feedback",
            Self::Timeout => "must be a positive whole number of seconds",
        }
    }
}

/// The id of a hook: text with a visible character, unique across the stack. The spec gives no
/// grammar, so it is printed as a quoted key, never as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HookId(String);

impl HookId {
    /// The id as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for HookId {
    type Error = HookRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            Err(HookRefusal::Id)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// The neutral events of §7.5.
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

impl NeutralEvent {
    /// The word the neutral vocabulary writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::Stop => "stop",
            Self::SubagentStop => "subagent_stop",
            Self::PromptSubmit => "prompt_submit",
        }
    }

    fn named(text: &str) -> Option<Self> {
        [
            Self::SessionStart,
            Self::PreTool,
            Self::PostTool,
            Self::Stop,
            Self::SubagentStop,
            Self::PromptSubmit,
        ]
        .into_iter()
        .find(|event| event.as_str() == text)
    }
}

/// A target's own event name: an upper-case letter, then at least one letter, `^[A-Z][A-Za-z]+$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NativeEventName(String);

impl NativeEventName {
    /// The name as written, such as `Notification`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn named(text: &str) -> Option<Self> {
        let mut letters = text.chars();
        let shape = letters
            .next()
            .is_some_and(|first| first.is_ascii_uppercase())
            && text.len() >= 2
            && letters.all(|letter| letter.is_ascii_alphabetic());
        shape.then(|| Self(text.to_owned()))
    }
}

/// A target's own event, written `<target>:<Event>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NativeEvent {
    target: TargetName,
    name: NativeEventName,
}

impl NativeEvent {
    /// The only target the event runs on.
    #[must_use]
    pub const fn target(&self) -> TargetName {
        self.target
    }

    /// The event's name in the target's vocabulary.
    #[must_use]
    pub const fn name(&self) -> &NativeEventName {
        &self.name
    }
}

/// The `event` field as written: a neutral event or a target's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum HookEvent {
    Neutral(NeutralEvent),
    Native(NativeEvent),
}

impl TryFrom<&str> for HookEvent {
    type Error = HookRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        NeutralEvent::named(text).map(Self::Neutral).map_or_else(
            || {
                text.split_once(':')
                    .and_then(|(target, name)| {
                        Some(NativeEvent {
                            target: TargetName::try_from(target).ok()?,
                            name: NativeEventName::named(name)?,
                        })
                    })
                    .map(Self::Native)
                    .ok_or(HookRefusal::Event)
            },
            Ok,
        )
    }
}

/// A tool a hook's `match` names, without a scope: a neutral tool or an `mcp__` tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HookTool(ToolName);

impl HookTool {
    /// The tool.
    #[must_use]
    pub const fn name(&self) -> &ToolName {
        &self.0
    }
}

impl TryFrom<&str> for HookTool {
    type Error = HookRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        ToolName::named(text).map(Self).ok_or(HookRefusal::Tool)
    }
}

/// A command line, used as written. `${PROJECT_DIR}` in it is the repository root, which each
/// adapter replaces with its own variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine(String);

impl CommandLine {
    /// The command line as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CommandLine {
    type Error = HookRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            Err(HookRefusal::Command)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// A target's own matcher string, passed through as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matcher(String);

impl Matcher {
    /// The matcher as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Matcher {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// What a hook does when it fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnFail {
    /// `deny`.
    Deny,
    /// `warn`, which a hook without `on_fail` does.
    #[default]
    Warn,
    /// `feedback`.
    Feedback,
}

impl TryFrom<&str> for OnFail {
    type Error = HookRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "deny" => Ok(Self::Deny),
            "warn" => Ok(Self::Warn),
            "feedback" => Ok(Self::Feedback),
            _ => Err(HookRefusal::OnFail),
        }
    }
}

/// A timeout: a positive whole number of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seconds(NonZeroU32);

impl Seconds {
    /// The seconds, at least 1.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// The timeout of a whole number, `None` unless it is in `1..=u32::MAX`.
    pub(super) fn of(number: i128) -> Option<Self> {
        u32::try_from(number)
            .ok()
            .and_then(NonZeroU32::new)
            .map(Self)
    }
}

/// Whether a hook can be disabled by a more specific layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Lock {
    /// A more specific layer may disable the hook.
    #[default]
    Open,
    /// `locked: true`: no layer may disable the hook (a security floor).
    Locked,
}

impl Lock {
    /// The lock of a `locked` field: absent or `false` is open.
    #[must_use]
    pub const fn of(locked: bool) -> Self {
        if locked { Self::Locked } else { Self::Open }
    }
}
