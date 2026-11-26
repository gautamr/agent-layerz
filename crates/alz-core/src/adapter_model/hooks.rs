//! Hooks with their scripts (spec §7.5).

use std::num::NonZeroU32;

use alz_adapter::{
    Hook, HookEvent, HookMatch, HookRun, HookScript, NeutralEvent, OnFail, TargetId,
};

use crate::item::{self, HookAction, LifecycleEvent, Lock, ToolFilter, ToolMoment, Trigger};
use crate::merge::{Contribution, Merged, Traced};

use super::sources::of_origins;

/// Whether the hook runs on `target`: its `targets` say so, and a native event belongs to its own
/// target only.
pub(super) fn runs_on(hook: &item::Hook, target: &TargetId) -> bool {
    hook.targets()
        .is_none_or(|targets| targets.contains(target))
        && match hook.trigger() {
            Trigger::Native { event, .. } => event.target() == target,
            Trigger::Tool { .. } | Trigger::Lifecycle(_) => true,
        }
}

/// The hooks that run on `target`, in stack order and then file order.
pub(super) fn of(merged: &Merged, target: &TargetId) -> Vec<Hook> {
    merged
        .hooks()
        .iter()
        .filter(|traced| runs_on(traced.value(), target))
        .filter_map(one)
        .collect()
}

/// The event of the hook, as the neutral vocabulary or its target names it.
pub(super) fn event(hook: &item::Hook) -> HookEvent {
    match hook.trigger() {
        Trigger::Tool {
            when: ToolMoment::Before,
            ..
        } => HookEvent::Neutral(NeutralEvent::PreTool),
        Trigger::Tool {
            when: ToolMoment::After,
            ..
        } => HookEvent::Neutral(NeutralEvent::PostTool),
        Trigger::Lifecycle(LifecycleEvent::SessionStart) => {
            HookEvent::Neutral(NeutralEvent::SessionStart)
        }
        Trigger::Lifecycle(LifecycleEvent::Stop) => HookEvent::Neutral(NeutralEvent::Stop),
        Trigger::Lifecycle(LifecycleEvent::SubagentStop) => {
            HookEvent::Neutral(NeutralEvent::SubagentStop)
        }
        Trigger::Lifecycle(LifecycleEvent::PromptSubmit) => {
            HookEvent::Neutral(NeutralEvent::PromptSubmit)
        }
        Trigger::Native { event, .. } => HookEvent::Native {
            target: event.target().clone(),
            event: event.name().as_str().to_owned(),
        },
    }
}

/// What the hook does when its command fails.
pub(super) const fn on_fail(hook: &item::Hook) -> OnFail {
    match hook.on_fail() {
        item::OnFail::Deny => OnFail::Deny,
        item::OnFail::Warn => OnFail::Warn,
        item::OnFail::Feedback => OnFail::Feedback,
    }
}

/// The layer that defines the hook: the first to contribute. None only for an item with no
/// contribution, which the merge never makes.
pub(super) fn defined_by(traced: &Traced<item::Hook>) -> Option<alz_adapter::LayerName> {
    traced
        .provenance()
        .contributions()
        .first()
        .map(|first| first.layer().to_contract())
}

fn one(traced: &Traced<item::Hook>) -> Option<Hook> {
    let hook = traced.value();
    let mut made = Hook::new(
        hook.id().as_str(),
        event(hook),
        run(hook.action()),
        defined_by(traced)?,
    );
    made.matcher = matcher(hook.trigger());
    made.timeout = hook
        .timeout()
        .and_then(|seconds| NonZeroU32::new(seconds.get()));
    made.on_fail = on_fail(hook);
    made.locked = matches!(hook.lock(), Lock::Locked);
    made.sources = of_origins(
        traced
            .provenance()
            .contributions()
            .iter()
            .map(Contribution::origin),
    );
    Some(made)
}

/// What the hook runs. A script's path is the one below the layer's `hooks/`.
fn run(action: &HookAction) -> HookRun {
    match action {
        HookAction::Command(line) => HookRun::Command(line.as_str().to_owned()),
        HookAction::Run(script) => {
            let path = script.path().as_str();
            HookRun::Script(HookScript::new(
                path.strip_prefix("hooks/").unwrap_or(path),
                script.bytes().to_vec(),
                script.executable(),
            ))
        }
    }
}

fn matcher(trigger: &Trigger) -> HookMatch {
    match trigger {
        Trigger::Tool {
            filter: ToolFilter::Tools { tools, paths },
            ..
        } => HookMatch::Neutral {
            tools: tools
                .iter()
                .map(|tool| tool.name().as_str().to_owned())
                .collect(),
            paths: paths.iter().map(|glob| glob.as_str().to_owned()).collect(),
        },
        Trigger::Native {
            matcher: Some(matcher),
            ..
        } => HookMatch::Native(matcher.as_str().to_owned()),
        Trigger::Tool {
            filter: ToolFilter::Any,
            ..
        }
        | Trigger::Lifecycle(_)
        | Trigger::Native { matcher: None, .. } => HookMatch::Any,
    }
}
