//! The adapter contract of agent-layerz (spec §8.0, decision-40).
//!
//! An adapter maps the neutral [`Model`] of one target onto native files. It is a pure function:
//! from the model and a [`Context`] it returns an [`Output`] and does nothing else. This crate
//! holds the data types of the contract and the [`Adapter`] trait, and nothing that acts. It
//! depends on no other alz crate, so an adapter depends on it and never on `alz-core`, and its
//! `clippy.toml` bans the file system, process and environment functions of the standard library.
//!
//! The model has a version, `major.minor` ([`MODEL_VERSION`]). A minor version only adds to the
//! model; a major version may change or remove.

#![forbid(unsafe_code)]

mod adapter;
mod context;
mod descriptor;
mod json;
mod layer_name;
mod model;
mod number;
mod output;
mod source;
mod target_id;
mod toml;

pub use adapter::Adapter;
pub use context::Context;
pub use descriptor::{
    AgentField, Descriptor, FirstClassKey, FloorKey, FloorRule, NativeVocabulary, ReservedKey,
    SettingsKey, Territory, ToolVersion, ToolVersionRange, ValueKind,
};
pub use json::{DuplicateKey, JsonObject, JsonValue};
pub use layer_name::{LayerName, LayerNameError};
pub use model::{
    Agent, AgentTool, Command, DefaultModeFloor, DenyRule, FloorField, FloorFieldItem, Hook,
    HookEvent, HookMatch, HookRun, HookScript, Isolation, LockedHook, MODEL_VERSION, McpServer,
    McpTransport, Memory, Mode, Model, ModelProblem, ModelVersion, Models, NeutralEvent, OnFail,
    PathRule, Permissions, SecurityItemId, SecurityItems, SettingLayers, Skill, SkillFile,
    SlotStub, TargetInstruction,
};
pub use number::{Finite, NotFinite};
pub use output::{
    Bytes, Content, Coverage, CoverageEntry, Diagnostic, Enforcement, Frontmatter, HeaderSource,
    Level, Markdown, Output, OutputFile,
};
pub use source::Source;
pub use target_id::{TargetId, TargetIdError};
pub use toml::TomlValue;
