//! The neutral model that the core gives an adapter (spec §8.0 **Model.**): the merged layers for
//! one target, as §7.1 to §7.10 define them.
//!
//! `AGENTS.md` (§7.1, §7.12), passthrough files (§7.11) and accounts (§7.13) stay the core's and
//! are not in the model. Every list is in the order the merged layers give it, which is stack
//! order and then file order, so the same layers always give the same model.
//!
//! The core has checked every value before it builds the model: names, rules, paths and ids are
//! the text that §7 accepts. An adapter may rely on that and does not check it again.

mod agent;
mod hook;
mod items;
mod mode;
mod security;
mod version;

pub use agent::{Agent, AgentTool, Isolation, Memory, ModelProblem};
pub use hook::{Hook, HookEvent, HookMatch, HookRun, HookScript, NeutralEvent, OnFail};
pub use items::{
    Command, McpServer, McpTransport, Models, PathRule, Permissions, Skill, SkillFile, SlotStub,
};
pub use mode::{FloorField, Mode};
pub use security::{
    DefaultModeFloor, DenyRule, FloorFieldItem, LockedHook, SecurityItemId, SecurityItems,
    SettingLayers,
};
pub use version::{MODEL_VERSION, ModelVersion};

use crate::json::JsonObject;
use crate::target_id::TargetId;

/// The model of one target. Its fields are public to read and to set; the struct is
/// `#[non_exhaustive]`, so a minor version of the model can add a field, and a value is made with
/// [`Model::new`] and then filled.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Model {
    /// The target this model is for. A target's own hook events (`<target>:<Event>`) and native
    /// sections are already narrowed to it.
    pub target: TargetId,
    /// The agents for the target, with the target's native fields (§7.2).
    pub agents: Vec<Agent>,
    /// The skills (§7.3).
    pub skills: Vec<Skill>,
    /// The commands (§7.4).
    pub commands: Vec<Command>,
    /// The slot stubs of skills that no layer defines (§7.3).
    pub slot_stubs: Vec<SlotStub>,
    /// The path-scoped rules (§7.10).
    pub rules: Vec<PathRule>,
    /// The hooks that run on the target, with their scripts (§7.5).
    pub hooks: Vec<Hook>,
    /// The merged permissions (§7.6).
    pub permissions: Permissions,
    /// The enabled MCP servers, in id order (§7.7).
    pub mcp_servers: Vec<McpServer>,
    /// The models resolved for the target (§7.8).
    pub models: Models,
    /// The target's own native settings (§7.9), the merged section of `settings.yaml` for this
    /// target.
    pub settings: JsonObject,
    /// The security items of the model (§8.0 **Model.**).
    pub security: SecurityItems,
}

impl Model {
    /// An empty model for `target`.
    #[must_use]
    pub fn new(target: TargetId) -> Self {
        Self {
            target,
            agents: Vec::new(),
            skills: Vec::new(),
            commands: Vec::new(),
            slot_stubs: Vec::new(),
            rules: Vec::new(),
            hooks: Vec::new(),
            permissions: Permissions::default(),
            mcp_servers: Vec::new(),
            models: Models::default(),
            settings: JsonObject::default(),
            security: SecurityItems::default(),
        }
    }

    /// The version of the model: [`MODEL_VERSION`].
    #[must_use]
    pub const fn version(&self) -> ModelVersion {
        MODEL_VERSION
    }
}
