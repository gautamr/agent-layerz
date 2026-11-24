//! The merge of skills and commands, and the filling of slots (spec §7.3, §7.4).
//!
//! A skill is identified by its directory name and a command by its file name; they share one
//! namespace. The most specific layer that defines a name wins with the whole directory (or the
//! whole command): the earlier definition is dropped and the layer is added to the item's
//! provenance with [`Action::Replace`]. Skills are atomic, so there is nothing to extend.
//!
//! A slot is a skill name that a layer's `requires_slots` or a merged agent's `skills` lists. When
//! the finished set has no item of that name, `strict_slots: true` makes it a `slot-missing`
//! error, and otherwise the name gets a stub (`Merged::slot_stubs`) and a warning. An agent that
//! lists a command is a `slot-command` error: a command can be called only explicitly, so it
//! cannot be preloaded.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{AgentName, Skill, SkillItems, SkillName};
use crate::layer::LayerName;
use crate::merge::action::Action;
use crate::merge::agents::MergedAgent;
use crate::merge::algebra::Replace;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::untrusted_text::Stripped;

/// Who asked for a slot: the first to name it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotDemand {
    /// A layer's `requires_slots`.
    Layer(LayerName),
    /// A merged agent's `skills`.
    Agent(AgentName),
}

impl SlotDemand {
    /// The words that name the demand in a message.
    fn message(&self, head: Message) -> Message {
        match self {
            Self::Layer(layer) => head
                .then(MessagePart::printable(layer))
                .then(" (requires_slots)"),
            Self::Agent(agent) => head.then("agent ").then(MessagePart::printable(agent)),
        }
    }
}

/// The skills and commands after the merge, and the slots no layer filled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MergedSkills {
    pub(super) items: Replace<SkillName, Traced<Skill>>,
    pub(super) stubs: BTreeMap<SkillName, SlotDemand>,
}

/// The state of the fold for skills and commands: what is defined so far, and the slots the layers
/// required.
#[derive(Debug, Default)]
pub(super) struct SkillState {
    items: Replace<SkillName, Traced<Skill>>,
    required: BTreeMap<SkillName, SlotDemand>,
}

impl SkillState {
    /// The state after one more, more specific, layer: its items replace those of the same name.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = SkillItems::read(context.layer());
        let (skills, slots) = items.into_parts();
        let arrived: Replace<SkillName, Traced<Skill>> = skills
            .into_iter()
            .map(|skill| {
                let name = skill.name().clone();
                let origin = Origin::new(context.layer(), skill.source().clone());
                let traced = match self.items.get(&name) {
                    Some(previous) => previous.clone().revised(skill, origin, Action::Replace),
                    None => Traced::new(skill, origin, Action::Define),
                };
                (name, traced)
            })
            .collect();
        let required = slots.into_iter().fold(self.required, |mut all, name| {
            all.entry(name)
                .or_insert_with(|| SlotDemand::Layer(layer.clone()));
            all
        });
        (
            Self {
                items: self.items.merge(arrived),
                required,
            },
            found,
        )
    }

    /// The finished set, with `agents` as merged. Every slot that a layer required or an agent
    /// lists must be in it: a missing one is an error when `strict`, else a warning with a stub.
    pub(super) fn finish(
        self,
        agents: &BTreeMap<AgentName, Traced<MergedAgent>>,
        strict: bool,
    ) -> (MergedSkills, Diagnostics) {
        let preloaded = || {
            agents.iter().flat_map(|(name, agent)| {
                agent
                    .value()
                    .skills()
                    .iter()
                    .map(move |skill| (name, agent, skill))
            })
        };
        let commands: Diagnostics = preloaded()
            .filter(|(_, _, skill)| {
                matches!(
                    self.items.get(skill).map(Traced::value),
                    Some(Skill::Command(_))
                )
            })
            .map(|(name, agent, skill)| slot_command(name, agent, skill))
            .collect();
        let demands = preloaded().fold(self.required, |mut all, (name, _, skill)| {
            all.entry(skill.clone())
                .or_insert_with(|| SlotDemand::Agent(name.clone()));
            all
        });
        let (missing, found) = demands
            .into_iter()
            .filter(|(skill, _)| self.items.get(skill).is_none())
            .fold(
                (BTreeMap::new(), commands),
                |(mut missing, found), (skill, demand)| {
                    let problem = slot_missing(&skill, &demand, agents, strict);
                    if !strict {
                        missing.insert(skill, demand);
                    }
                    (missing, found.with(problem))
                },
            );
        (
            MergedSkills {
                items: self.items,
                stubs: missing,
            },
            found,
        )
    }
}

/// Where an agent was defined: its layer and file.
fn agent_place(agent: &Traced<MergedAgent>) -> Option<Where> {
    agent.provenance().contributions().first().map(|first| {
        Where::layer_file(
            Stripped::from_untrusted(first.layer().as_str()),
            Stripped::from_untrusted(agent.value().file().as_str()),
        )
    })
}

fn slot_command(name: &AgentName, agent: &Traced<MergedAgent>, skill: &SkillName) -> Diagnostic {
    Diagnostic::error(
        Code::SlotCommand,
        agent_place(agent),
        Message::new("agent ")
            .then(MessagePart::printable(name))
            .then(" preloads ")
            .then(MessagePart::printable(skill))
            .then(", which is a command; commands cannot be preloaded"),
    )
}

/// The diagnostic for a slot with no item: an error when `strict`, else a warning that a stub
/// is emitted. It stands at the file of the demand.
fn slot_missing(
    skill: &SkillName,
    demand: &SlotDemand,
    agents: &BTreeMap<AgentName, Traced<MergedAgent>>,
    strict: bool,
) -> Diagnostic {
    let at = match demand {
        SlotDemand::Layer(layer) => Some(Where::layer_file(
            Stripped::from_untrusted(layer.as_str()),
            Stripped::from_untrusted("layer.yaml"),
        )),
        SlotDemand::Agent(agent) => agents.get(agent).and_then(agent_place),
    };
    let message = demand
        .message(
            Message::new("slot ")
                .then(MessagePart::printable(skill))
                .then(" required by "),
        )
        .then(" is not filled by any layer");
    if strict {
        Diagnostic::error(Code::SlotMissing, at, message)
    } else {
        Diagnostic::warning(
            Code::SlotMissing,
            at,
            message.then("; emitting a stub skill"),
        )
    }
}

#[cfg(test)]
mod tests;
