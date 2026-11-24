//! The pure domain of agent-layerz.
//!
//! This crate performs no input or output: `clippy.toml` bans the file system, process and
//! environment functions of the standard library (decision-23), and the crate does not depend
//! on `alz-io`.

pub mod account;
pub mod cache;
mod diagnostic;
pub mod env;
pub mod git;
pub mod identity;
pub mod item;
pub mod json;
pub mod layer;
pub mod leak_guard;
pub mod lint;
pub mod lock;
pub mod markdown;
pub mod merge;
pub mod path;
pub mod render;
pub mod scaffold;
pub mod schema;
pub mod secret;
pub mod stack;
pub mod target;
mod untrusted_text;
pub mod yaml;

pub use diagnostic::{
    Code, Diagnostic, Diagnostics, Level, Message, MessagePart, Place, Position, Printable, Where,
};

pub use untrusted_text::{Escaped, QuotedKey, Stripped, has_control_or_format};
