//! The one entry point for every YAML file alz reads (decision-28, spec §7.13). A file goes
//! through [`YamlText::from_bytes`] (1 MiB cap, strict UTF-8, raw control characters), then
//! [`parse`] (tags, documents, aliases, the budgets, decoded control characters, and
//! serde-saphyr into alz's own tree). [`fields`] holds the helpers that schemas read the tree with.

mod error;
mod events;
pub mod fields;
mod load;
mod text;
mod tree;

#[cfg(test)]
mod tests;

pub use error::{YamlError, YamlErrorKind};
pub use load::{parse, parse_paths};
pub use text::{DecodedText, MAX_BYTES, YamlText};
pub use tree::{Mapping, Node, Value};
