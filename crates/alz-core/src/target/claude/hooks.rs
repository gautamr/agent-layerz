//! `hooks` of `.claude/settings.json` and the scripts under `.claude/hooks/` (spec §7.5, §8.1,
//! target-facts §8.5): one group per hook, grouped by native event in stack and file order.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::item::{
    Glob, Hook, HookAction, HookScript, LifecycleEvent, OnFail, ToolFilter, ToolMoment, Trigger,
};
use crate::merge::{Merged, Traced, Tree};
use crate::render::tree::{FileMode, GeneratedFile};
use crate::target::TargetName;
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
    pub(super) events: BTreeMap<String, Vec<Tree>>,
    pub(super) output: Output,
}

/// The hooks of `merged` that run on Claude. A script that two hooks run is copied once.
pub(super) fn render(merged: &Merged) -> Hooks {
    let all = all(merged);
    Hooks {
        output: Output {
            files: once(all.output.files),
            ..all.output
        },
        ..all
    }
}

/// The files in order, each path once: the copies of one script are the same file.
fn once(
    files: Vec<(crate::render::tree::OutputPath, GeneratedFile)>,
) -> Vec<(crate::render::tree::OutputPath, GeneratedFile)> {
    files
        .into_iter()
        .fold(Vec::new(), |mut kept, (path, file)| {
            if kept.iter().all(|(seen, _)| *seen != path) {
                kept.push((path, file));
            }
            kept
        })
}

fn all(merged: &Merged) -> Hooks {
    merged
        .hooks()
        .iter()
        .filter(|traced| runs_on_claude(traced.value()))
        .map(one)
        .fold(
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

/// A hook runs on Claude when its targets say so, and a native event is Claude's own.
fn runs_on_claude(hook: &Hook) -> bool {
    hook.targets()
        .is_none_or(|targets| targets.contains(&TargetName::Claude))
        && match hook.trigger() {
            Trigger::Native { event, .. } => event.target() == TargetName::Claude,
            Trigger::Tool { .. } | Trigger::Lifecycle(_) => true,
        }
}

/// What one hook gave: its group under its event (none when its script cannot be written), and
/// its script and warnings.
struct Made {
    group: Option<(String, Tree)>,
    output: Output,
}

fn one(traced: &Traced<Hook>) -> Made {
    let hook = traced.value();
    let at = place(traced.provenance());
    let layer = traced
        .provenance()
        .contributions()
        .first()
        .map_or_else(String::new, |first| first.layer().as_str().to_owned());
    let (blocks, warning) = on_failure(hook, at.as_ref());
    let warnings = Output {
        files: Vec::new(),
        diagnostics: warning
            .into_iter()
            .fold(Diagnostics::new(), Diagnostics::with),
    };
    match command(hook, &layer) {
        Err(problem) => Made {
            group: None,
            output: warnings.then(Output {
                files: Vec::new(),
                diagnostics: Diagnostics::new().with(problem),
            }),
        },
        Ok((command, script)) => Made {
            group: Some((event_name(hook), group(hook, &command, blocks))),
            output: warnings.then(Output {
                files: script.into_iter().collect(),
                diagnostics: Diagnostics::new(),
            }),
        },
    }
}

/// The native event of the hook.
fn event_name(hook: &Hook) -> String {
    match hook.trigger() {
        Trigger::Tool {
            when: ToolMoment::Before,
            ..
        } => "PreToolUse".to_owned(),
        Trigger::Tool {
            when: ToolMoment::After,
            ..
        } => "PostToolUse".to_owned(),
        Trigger::Lifecycle(LifecycleEvent::SessionStart) => "SessionStart".to_owned(),
        Trigger::Lifecycle(LifecycleEvent::Stop) => "Stop".to_owned(),
        Trigger::Lifecycle(LifecycleEvent::SubagentStop) => "SubagentStop".to_owned(),
        Trigger::Lifecycle(LifecycleEvent::PromptSubmit) => "UserPromptSubmit".to_owned(),
        Trigger::Native { event, .. } => event.name().as_str().to_owned(),
    }
}

/// The command line, and the script it runs when the hook has one: copied once to
/// `.claude/hooks/<layer>/<path inside hooks/>`, so two hooks of one script share the file.
fn command(
    hook: &Hook,
    layer: &str,
) -> Result<
    (
        String,
        Option<(crate::render::tree::OutputPath, GeneratedFile)>,
    ),
    Diagnostic,
> {
    match hook.action() {
        HookAction::Command(line) => Ok((
            line.as_str()
                .replace(NEUTRAL_PROJECT_DIR, CLAUDE_PROJECT_DIR),
            None,
        )),
        HookAction::Run(script) => {
            let inside = script
                .path()
                .as_str()
                .strip_prefix("hooks/")
                .unwrap_or_else(|| script.path().as_str());
            let path = output_path(&format!("{HOOKS_DIR}/{layer}/{inside}"))?;
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
        bytes: script.bytes().to_vec(),
        mode: if script.executable() {
            FileMode::Executable
        } else {
            FileMode::Regular
        },
    }
}

/// The group of a hook: its handlers, and the matcher the hook gives.
fn group(hook: &Hook, command: &str, blocks: Option<&'static str>) -> Tree {
    let (tools, paths, matcher) = match hook.trigger() {
        Trigger::Tool {
            filter: ToolFilter::Tools { tools, paths },
            ..
        } => {
            let natives: Vec<String> = tools.iter().map(|tool| tools::tool(tool.name())).collect();
            let joined = natives.join("|");
            (natives, paths.iter().collect::<Vec<_>>(), Some(joined))
        }
        Trigger::Native { matcher, .. } => (
            Vec::new(),
            Vec::new(),
            matcher.as_ref().map(|found| found.as_str().to_owned()),
        ),
        Trigger::Tool {
            filter: ToolFilter::Any,
            ..
        }
        | Trigger::Lifecycle(_) => (Vec::new(), Vec::new(), None),
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
    let entries = [Some(("hooks", Tree::List(handlers)))]
        .into_iter()
        .chain([matcher.map(|matcher| ("matcher", Tree::Text(matcher)))])
        .flatten();
    map(entries)
}

fn condition(tool: &str, path: &Glob) -> String {
    format!("{tool}({})", path.as_str())
}

fn handler(
    hook: &Hook,
    command: &str,
    blocks: Option<&'static str>,
    condition: Option<String>,
) -> Tree {
    map([
        Some(("type", text("command"))),
        Some(("command", text(command))),
        condition.map(|rule| ("if", Tree::Text(rule))),
        hook.timeout()
            .map(|seconds| ("timeout", Tree::Int(i128::from(seconds.get())))),
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
                .then(QuotedKey::from_untrusted(hook.id().as_str()))
                .then(": ")
                .then(Stripped::from_untrusted(what)),
        )
    };
    match (hook.on_fail(), hook.trigger()) {
        (OnFail::Warn | OnFail::Feedback, _) => (None, None),
        (
            OnFail::Deny,
            Trigger::Native { .. }
            | Trigger::Tool {
                when: ToolMoment::Before,
                ..
            }
            | Trigger::Lifecycle(LifecycleEvent::SessionStart | LifecycleEvent::PromptSubmit),
        ) => (Some("block"), None),
        (
            OnFail::Deny,
            Trigger::Lifecycle(event @ (LifecycleEvent::Stop | LifecycleEvent::SubagentStop)),
        ) => {
            let word = match event {
                LifecycleEvent::Stop => "stop",
                LifecycleEvent::SubagentStop => "subagent_stop",
                LifecycleEvent::SessionStart | LifecycleEvent::PromptSubmit => "this event",
            };
            (
                None,
                Some(warn(&format!(
                    "on_fail: deny has no effect on {word} in Claude; the script must exit 2 itself"
                ))),
            )
        }
        (
            OnFail::Deny,
            Trigger::Tool {
                when: ToolMoment::After,
                ..
            },
        ) => (
            Some("block"),
            Some(warn("post_tool cannot block; a failure is shown to Claude")),
        ),
    }
}
