//! The merge of a resolved stack (spec §7): the structures that combine values, and the fold of
//! the stack into one [`Merged`] model.
//!
//! The merge is pure: it reads the bytes the layer walk kept in each layer's contents and never
//! the file system. It follows the general rule of §7, "the more specific layer wins on
//! conflicts, collections are combined, and security can only get stricter", and each part of
//! that rule is one structure of [`algebra`] with its laws stated as property tests:
//!
//! | Rule | Structure |
//! | --- | --- |
//! | the more specific value wins | [`Last`], and [`Replace`] per key |
//! | collections are combined | [`OrderedSet`], [`ListEdit`] |
//! | settings are deep-merged | [`Tree::deep_merge`] |
//! | security only gets stricter | [`Floor`] |
//! | an overlay may only narrow a tool | [`covers`] |
//!
//! The fold lives in [`merge`]. Each kind of item (instructions, agents, skills, and the rest of
//! §7) adds its own state to the fold and its own field to [`Merged`]; see the module docs of
//! `stack` for the places to add them.

mod agents;
pub mod algebra;
mod instructions;
mod placement;
mod provenance;
mod rules;
mod skills;
mod stack;
#[cfg(test)]
pub(crate) mod testing;

pub use agents::{AgentBody, ContextSection, MergedAgent};
pub use algebra::{FloatBits, Floor, Last, ListEdit, OrderedSet, Replace, Tree, covers, widened};
pub use instructions::InstructionBlock;
pub use placement::{Placement, Placements};
pub use provenance::{Action, Contribution, Provenance, Traced};
pub use rules::{MergedRule, RuleChain, RuleSection};
pub use skills::SlotDemand;
pub use stack::{LayerContext, Merged, Merging, merge};
