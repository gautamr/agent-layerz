//! Writing files alz generates: the file tree with its generated header, path globs and protected
//! paths, the lock of a build, the YAML emitter (decision-28) and the stable JSON emitter
//! (decision-33).

pub mod action_word;
pub mod diff;
pub mod gitignore;
pub mod glob;
pub mod header;
pub mod instructions;
pub mod json;
pub mod lock_of;
pub(crate) mod passthrough;
pub mod protected;
pub mod rendered;
pub(crate) mod sources;
pub mod tree;
pub mod write_plan;
pub mod yaml;
