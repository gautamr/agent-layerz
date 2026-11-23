//! JSON as alz reads it (decision-33, spec §5.3 "Reading the lock"): at most 1 MiB of strict
//! UTF-8, nested at most 128 levels deep, a duplicate key an error, read into alz's own tree.
//! [`parse`] is the one entry point; writing JSON is `render::json`.

mod error;
mod load;
mod tree;

pub use error::{JsonError, JsonErrorKind};
pub use load::{MAX_BYTES, MAX_DEPTH, parse};
pub use tree::{DuplicateKey, JsonNumber, JsonObject, JsonValue};

#[cfg(test)]
mod tests;
