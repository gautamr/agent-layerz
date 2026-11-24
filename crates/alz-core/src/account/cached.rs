//! Reading a stack's account declaration from the layer cache, without git (spec §7.13 "No fetch
//! on resolve"; decision-18; TASK-55.7.8). This module holds the decisions and no effect: which
//! cache miss a fact is ([`MissReason`]), which lock entry and which cache entry a layer is read
//! from ([`lock_entry`], [`read_plan`], [`IndexEntry`]) and what the layers declare
//! ([`decide`]). `alz-io` finds the facts.

mod decision;
mod miss;
mod plan;
#[cfg(test)]
mod tests;

pub use decision::{CachedDeclaration, CachedLayer, DeclaredAccount, Ignored, decide};
pub use miss::{Miss, MissReason, lock_miss};
pub use plan::{IndexEntry, ReadPlan, has_parent_segment, lock_entry, read_plan};
