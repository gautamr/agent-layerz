//! The hooks of a layer (spec §7.5): `hooks/hooks.yaml`, a list of hooks to define and hooks to
//! disable, in file order.
//!
//! A defined hook has an id, a trigger, exactly one action, and what to do on failure. The types
//! hold only the legal combinations: a tool filter belongs to the two tool events only, a native
//! event carries its target's matcher and runs on its target only, and a hook runs a script of the
//! layer or a command line, never both and never neither. A disable entry holds an id and nothing
//! else.

mod vocabulary;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::layer::{ItemPath, LayerContents};
use crate::merge::OrderedSet;
use crate::path::SafeRelPath;
use crate::schema::{
    Checked, SchemaError, SchemaErrors, refine, refine_list, require, unknown_fields, zip, zip3,
};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{self, Node, Value, YamlText};

use super::{Glob, place, schema_diagnostics};
use vocabulary::HookEvent;
pub use vocabulary::{
    CommandLine, HookId, HookRefusal, HookTool, Lock, Matcher, NativeEvent, NativeEventName,
    NeutralEvent, OnFail, Seconds,
};

/// The fields of a hook entry.
const FIELDS: [&str; 11] = [
    "id", "event", "match", "matcher", "run", "command", "timeout", "on_fail", "targets", "locked",
    "disabled",
];

/// The moment of a tool event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMoment {
    /// `pre_tool`: before the tool runs.
    Before,
    /// `post_tool`: after the tool ran.
    After,
}

/// The events that carry no tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// `session_start`.
    SessionStart,
    /// `stop`.
    Stop,
    /// `subagent_stop`.
    SubagentStop,
    /// `prompt_submit`.
    PromptSubmit,
}

/// Which tool calls a tool event is for: all, or those that name one of the tools and, when there
/// are paths, one of the paths. Paths without tools are not a state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolFilter {
    /// Every tool call.
    Any,
    /// The calls of these tools, on these paths when there are any.
    Tools {
        /// The tools, at least one.
        tools: OrderedSet<HookTool>,
        /// The path globs, which may be none.
        paths: OrderedSet<Glob>,
    },
}

impl ToolFilter {
    /// The filter of `match`, `None` when there are paths but no tools.
    fn new(tools: Vec<HookTool>, paths: Vec<Glob>) -> Option<Self> {
        match (tools.is_empty(), paths.is_empty()) {
            (true, true) => Some(Self::Any),
            (true, false) => None,
            (false, _) => Some(Self::Tools {
                tools: tools.into_iter().collect(),
                paths: paths.into_iter().collect(),
            }),
        }
    }
}

/// When a hook runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// A neutral tool event, `pre_tool` or `post_tool`, with the calls it is for.
    Tool {
        /// Before or after the tool.
        when: ToolMoment,
        /// Which calls.
        filter: ToolFilter,
    },
    /// A neutral event that carries no tool.
    Lifecycle(LifecycleEvent),
    /// A target's own event, which runs on that target only and takes its `matcher`.
    Native {
        /// The target and the event's name.
        event: NativeEvent,
        /// The target's own matcher string, when the hook has one.
        matcher: Option<Matcher>,
    },
}

/// A script of the layer: a file below `hooks/scripts/` that the layer holds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HookScript(SafeRelPath);

impl HookScript {
    /// The path in the layer, such as `hooks/scripts/block-secrets.sh`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.0
    }

    /// The script that `run` names, which is written relative to `hooks/`.
    fn of(run: &str, contents: &LayerContents) -> Result<Self, Problem> {
        let path = SafeRelPath::try_from(format!("hooks/{run}").as_str())
            .ok()
            .filter(|path| matches!(ItemPath::of(path), ItemPath::HookScript { .. }))
            .ok_or(Problem::ScriptOutside)?;
        match contents.get(&path) {
            Some(_) => Ok(Self(path)),
            None => Err(Problem::ScriptMissing(path)),
        }
    }
}

/// What a hook does: exactly one of a script of the layer and a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAction {
    /// `run:`, a script that is copied into the repository's output.
    Run(HookScript),
    /// `command:`, a command line used as written.
    Command(CommandLine),
}

/// A hook that a layer defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    id: HookId,
    trigger: Trigger,
    action: HookAction,
    on_fail: OnFail,
    targets: Option<OrderedSet<TargetName>>,
    timeout: Option<Seconds>,
    lock: Lock,
}

impl Hook {
    /// The id, unique across the stack.
    #[must_use]
    pub const fn id(&self) -> &HookId {
        &self.id
    }

    /// When the hook runs.
    #[must_use]
    pub const fn trigger(&self) -> &Trigger {
        &self.trigger
    }

    /// What the hook does.
    #[must_use]
    pub const fn action(&self) -> &HookAction {
        &self.action
    }

    /// What happens when the hook fails: `warn` when the file says nothing.
    #[must_use]
    pub const fn on_fail(&self) -> OnFail {
        self.on_fail
    }

    /// The targets the hook is for, `None` for every target. A native event gives its target.
    #[must_use]
    pub const fn targets(&self) -> Option<&OrderedSet<TargetName>> {
        self.targets.as_ref()
    }

    /// The timeout in seconds, when the hook has one.
    #[must_use]
    pub const fn timeout(&self) -> Option<Seconds> {
        self.timeout
    }

    /// Whether a more specific layer may disable the hook.
    #[must_use]
    pub const fn lock(&self) -> Lock {
        self.lock
    }
}

/// One entry of `hooks.yaml`: a hook to define, or the id of a hook of a more general layer to
/// disable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEntry {
    /// `- id: x` with the hook's fields.
    Define(Hook),
    /// `- id: x` and `disabled: true`, nothing else.
    Disable(HookId),
}

/// What one layer holds of §7.5: the entries of its `hooks/hooks.yaml` in file order, and the
/// path they were read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookItems {
    file: Option<(SafeRelPath, Vec<HookEntry>)>,
}

impl HookItems {
    /// The path of the file and its entries in file order, `None` when the layer has no
    /// `hooks/hooks.yaml`.
    #[must_use]
    pub fn into_file(self) -> Option<(SafeRelPath, Vec<HookEntry>)> {
        self.file
    }

    /// Reads the hooks of `layer`. Every problem of every entry is reported; an entry with a
    /// problem gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        layer
            .contents()
            .iter()
            .find(|(path, _)| matches!(ItemPath::of(path), ItemPath::Hooks))
            .map_or_else(
                || (Self::default(), Diagnostics::new()),
                |(path, bytes)| {
                    let at = place(layer.meta().name(), path);
                    let (entries, found) = read_file(&at, bytes, layer.contents());
                    (
                        Self {
                            file: Some((path.clone(), entries)),
                        },
                        found,
                    )
                },
            )
    }
}

fn error(at: &Where, message: Message) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(Code::Hooks, Some(at.clone()), message))
}

/// The entries of the file, and the problems of those that have any.
fn read_file(at: &Where, bytes: &[u8], contents: &LayerContents) -> (Vec<HookEntry>, Diagnostics) {
    let document = YamlText::from_bytes(bytes)
        .and_then(|text| yaml::parse(&text))
        .map_err(|problem| Diagnostics::new().with(problem.into_diagnostic(at.clone())));
    match document {
        Err(found) => (Vec::new(), found),
        Ok(node) => match node.value() {
            Value::Null => (Vec::new(), Diagnostics::new()),
            Value::Seq(items) => items.iter().enumerate().fold(
                (Vec::new(), Diagnostics::new()),
                |(entries, found), (index, item)| match read_entry(at, index, item, contents) {
                    Ok(entry) => (entries.into_iter().chain([entry]).collect(), found),
                    Err(problems) => (entries, found.merge(problems)),
                },
            ),
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Map(_) => (
                Vec::new(),
                error(at, Message::new("hooks.yaml must be a list")),
            ),
        },
    }
}

fn read_entry(
    at: &Where,
    index: usize,
    node: &Node,
    contents: &LayerContents,
) -> Result<HookEntry, Diagnostics> {
    let schema = |errors: &SchemaErrors| schema_diagnostics(Code::Hooks, at, errors);
    let section = Section::new(node, KeyPath::root().item(index))
        .map_err(|problem| schema(&problem.into()))?;
    let id = require::<HookId>(&section, "id").map_err(|errors| schema(&errors))?;
    let here = at.clone().at(section.at());
    let named = |text: &'static str| {
        Message::new("hook ")
            .then(QuotedKey::from_untrusted(id.as_str()))
            .then(text)
    };
    if section.node("disabled").is_some() {
        let only_id = matches!(section.boolean("disabled"), Ok(Some(true)))
            && section.unknown_fields(&["id", "disabled"]).is_empty();
        return if only_id {
            Ok(HookEntry::Disable(id))
        } else {
            Err(error(
                &here,
                named(": a disable entry has only 'id' and 'disabled: true'"),
            ))
        };
    }
    let raw = Raw::read(&section).map_err(|errors| schema(&errors))?;
    raw.assemble(id.clone(), contents)
        .map(HookEntry::Define)
        .map_err(|problem| error(&here, problem.message(named(": "))))
}

/// The fields of a hook entry, each valid alone; how they fit together is [`Raw::assemble`].
struct Raw {
    event: HookEvent,
    tools: Vec<HookTool>,
    paths: Vec<Glob>,
    matcher: Option<Matcher>,
    run: Option<String>,
    command: Option<CommandLine>,
    timeout: Option<Seconds>,
    on_fail: OnFail,
    targets: Option<Vec<TargetName>>,
    lock: Lock,
}

impl Raw {
    fn read(section: &Section<'_>) -> Checked<Self> {
        let first = zip3(
            unknown_fields(section, &FIELDS),
            require::<HookEvent>(section, "event"),
            filter(section),
        );
        let second = zip3(
            text(section, "matcher").map(|text| text.map(|text| Matcher::from(text.as_str()))),
            refine::<CommandLine>(section, "command"),
            text(section, "run"),
        );
        let third = zip3(
            timeout(section),
            refine::<OnFail>(section, "on_fail"),
            refine_list::<TargetName>(section, "targets"),
        );
        let lock = section
            .boolean("locked")
            .map(|locked| Lock::of(locked.unwrap_or(false)))
            .map_err(SchemaErrors::from);
        zip(zip(first, second), zip(third, lock)).map(
            |(
                (((), event, (tools, paths)), (matcher, command, run)),
                ((timeout, on_fail, targets), lock),
            )| Self {
                event,
                tools,
                paths,
                matcher,
                run,
                command,
                timeout,
                on_fail: on_fail.unwrap_or_default(),
                targets,
                lock,
            },
        )
    }

    /// The hook these fields make, or the first way they do not fit together.
    fn assemble(self, id: HookId, contents: &LayerContents) -> Result<Hook, Problem> {
        let action = match (self.run, self.command) {
            (Some(run), None) => HookScript::of(&run, contents).map(HookAction::Run),
            (None, Some(command)) => Ok(HookAction::Command(command)),
            (None, None) | (Some(_), Some(_)) => Err(Problem::RunOrCommand),
        }?;
        let filter = ToolFilter::new(self.tools, self.paths).ok_or(Problem::PathsWithoutTools)?;
        let trigger = trigger(self.event, filter, self.matcher)?;
        let targets = reach(self.targets, &trigger)?;
        Ok(Hook {
            id,
            trigger,
            action,
            on_fail: self.on_fail,
            targets,
            timeout: self.timeout,
            lock: self.lock,
        })
    }
}

fn text(section: &Section<'_>, field: &'static str) -> Checked<Option<String>> {
    section
        .text(field)
        .map(|text| text.map(str::to_owned))
        .map_err(SchemaErrors::from)
}

/// The tools and paths of `match`, both empty when the hook has none.
fn filter(section: &Section<'_>) -> Checked<(Vec<HookTool>, Vec<Glob>)> {
    section.node("match").map_or_else(
        || Ok((Vec::new(), Vec::new())),
        |node| {
            let inner = Section::new(node, section.field_path("match"))?;
            zip3(
                unknown_fields(&inner, &["tools", "paths"]),
                refine_list::<HookTool>(&inner, "tools"),
                refine_list::<Glob>(&inner, "paths"),
            )
            .map(|((), tools, paths)| (tools.unwrap_or_default(), paths.unwrap_or_default()))
        },
    )
}

fn timeout(section: &Section<'_>) -> Checked<Option<Seconds>> {
    section
        .whole_number("timeout")
        .map_err(SchemaErrors::from)?
        .map(|number| {
            Seconds::of(number).ok_or_else(|| {
                SchemaErrors::from(SchemaError::refused(
                    section.field_path("timeout"),
                    HookRefusal::Timeout.into(),
                    section.node("timeout").map_or(section.at(), Node::at),
                ))
            })
        })
        .transpose()
}

/// The trigger for an event, the filter of `match` and the `matcher`.
fn trigger(
    event: HookEvent,
    filter: ToolFilter,
    matcher: Option<Matcher>,
) -> Result<Trigger, Problem> {
    match event {
        HookEvent::Native(event) => match filter {
            ToolFilter::Any => Ok(Trigger::Native { event, matcher }),
            ToolFilter::Tools { .. } => Err(Problem::NativeMatch),
        },
        HookEvent::Neutral(event) => {
            if matcher.is_some() {
                return Err(Problem::MatcherOnNeutral);
            }
            match event {
                NeutralEvent::PreTool => Ok(Trigger::Tool {
                    when: ToolMoment::Before,
                    filter,
                }),
                NeutralEvent::PostTool => Ok(Trigger::Tool {
                    when: ToolMoment::After,
                    filter,
                }),
                NeutralEvent::SessionStart => lifecycle(LifecycleEvent::SessionStart, &filter),
                NeutralEvent::Stop => lifecycle(LifecycleEvent::Stop, &filter),
                NeutralEvent::SubagentStop => lifecycle(LifecycleEvent::SubagentStop, &filter),
                NeutralEvent::PromptSubmit => lifecycle(LifecycleEvent::PromptSubmit, &filter),
            }
        }
    }
}

fn lifecycle(event: LifecycleEvent, filter: &ToolFilter) -> Result<Trigger, Problem> {
    match filter {
        ToolFilter::Any => Ok(Trigger::Lifecycle(event)),
        ToolFilter::Tools { .. } => Err(Problem::MatchNotTool),
    }
}

/// The targets of a hook: those it lists, or for a native event its own target, which it may
/// list but no other.
fn reach(
    listed: Option<Vec<TargetName>>,
    trigger: &Trigger,
) -> Result<Option<OrderedSet<TargetName>>, Problem> {
    let own = match trigger {
        Trigger::Native { event, .. } => Some(event.target()),
        Trigger::Tool { .. } | Trigger::Lifecycle(_) => None,
    };
    match (listed, own) {
        (None, None) => Ok(None),
        (None, Some(own)) => Ok(Some(OrderedSet::from_iter([own]))),
        (Some(listed), None) if listed.is_empty() => Err(Problem::NoTargets),
        (Some(listed), None) => Ok(Some(listed.into_iter().collect())),
        (Some(listed), Some(own)) => {
            let set: OrderedSet<TargetName> = listed.into_iter().collect();
            if set.as_slice() == [own] {
                Ok(Some(set))
            } else {
                Err(Problem::NativeTargets(own))
            }
        }
    }
}

/// Fields that are valid alone and do not fit together.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    RunOrCommand,
    ScriptOutside,
    ScriptMissing(SafeRelPath),
    PathsWithoutTools,
    NativeMatch,
    MatcherOnNeutral,
    MatchNotTool,
    NoTargets,
    NativeTargets(TargetName),
}

impl Problem {
    /// The message that continues `head`.
    fn message(&self, head: Message) -> Message {
        match self {
            Self::RunOrCommand => {
                head.then("needs exactly one of 'run' (a script in the layer) or 'command'")
            }
            Self::ScriptOutside => head
                .then("'run' must name a script below hooks/scripts/, written relative to hooks/"),
            Self::ScriptMissing(path) => head
                .then("script ")
                .then(QuotedKey::from_untrusted(path.as_str()))
                .then(" is not in the layer"),
            Self::PathsWithoutTools => head.then("'match.paths' needs 'match.tools'"),
            Self::NativeMatch => {
                head.then("a native event takes the target's own 'matcher', not 'match'")
            }
            Self::MatcherOnNeutral => head.then("'matcher' is only for native events; use 'match'"),
            Self::MatchNotTool => {
                head.then("'match' only applies to the tool events pre_tool and post_tool")
            }
            Self::NoTargets => head.then("'targets' must list at least one target"),
            Self::NativeTargets(target) => head
                .then("an event of ")
                .then(MessagePart::printable(target))
                .then(" only runs on that target"),
        }
    }
}

#[cfg(test)]
mod tests;
