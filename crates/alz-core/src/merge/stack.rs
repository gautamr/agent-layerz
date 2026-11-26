//! The fold of a resolved stack into the merged model: a left fold over the layers in stack
//! order, most general first, where each kind of item of §7 absorbs one layer at a time.
//!
//! The typestate is `ResolvedStack` to [`Merging`] to [`Merged`]. [`Merged`] has private fields
//! and no public constructor, so the renderer can take only a stack that was merged. Every kind
//! is merged and every problem reported in one run; [`Merging::merged`] is `None` when any error
//! was found.
//!
//! Who may declare `commit_identity` and `account` (§7.12, §7.13) is not a kind of item: the two
//! fields of `layer.yaml` follow one rule, in `declarers`, which the fold feeds from each layer's
//! position and [`Placement`] and which fills [`Merged::commit_identity`] and [`Merged::account`].
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

use crate::account::AccountDeclaration;
use crate::diagnostic::Diagnostics;
use crate::identity::CommitIdentity;
use crate::item::{AgentName, Hook, RuleName, SettingsTree, Skill, SkillName};
use crate::layer::{LayerMeta, LayerName};
use crate::merge::agents::{AgentState, MergedAgent};
use crate::merge::algebra::Replace;
use crate::merge::declarers::{Declarations, DeclaredField, DeclarerState};
use crate::merge::hooks::HookState;
use crate::merge::instructions::{
    InstructionBlock, InstructionState, MergedClaudeExtra, MergedInstructions,
};
use crate::merge::mcp::{McpState, MergedMcp};
use crate::merge::models::{MergedModels, ModelState};
use crate::merge::passthrough::{MergedPassthrough, PassthroughState};
use crate::merge::permissions::{MergedPermissions, PermissionState};
use crate::merge::placement::{Placement, Placements};
use crate::merge::provenance::Traced;
use crate::merge::rules::{MergedRule, RuleState};
use crate::merge::settings::SettingsState;
use crate::merge::skills::{MergedSkills, SkillState, SlotDemand};
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack};
use crate::target::Registry;
use alz_adapter::TargetId;

/// What a kind needs to absorb one layer: the layer, where it lies in the stack and where the layer
/// lies in relation to the repository.
#[derive(Debug, Clone, Copy)]
pub struct LayerContext<'a> {
    layer: &'a ResolvedLayer,
    index: usize,
    placement: Placement,
    registry: &'a Registry,
}

impl<'a> LayerContext<'a> {
    /// The layer, with its description and the bytes of its files.
    #[must_use]
    pub const fn layer(&self) -> &'a ResolvedLayer {
        self.layer
    }

    /// The first layer of a stack of one, for the checks of `alz lint` that read a layer alone
    /// (§10). Its placement is [`Placement::Inside`], the stricter answer.
    pub(super) const fn alone(layer: &'a ResolvedLayer, registry: &'a Registry) -> Self {
        Self {
            layer,
            index: 0,
            placement: Placement::Inside,
            registry,
        }
    }

    /// The adapters of the build, whose descriptors state the key rules of each target's native
    /// settings (§7.9).
    #[must_use]
    pub const fn registry(&self) -> &'a Registry {
        self.registry
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
    claude_extras: Vec<MergedClaudeExtra>,
    rules: BTreeMap<RuleName, Traced<MergedRule>>,
    agents: BTreeMap<AgentName, Traced<MergedAgent>>,
    skills: Replace<SkillName, Traced<Skill>>,
    slot_stubs: BTreeMap<SkillName, SlotDemand>,
    hooks: Vec<Traced<Hook>>,
    permissions: MergedPermissions,
    settings: BTreeMap<TargetId, Traced<SettingsTree>>,
    mcp: MergedMcp,
    models: MergedModels,
    passthrough: MergedPassthrough,
    commit_identity: Declarations<CommitIdentity>,
    account: Declarations<AccountDeclaration>,
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
    pub fn claude_extras(&self) -> &[MergedClaudeExtra] {
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

    /// The skills and commands by name (§7.3, §7.4), each with the layers that defined it. A name
    /// is one skill's whole directory or one command, from the most specific layer that has it;
    /// the earlier definitions are in the provenance only.
    #[must_use]
    pub const fn skills(&self) -> &Replace<SkillName, Traced<Skill>> {
        &self.skills
    }

    /// The hooks that stand, in stack order and then file order (§7.5), each with the layer that
    /// defined it. A hook that a layer disabled is not here; a locked hook is always here.
    #[must_use]
    pub fn hooks(&self) -> &[Traced<Hook>] {
        &self.hooks
    }

    /// The permissions (§7.6): the `allow`, `ask` and `deny` rules combined across layers, and the
    /// `default_mode`. Every deny rule of every layer is in it.
    #[must_use]
    pub const fn permissions(&self) -> &MergedPermissions {
        &self.permissions
    }

    /// The settings of each target, deep-merged across layers (§7.9), each with the layers that
    /// set them. A target no layer set has no entry, and no other key can be there.
    #[must_use]
    pub const fn settings(&self) -> &BTreeMap<TargetId, Traced<SettingsTree>> {
        &self.settings
    }

    /// The MCP servers (§7.7): every definition by id, and the ids of `mcp.enable` that are on,
    /// in order. Only the enabled servers are emitted.
    #[must_use]
    pub const fn mcp(&self) -> &MergedMcp {
        &self.mcp
    }

    /// The model routing (§7.8): aliases, roles and the default alias, each with the layers that
    /// set it.
    #[must_use]
    pub const fn models(&self) -> &MergedModels {
        &self.models
    }

    /// The passthrough files of each target (§7.11), by their path inside the target's folder,
    /// each with the layers that had the path. The most specific layer's file is the one here; the
    /// earlier ones are in the provenance only.
    #[must_use]
    pub const fn passthrough(&self) -> &MergedPassthrough {
        &self.passthrough
    }

    /// Who declared the commit identity (§7.12): the winner, which layer R6 names in the Commits
    /// section and the reports, and the layers that were refused.
    #[must_use]
    pub const fn commit_identity(&self) -> &Declarations<CommitIdentity> {
        &self.commit_identity
    }

    /// Who declared the account (§7.13): the winner, and the layers that were refused.
    #[must_use]
    pub const fn account(&self) -> &Declarations<AccountDeclaration> {
        &self.account
    }

    /// The slots that no layer filled, when `strict_slots` is false: each needs a stub skill
    /// that says "not defined for this repo", and who asked for it first (§7.3).
    #[must_use]
    pub const fn slot_stubs(&self) -> &BTreeMap<SkillName, SlotDemand> {
        &self.slot_stubs
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
    skills: SkillState,
    hooks: HookState,
    permissions: PermissionState,
    settings: SettingsState,
    mcp: McpState,
    models: ModelState,
    passthrough: PassthroughState,
    commit_identity: DeclarerState<CommitIdentity>,
    account: DeclarerState<AccountDeclaration>,
    diagnostics: Diagnostics,
}

impl Accumulator {
    /// The state after one more, more specific, layer. Each kind's `absorb` is called here, in
    /// the order of §7.
    fn absorb(self, context: &LayerContext<'_>) -> Self {
        let (instructions, found_instructions) = self.instructions.absorb(context);
        let (rules, found_rules) = self.rules.absorb(context);
        let (agents, found_agents) = self
            .agents
            .absorb_under(context, self.permissions.declared_floor().as_ref());
        let (skills, found_skills) = self.skills.absorb(context);
        let (hooks, found_hooks) = self.hooks.absorb(context);
        let (permissions, found_permissions) = self.permissions.absorb(context);
        let (settings, found_settings) = self.settings.absorb(context);
        let (mcp, found_mcp) = self.mcp.absorb(context);
        let (models, found_models) = self.models.absorb(context);
        let (passthrough, found_passthrough) = self.passthrough.absorb_deferring(context);
        let commit_identity = self
            .commit_identity
            .absorb(context, LayerMeta::commit_identity);
        let account = self.account.absorb(context, LayerMeta::account);
        Self {
            layers: self
                .layers
                .into_iter()
                .chain([context.layer().meta().name().clone()])
                .collect(),
            instructions,
            rules,
            agents,
            skills,
            hooks,
            permissions,
            settings,
            mcp,
            models,
            passthrough,
            commit_identity,
            account,
            diagnostics: self
                .diagnostics
                .merge(found_instructions)
                .merge(found_rules)
                .merge(found_agents)
                .merge(found_skills)
                .merge(found_hooks)
                .merge(found_permissions)
                .merge(found_settings)
                .merge(found_mcp)
                .merge(found_models)
                .merge(found_passthrough),
        }
    }

    fn finish(self, manifest: &Manifest) -> Merging {
        let MergedInstructions {
            blocks,
            claude_extras,
        } = self.instructions.finish();
        let agents = self.agents.finish();
        let (MergedSkills { items, stubs }, found_slots) = self
            .skills
            .finish(&agents, manifest.options().strict_slots());
        let (mcp, found_mcp) = self.mcp.finish(manifest);
        let (models, found_unresolved) = self.models.finish();
        let shadowed = self.permissions.shadowed();
        let (passthrough, found_passthrough) = self
            .passthrough
            .finish(|target| manifest.targets().contains(target));
        let (commit_identity, found_identity) =
            self.commit_identity.finish(DeclaredField::CommitIdentity);
        let (account, found_account) = self.account.finish(DeclaredField::Account);
        Merging::of(
            Merged {
                layers: self.layers,
                instructions: blocks,
                claude_extras,
                rules: self.rules.finish(),
                agents,
                skills: items,
                slot_stubs: stubs,
                hooks: self.hooks.finish(),
                permissions: self.permissions.finish(),
                settings: self.settings.finish(),
                mcp,
                models,
                passthrough,
                commit_identity,
                account,
            },
            self.diagnostics
                .merge(found_slots)
                .merge(shadowed)
                .merge(found_mcp)
                .merge(found_unresolved)
                .merge(found_identity)
                .merge(found_account)
                .merge(found_passthrough),
        )
    }
}

/// Merges the layers of `stack` (§7), with the key rules that the adapters of `registry` state
/// for each target's native settings (§7.9). Pure: the same input gives an equal result, because
/// every collection is ordered and the fold runs in stack order.
#[must_use]
pub fn merge(
    stack: &ResolvedStack,
    manifest: &Manifest,
    placements: &Placements,
    registry: &Registry,
) -> Merging {
    stack
        .layers()
        .iter()
        .enumerate()
        .map(|(index, layer)| LayerContext {
            layer,
            index,
            placement: placements.of(layer.meta().name()),
            registry,
        })
        .fold(Accumulator::default(), |state, context| {
            state.absorb(&context)
        })
        .finish(manifest)
}

#[cfg(test)]
mod tests;
