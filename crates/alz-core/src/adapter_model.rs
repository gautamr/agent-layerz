//! The merged stack as the neutral model of the adapter contract (spec §8.0 **Model.**, decision-40).
//!
//! [`model_for`] is a pure function from a [`Merged`] stack and a target to the [`Model`] that an
//! adapter reads. The merge types stay the core's: an adapter reads only the model, so they can
//! change without breaking one. Each file converts one kind of §7:
//!
//! | File | Kinds |
//! | --- | --- |
//! | `agents` | agents (§7.2) with their native fields and the routing of their model (§7.8) |
//! | `skills` | skills, commands and slot stubs (§7.3, §7.4) |
//! | `rules` | path-scoped rules (§7.10) |
//! | `hooks` | hooks with their scripts (§7.5) |
//! | `permissions` | permissions (§7.6) |
//! | `mcp` | the enabled MCP servers (§7.7) |
//! | `models` | the routing resolved for the target (§7.8) |
//! | `settings` | the target's native settings (§7.9) |
//! | `security` | the four kinds of security item (§8.0) |
//!
//! `AGENTS.md`, passthrough files and accounts are not in the model; the core writes them.

mod agents;
mod hooks;
mod json;
mod mcp;
mod models;
mod permissions;
mod rules;
mod security;
mod settings;
mod skills;
mod sources;

use alz_adapter::{Model, TargetId};

use crate::merge::Merged;

pub(crate) use permissions::mode as contract_mode;

/// The model of `merged` for `target`: what an adapter for that target reads.
///
/// The agents and hooks that do not run on `target` are left out, and so are the native fields and
/// native hook events of other targets. Every list is in the order the merge keeps, so the same
/// stack always gives the same model.
#[must_use]
pub fn model_for(merged: &Merged, target: &TargetId) -> Model {
    let mut model = Model::new(target.clone());
    // `Model` is `#[non_exhaustive]`, so a foreign crate fills a new value field by field.
    model.agents = agents::of(merged, target);
    model.skills = skills::skills(merged);
    model.commands = skills::commands(merged);
    model.slot_stubs = skills::slot_stubs(merged);
    model.rules = rules::of(merged);
    model.hooks = hooks::of(merged, target);
    model.permissions = permissions::of(merged);
    model.mcp_servers = mcp::of(merged);
    model.models = models::of(merged, target);
    model.settings = settings::of(merged, target);
    model.security = security::of(merged, target);
    model
}

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;
