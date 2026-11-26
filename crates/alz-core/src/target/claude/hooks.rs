//! `hooks` of `.claude/settings.json` and the scripts under `.claude/hooks/` (spec §7.5, §8.1,
//! target-facts §8.5): one group per hook, grouped by native event in stack and file order.

use std::collections::BTreeMap;

use alz_adapter::{
    Hook, HookEvent, HookMatch, HookRun, HookScript, JsonValue, Model, NeutralEvent, OnFail,
};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::render::tree::{FileMode, GeneratedFile, OutputPath};
use crate::untrusted_text::{QuotedKey, Stripped};

use super::document::{map, text};
use super::frontmatter::place;
use super::{Output, output_path, tools};

/// The folder of the copied scripts.
const HOOKS_DIR: &str = ".claude/hooks";
/// The project-directory variable of the neutral vocabulary.
const NEUTRAL_PROJECT_DIR: &str = "${PROJECT_DIR}";
/// What Claude expands to the repository root, in a hook command (target-facts §4).
const CLAUDE_PROJECT_DIR: &str = "${CLAUDE_PROJECT_DIR}";

/// The groups of each native event in order, the scripts they run and what the rendering found.
pub(super) struct Hooks {
    pub(super) events: BTreeMap<String, Vec<JsonValue>>,
    pub(super) output: Output,
}

/// The hooks of `model`, which are those that run on Claude. A script that two hooks run is
/// copied once.
pub(super) fn render(model: &Model) -> Hooks {
    let all = all(model);
    Hooks {
        output: Output {
            files: once(all.output.files),
            ..all.output
        },
        ..all
    }
}

/// The files in order, each path once: the copies of one script are the same file.
fn once(files: Vec<(OutputPath, GeneratedFile)>) -> Vec<(OutputPath, GeneratedFile)> {
    files
        .into_iter()
        .fold(Vec::new(), |mut kept, (path, file)| {
            if kept.iter().all(|(seen, _)| *seen != path) {
                kept.push((path, file));
            }
            kept
        })
}

fn all(model: &Model) -> Hooks {
    model.hooks.iter().map(one).fold(
        Hooks {
            events: BTreeMap::new(),
            output: Output::default(),
        },
        |hooks, made| Hooks {
            events: match made.group {
                Some((event, group)) => {
                    let mut events = hooks.events;
                    events.entry(event).or_default().push(group);
                    events
                }
                None => hooks.events,
            },
            output: hooks.output.then(made.output),
        },
    )
}

/// What one hook gave: its group under its event (none when its script cannot be written), and
/// its script and warnings.
struct Made {
    group: Option<(String, JsonValue)>,
    output: Output,
}

fn one(hook: &Hook) -> Made {
    let at = place(&hook.sources);
    let (blocks, warning) = on_failure(hook, at.as_ref());
    let warnings = Output {
        files: Vec::new(),
        diagnostics: warning
            .into_iter()
            .fold(Diagnostics::new(), Diagnostics::with),
    };
    match command(hook) {
        Err(problem) => Made {
            group: None,
            output: warnings.then(Output {
                files: Vec::new(),
                diagnostics: Diagnostics::new().with(problem),
            }),
        },
        Ok((command, script)) => Made {
            group: Some((event_name(&hook.event), group(hook, &command, blocks))),
            output: warnings.then(Output {
                files: script.into_iter().collect(),
                diagnostics: Diagnostics::new(),
            }),
        },
    }
}

/// The native event of the hook.
fn event_name(event: &HookEvent) -> String {
    match event {
        HookEvent::Neutral(NeutralEvent::PreTool) => "PreToolUse".to_owned(),
        HookEvent::Neutral(NeutralEvent::PostTool) => "PostToolUse".to_owned(),
        HookEvent::Neutral(NeutralEvent::SessionStart) => "SessionStart".to_owned(),
        HookEvent::Neutral(NeutralEvent::Stop) => "Stop".to_owned(),
        HookEvent::Neutral(NeutralEvent::SubagentStop) => "SubagentStop".to_owned(),
        HookEvent::Neutral(NeutralEvent::PromptSubmit) => "UserPromptSubmit".to_owned(),
        HookEvent::Native { event, .. } => event.clone(),
    }
}

/// The command line, and the script it runs when the hook has one: copied once to
/// `.claude/hooks/<layer>/<path inside hooks/>`, so two hooks of one script share the file.
fn command(hook: &Hook) -> Result<(String, Option<(OutputPath, GeneratedFile)>), Diagnostic> {
    match &hook.run {
        HookRun::Command(line) => Ok((line.replace(NEUTRAL_PROJECT_DIR, CLAUDE_PROJECT_DIR), None)),
        HookRun::Script(script) => {
            let path = output_path(&format!("{HOOKS_DIR}/{}/{}", hook.defined_by, script.path))?;
            Ok((
                format!("\"{CLAUDE_PROJECT_DIR}/{}\"", path.as_str()),
                Some((path, copy(script))),
            ))
        }
    }
}

/// The script as a generated file, with its executable bit.
fn copy(script: &HookScript) -> GeneratedFile {
    GeneratedFile {
        bytes: script.content.clone(),
        mode: if script.executable {
            FileMode::Executable
        } else {
            FileMode::Regular
        },
    }
}

/// The group of a hook: its handlers, and the matcher the hook gives.
fn group(hook: &Hook, command: &str, blocks: Option<&'static str>) -> JsonValue {
    let (tools, paths, matcher) = match &hook.matcher {
        HookMatch::Neutral { tools, paths } => {
            let natives: Vec<String> = tools.iter().map(|tool| tools::tool(tool)).collect();
            let joined = natives.join("|");
            (natives, paths.as_slice(), Some(joined))
        }
        HookMatch::Native(matcher) => (Vec::new(), &[][..], Some(matcher.clone())),
        HookMatch::Any => (Vec::new(), &[][..], None),
    };
    // The `if` field takes exactly one permission rule, so each tool and path gets a handler.
    let conditions: Vec<Option<String>> = if paths.is_empty() {
        vec![None]
    } else {
        tools
            .iter()
            .flat_map(|tool| paths.iter().map(move |path| Some(condition(tool, path))))
            .collect()
    };
    let handlers = conditions
        .into_iter()
        .map(|condition| handler(hook, command, blocks, condition))
        .collect();
    let entries = [Some(("hooks", JsonValue::Array(handlers)))]
        .into_iter()
        .chain([matcher.map(|matcher| ("matcher", JsonValue::Text(matcher)))])
        .flatten();
    map(entries)
}

fn condition(tool: &str, path: &str) -> String {
    format!("{tool}({path})")
}

fn handler(
    hook: &Hook,
    command: &str,
    blocks: Option<&'static str>,
    condition: Option<String>,
) -> JsonValue {
    map([
        Some(("type", text("command"))),
        Some(("command", text(command))),
        condition.map(|rule| ("if", JsonValue::Text(rule))),
        hook.timeout
            .map(|seconds| ("timeout", JsonValue::Int(i128::from(seconds.get())))),
        blocks.map(|how| ("onFailure", text(how))),
    ]
    .into_iter()
    .flatten())
}

/// `on_fail: deny` as `onFailure: block`, so a crashing guard blocks instead of letting the action
/// through, and the warning where Claude cannot do that.
fn on_failure(hook: &Hook, at: Option<&Where>) -> (Option<&'static str>, Option<Diagnostic>) {
    let warn = |what: &str| {
        Diagnostic::warning(
            Code::HookOnFail,
            at.cloned(),
            Message::new("hook ")
                .then(QuotedKey::from_untrusted(&hook.id))
                .then(": ")
                .then(Stripped::from_untrusted(what)),
        )
    };
    match (hook.on_fail, &hook.event) {
        (OnFail::Warn | OnFail::Feedback, _) => (None, None),
        (
            OnFail::Deny,
            HookEvent::Native { .. }
            | HookEvent::Neutral(
                NeutralEvent::PreTool | NeutralEvent::SessionStart | NeutralEvent::PromptSubmit,
            ),
        ) => (Some("block"), None),
        (OnFail::Deny, HookEvent::Neutral(NeutralEvent::Stop)) => (
            None,
            Some(warn(
                "on_fail: deny has no effect on stop in Claude; the script must exit 2 itself",
            )),
        ),
        (OnFail::Deny, HookEvent::Neutral(NeutralEvent::SubagentStop)) => (
            None,
            Some(warn(
                "on_fail: deny has no effect on subagent_stop in Claude; the script must exit 2 itself",
            )),
        ),
        (OnFail::Deny, HookEvent::Neutral(NeutralEvent::PostTool)) => (
            Some("block"),
            Some(warn("post_tool cannot block; a failure is shown to Claude")),
        ),
    }
}
