//! A value read from a file that alz may ignore in part (spec §7.13 "User config", "Allow state"):
//! what was usable, and one warning for each thing that was not.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Position, Where};
use crate::yaml::{Node, YamlError, YamlText, parse};

/// A value and the warnings its reading raised. A warning is not an error: the file is read as far
/// as it can be, and the problem is reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading<T> {
    value: T,
    warnings: Diagnostics,
}

impl<T> Reading<T> {
    /// A value read without a warning.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            warnings: Diagnostics::new(),
        }
    }

    /// The same value with one more warning, after the ones before it.
    #[must_use]
    pub fn warned(self, warning: Diagnostic) -> Self {
        Self {
            warnings: self.warnings.with(warning),
            ..self
        }
    }

    /// The value read.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The warnings, in the order they were found.
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
    }

    /// The same warnings with the value changed by `change`.
    #[must_use]
    pub fn map<U>(self, change: impl FnOnce(T) -> U) -> Reading<U> {
        Reading {
            value: change(self.value),
            warnings: self.warnings,
        }
    }

    /// The value and the warnings, taken apart.
    #[must_use]
    pub fn into_parts(self) -> (T, Diagnostics) {
        (self.value, self.warnings)
    }

    /// Folds `later` into this reading: `join` combines the values, and the warnings of `later`
    /// come after those of `self`.
    #[must_use]
    pub fn then<U>(self, later: Reading<U>, join: impl FnOnce(T, U) -> T) -> Self {
        Self {
            value: join(self.value, later.value),
            warnings: self.warnings.merge(later.warnings),
        }
    }
}

/// A warning of `code`, placed in `file` at `at`.
pub(super) fn warning(code: Code, file: &Where, at: Position, message: Message) -> Diagnostic {
    Diagnostic::warning(code, Some(file.clone().at(at)), message)
}

/// The warning for a file that is not valid YAML or is over a limit: the file is ignored. It
/// keeps the YAML error's code and its line and column, and holds no input but the one quoted key
/// a duplicate key may carry.
pub(super) fn unreadable(error: &YamlError, file: &Where) -> Diagnostic {
    let at = error
        .position()
        .map_or_else(|| file.clone(), |position| file.clone().at(position));
    Diagnostic::warning(
        error.kind().code(),
        Some(at),
        error.kind().message().then("; the file is ignored"),
    )
}

/// The tree of a file's bytes, through the one YAML entry point.
pub(super) fn tree(bytes: &[u8]) -> Result<Node, YamlError> {
    YamlText::from_bytes(bytes).and_then(|text| parse(&text))
}
