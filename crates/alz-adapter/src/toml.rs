//! The TOML value of the contract, for a `Toml` file (spec §8.0 **Output.**). It is the value,
//! not its text: the core's TOML emitter writes it, and that comes with the first adapter that
//! writes TOML (TASK-126).

use std::collections::BTreeMap;

use crate::number::Finite;

/// One TOML value. A table is ordered by key, so the same value always has the same text.
#[derive(Debug, Clone, PartialEq)]
pub enum TomlValue {
    /// Text.
    Text(String),
    /// A whole number of 64 bits, the range TOML allows.
    Integer(i64),
    /// A finite float.
    Float(Finite),
    /// `true` or `false`.
    Boolean(bool),
    /// An array.
    Array(Vec<TomlValue>),
    /// A table.
    Table(BTreeMap<String, TomlValue>),
}
