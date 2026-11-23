//! The `budgets` of the stack manifest (spec §5.2): whole numbers, 0 or more.

use thiserror::Error;

use crate::schema::{Checked, Refusal, SchemaError, SchemaErrors, unknown_fields, zip, zip3};
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};

/// Codex stops reading `AGENTS.md` at 32 KiB (`project_doc_max_bytes`).
const DEFAULT_MAX_BYTES: u64 = 32 * 1024;

/// A budget that is a whole number but outside 0 to 2^64 - 1. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a budget is a whole number from 0 to 18446744073709551615")]
pub struct BudgetError;

impl BudgetError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be a whole number from 0 to 18446744073709551615"
    }
}

/// The limits a build holds the composed output to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    instructions_max_lines: Option<u64>,
    instructions_max_bytes: u64,
    always_loaded_skills_max: Option<u64>,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            instructions_max_lines: None,
            instructions_max_bytes: DEFAULT_MAX_BYTES,
            always_loaded_skills_max: None,
        }
    }
}

/// The field as a budget, `None` when it is absent.
fn budget(section: &Section<'_>, field: &'static str) -> Checked<Option<u64>> {
    section
        .whole_number(field)?
        .map(|number| {
            u64::try_from(number).map_err(|_out_of_range| {
                SchemaErrors::from(SchemaError::refused(
                    section.field_path(field),
                    Refusal::Budget(BudgetError),
                    section.node(field).map_or(section.at(), Node::at),
                ))
            })
        })
        .transpose()
}

impl Budgets {
    /// Reads the `budgets` mapping at `node`.
    ///
    /// # Errors
    ///
    /// Every problem of the mapping: an unknown field, a value that is not a whole number, or one
    /// outside 0 to 2^64 - 1.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path)?;
        zip(
            unknown_fields(
                &section,
                &[
                    "instructions_max_lines",
                    "instructions_max_bytes",
                    "always_loaded_skills_max",
                ],
            ),
            zip3(
                budget(&section, "instructions_max_lines"),
                budget(&section, "instructions_max_bytes"),
                budget(&section, "always_loaded_skills_max"),
            ),
        )
        .map(|((), (lines, bytes, skills))| Self {
            instructions_max_lines: lines,
            instructions_max_bytes: bytes.unwrap_or(DEFAULT_MAX_BYTES),
            always_loaded_skills_max: skills,
        })
    }

    /// The most lines the composed `AGENTS.md` may have, if limited.
    #[must_use]
    pub const fn instructions_max_lines(&self) -> Option<u64> {
        self.instructions_max_lines
    }

    /// The most bytes the composed `AGENTS.md` may have: 32768 when the manifest says nothing.
    #[must_use]
    pub const fn instructions_max_bytes(&self) -> u64 {
        self.instructions_max_bytes
    }

    /// The most skills preloaded into the main session, if limited.
    #[must_use]
    pub const fn always_loaded_skills_max(&self) -> Option<u64> {
        self.always_loaded_skills_max
    }
}
