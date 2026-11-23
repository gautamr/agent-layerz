//! The checks of `alz lint` that read one layer alone (spec §10): every kind's reader and the
//! merge rules that one layer can break by itself, with none of the references to a more general
//! layer, which only a build can resolve.

use crate::diagnostic::{Code, Diagnostics};
use crate::item::SkillItems;
use crate::merge::agents::AgentState;
use crate::merge::hooks::HookState;
use crate::merge::instructions::InstructionState;
use crate::merge::mcp::McpState;
use crate::merge::models::ModelState;
use crate::merge::passthrough::PassthroughState;
use crate::merge::permissions::PermissionState;
use crate::merge::rules::RuleState;
use crate::merge::settings::SettingsState;
use crate::merge::skills::SkillState;
use crate::merge::stack::LayerContext;
use crate::stack::ResolvedLayer;

/// What is wrong with `layer` taken as a stack of its own: the errors and warnings of every kind
/// in the order of the reference (instructions, agents, skills, rules, hooks, permissions, MCP,
/// models, settings, passthrough), then the `slot` errors of its `provides_slots`. A diagnostic
/// that names a more general layer the layer does not have (`unresolved`) is dropped.
#[must_use]
pub fn check_alone(layer: &ResolvedLayer) -> Diagnostics {
    let context = LayerContext::alone(layer);
    let (_, instructions) = InstructionState::default().absorb(&context);
    let (_, agents) = AgentState::default().absorb(&context);
    let (_, skills) = SkillState::default().absorb(&context);
    let (_, rules) = RuleState::default().absorb(&context);
    let (_, hooks) = HookState::default().absorb(&context);
    let (permissions, found_permissions) = PermissionState::default().absorb(&context);
    let (_, mcp) = McpState::default().absorb(&context);
    let (_, models) = ModelState::default().absorb(&context);
    let (_, settings) = SettingsState::default().absorb(&context);
    let (_, passthrough) = PassthroughState::default().absorb(&context);
    let (provided, _) = SkillItems::read(layer);
    instructions
        .merge(agents)
        .merge(skills)
        .merge(rules)
        .merge(hooks)
        .merge(found_permissions)
        .merge(permissions.shadowed())
        .merge(mcp)
        .merge(models)
        .merge(settings)
        .merge(passthrough)
        .merge(provided.check_provided(layer.meta()))
        .into_iter()
        .filter(|diagnostic| diagnostic.code() != Code::Unresolved)
        .collect()
}
