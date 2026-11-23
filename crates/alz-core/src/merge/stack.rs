//! The fold of a resolved stack into the merged model: a left fold over the layers in stack
//! order, most general first, where each kind of item of §7 absorbs one layer at a time.
//!
//! The typestate is `ResolvedStack` to [`Merging`] to [`Merged`]. [`Merged`] has private fields
//! and no public constructor, so the renderer can take only a stack that was merged. Every kind
//! is merged and every problem reported in one run; [`Merging::merged`] is `None` when any error
//! was found.
//!
//! # Adding a kind
//!
//! A kind of §7 (instructions, agents, skills, and the rest) is added in four steps, and the
//! compiler shows the places:
//!
//! 1. A module `merge/<kind>.rs` with its state and `absorb(state, &LayerContext) -> (state,
//!    Diagnostics)`, built from the structures of [`algebra`](super::algebra).
//! 2. A field for the state in `Accumulator`, and a line in `Accumulator::absorb` that folds the
//!    layer into it and adds the diagnostics.
//! 3. A field for the finished kind in [`Merged`], filled in `Accumulator::finish`, with an
//!    accessor.
//! 4. A case for each merge rule of §7 and its law, in the kind's module.

use std::collections::BTreeMap;

use crate::diagnostic::Diagnostics;
use crate::item::{AgentName, ClaudeExtra, RuleName};
use crate::layer::LayerName;
use crate::merge::agents::{AgentState, MergedAgent};
use crate::merge::instructions::{InstructionBlock, InstructionState, MergedInstructions};
use crate::merge::placement::{Placement, Placements};
use crate::merge::provenance::Traced;
use crate::merge::rules::{MergedRule, RuleState};
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack};

/// What a kind needs to absorb one layer: the layer, where it lies in the stack, the manifest and
/// where the layer lies in relation to the repository.
#[derive(Debug, Clone, Copy)]
pub struct LayerContext<'a> {
    layer: &'a ResolvedLayer,
    index: usize,
    manifest: &'a Manifest,
    placement: Placement,
}

impl<'a> LayerContext<'a> {
    /// The layer, with its description and the bytes of its files.
    #[must_use]
    pub const fn layer(&self) -> &'a ResolvedLayer {
        self.layer
    }

    /// The repository's manifest, for `mcp.enable` and the other options a kind reads.
    #[must_use]
    pub const fn manifest(&self) -> &'a Manifest {
        self.manifest
    }

    /// Where the layer lies in relation to the repository.
    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.placement
    }

    /// Whether this is the most general layer, the first of the stack.
    #[must_use]
    pub const fn is_first(&self) -> bool {
        self.index == 0
    }
}

/// The merged model of a stack: what the renderer writes from, and nothing else. It has no public
/// constructor; the only way to get one is [`merge`].
///
/// ```compile_fail
/// let _ = alz_core::merge::Merged::new();
/// ```
///
/// ```compile_fail
/// let _ = alz_core::merge::Merged { layers: Vec::new() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    layers: Vec<LayerName>,
    instructions: Vec<Traced<InstructionBlock>>,
    claude_extras: Vec<ClaudeExtra>,
    rules: BTreeMap<RuleName, Traced<MergedRule>>,
    agents: BTreeMap<AgentName, Traced<MergedAgent>>,
}

impl Merged {
    /// The names of the layers merged, in stack order.
    #[must_use]
    pub fn layers(&self) -> &[LayerName] {
        &self.layers
    }

    /// The instruction blocks in stack order, then by file name within a layer (§7.1), each with
    /// the layers that made it. A block of `mode: replace` stands at the place of the layer that
    /// replaced it.
    #[must_use]
    pub fn instructions(&self) -> &[Traced<InstructionBlock>] {
        &self.instructions
    }

    /// The text of each layer's `targets/claude/CLAUDE.extra.md` that has any, in stack order
    /// (§7.1).
    #[must_use]
    pub fn claude_extras(&self) -> &[ClaudeExtra] {
        &self.claude_extras
    }

    /// The path-scoped rules by name (§7.10), each with the layers that made it.
    #[must_use]
    pub const fn rules(&self) -> &BTreeMap<RuleName, Traced<MergedRule>> {
        &self.rules
    }

    /// The agents by name (§7.2), each with the layers that made it: its base, then each overlay.
    #[must_use]
    pub const fn agents(&self) -> &BTreeMap<AgentName, Traced<MergedAgent>> {
        &self.agents
    }
}

/// What merging a stack found: the merged model when no error was found, and every diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merging {
    merged: Option<Merged>,
    diagnostics: Diagnostics,
}

impl Merging {
    /// The result of a fold. The model is kept only when `diagnostics` holds no error, so the two
    /// can never disagree.
    fn of(merged: Merged, diagnostics: Diagnostics) -> Self {
        Self {
            merged: (!diagnostics.has_errors()).then_some(merged),
            diagnostics,
        }
    }

    /// The merged model, when no error was found.
    #[must_use]
    pub const fn merged(&self) -> Option<&Merged> {
        self.merged.as_ref()
    }

    /// Every error and warning, in the order the layers and kinds found them.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The model and the diagnostics, for a caller that takes them apart.
    #[must_use]
    pub fn into_parts(self) -> (Option<Merged>, Diagnostics) {
        (self.merged, self.diagnostics)
    }
}

/// The state of the fold: what the layers absorbed so far gave.
#[derive(Debug, Default)]
struct Accumulator {
    layers: Vec<LayerName>,
    instructions: InstructionState,
    rules: RuleState,
    agents: AgentState,
    diagnostics: Diagnostics,
}

impl Accumulator {
    /// The state after one more, more specific, layer. Each kind's `absorb` is called here, in
    /// the order of §7.
    fn absorb(self, context: &LayerContext<'_>) -> Self {
        let (instructions, found_instructions) = self.instructions.absorb(context);
        let (rules, found_rules) = self.rules.absorb(context);
        let (agents, found_agents) = self.agents.absorb(context);
        Self {
            layers: self
                .layers
                .into_iter()
                .chain([context.layer().meta().name().clone()])
                .collect(),
            instructions,
            rules,
            agents,
            diagnostics: self
                .diagnostics
                .merge(found_instructions)
                .merge(found_rules)
                .merge(found_agents),
        }
    }

    fn finish(self) -> Merging {
        let MergedInstructions {
            blocks,
            claude_extras,
        } = self.instructions.finish();
        Merging::of(
            Merged {
                layers: self.layers,
                instructions: blocks,
                claude_extras,
                rules: self.rules.finish(),
                agents: self.agents.finish(),
            },
            self.diagnostics,
        )
    }
}

/// Merges the layers of `stack` (§7). Pure: the same input gives an equal result, because every
/// collection is ordered and the fold runs in stack order.
#[must_use]
pub fn merge(stack: &ResolvedStack, manifest: &Manifest, placements: &Placements) -> Merging {
    stack
        .layers()
        .iter()
        .enumerate()
        .map(|(index, layer)| LayerContext {
            layer,
            index,
            manifest,
            placement: placements.of(layer.meta().name()),
        })
        .fold(Accumulator::default(), |state, context| {
            state.absorb(&context)
        })
        .finish()
}

#[cfg(test)]
mod tests;
