//! Agents (spec §7.2) for one target, with the model they ask for resolved through the routing
//! (§7.8).

use alz_adapter::{Agent, AgentTool, Isolation, Memory, ModelProblem, TargetId};

use crate::item::{self, ModelAlias, ToolGrant};
use crate::merge::{Contribution, Merged, MergedAgent, MergedModels, Traced};

use super::json::of_native;
use super::permissions::mode;
use super::sources::of_origins;

/// The agents that run on `target`: those whose `targets` is absent or holds it.
pub(super) fn of(merged: &Merged, target: &TargetId) -> Vec<Agent> {
    merged
        .agents()
        .values()
        .filter(|agent| {
            agent
                .value()
                .targets()
                .is_none_or(|targets| targets.contains(target))
        })
        .map(|agent| one(agent, merged.models(), target))
        .collect()
}

fn one(traced: &Traced<MergedAgent>, models: &MergedModels, target: &TargetId) -> Agent {
    let merged = traced.value();
    let (model, model_problem) = routed(merged, models, target);
    let scalars = merged.scalars();
    let mut agent = Agent::new(
        merged.name().as_str(),
        merged.description().as_str(),
        merged.body().compose(),
    );
    agent.tools = merged
        .tools()
        .map(|grants| grants.iter().map(tool).collect());
    agent.model = model;
    agent.model_problem = model_problem;
    agent.permission_mode = scalars.permission_mode().copied().map(mode);
    agent.skills = merged
        .skills()
        .iter()
        .map(|skill| skill.as_str().to_owned())
        .collect();
    agent.isolation = scalars
        .isolation()
        .map_or(Isolation::None, |isolation| match isolation {
            item::Isolation::None => Isolation::None,
            item::Isolation::Worktree => Isolation::Worktree,
        });
    agent.memory = scalars
        .memory()
        .map_or(Memory::None, |memory| match memory {
            item::Memory::None => Memory::None,
            item::Memory::Project => Memory::Project,
            item::Memory::User => Memory::User,
            item::Memory::Local => Memory::Local,
        });
    agent.native = merged.native().of(target).map_or_else(Vec::new, |map| {
        map.iter()
            .map(|(key, value)| (key.to_owned(), of_native(value)))
            .collect()
    });
    agent.sources = of_origins(
        traced
            .provenance()
            .contributions()
            .iter()
            .map(Contribution::origin),
    );
    agent
}

fn tool(grant: &ToolGrant) -> AgentTool {
    AgentTool {
        name: grant.tool().as_str().to_owned(),
        scope: grant.scope().map(|scope| scope.as_str().to_owned()),
    }
}

/// The concrete model of an agent for `target`, or why it has none (§7.8): an explicit alias
/// first, else the alias of the agent's role. `inherit`, or no model and no routed role, asks for
/// none, and that is no problem.
fn routed(
    agent: &MergedAgent,
    models: &MergedModels,
    target: &TargetId,
) -> (Option<String>, Option<ModelProblem>) {
    let resolve = |alias: &ModelAlias| match models.model(alias, target) {
        Some(concrete) => (Some(concrete.as_str().to_owned()), None),
        None => (
            None,
            Some(ModelProblem::NoModelForTarget(alias.as_str().to_owned())),
        ),
    };
    match agent
        .scalars()
        .model()
        .filter(|id| id.as_str() != "inherit")
    {
        Some(id) => ModelAlias::try_from(id.as_str())
            .ok()
            .filter(|alias| models.aliases().contains_key(alias))
            .map_or_else(
                || (None, Some(ModelProblem::NotAnAlias(id.as_str().to_owned()))),
                |alias| resolve(&alias),
            ),
        None => agent
            .scalars()
            .role()
            .and_then(|role| models.alias_of(role))
            .map_or((None, None), resolve),
    }
}
