//! Why a JSON file is invalid, as a value (spec §5.3 "Reading the lock"). An error names its kind
//! and, when it can be placed, a line and a column. It holds no input text except the quoted key
//! of a duplicate; `serde_json`'s own message is never read (decision-33).

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Code, Diagnostic, Message, Position, Where};
use crate::untrusted_text::QuotedKey;

/// The kind of problem. Each kind has a fixed message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// The file is larger than 1 MiB.
    TooLarge,
    /// The file is not valid UTF-8.
    InvalidUtf8,
    /// Text that is not one JSON document, or one nested more than 128 levels deep.
    Syntax,
    /// Two equal keys in one object.
    DuplicateKey(QuotedKey),
}

impl JsonErrorKind {
    /// The message of the kind, built from literals and the one quoted key.
    #[must_use]
    pub fn message(&self) -> Message {
        match self {
            Self::TooLarge => Message::new("the file is larger than 1 MiB"),
            Self::InvalidUtf8 => Message::new("the file is not valid UTF-8"),
            Self::Syntax => {
                Message::new("the file is not one JSON document nested at most 128 levels deep")
            }
            Self::DuplicateKey(key) => Message::new("duplicate key ").then(key.clone()),
        }
    }
}

impl fmt::Display for JsonErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message().fmt(formatter)
    }
}

/// A JSON file that is invalid: the kind of problem and, when it can be placed, where.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{kind}")]
pub struct JsonError {
    kind: JsonErrorKind,
    at: Option<Position>,
}

impl JsonError {
    /// An error of `kind` at `at`.
    #[must_use]
    pub const fn new(kind: JsonErrorKind, at: Option<Position>) -> Self {
        Self { kind, at }
    }

    /// The kind of problem.
    #[must_use]
    pub const fn kind(&self) -> &JsonErrorKind {
        &self.kind
    }

    /// Where the problem is, when it can be placed.
    #[must_use]
    pub const fn position(&self) -> Option<Position> {
        self.at
    }

    /// The error as a diagnostic with `code` in `file`, to which the error adds its own line and
    /// column.
    #[must_use]
    pub fn into_diagnostic(self, code: Code, file: Where) -> Diagnostic {
        let at = self.at.map_or(file.clone(), |position| file.at(position));
        Diagnostic::error(code, Some(at), self.kind.message())
    }
}
