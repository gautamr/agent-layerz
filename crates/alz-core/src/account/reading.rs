//! A value read from a file that alz may ignore in part (spec §7.13 "User config", "Allow state"):
//! what was usable, and one warning for each thing that was not.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::yaml::{Node, YamlError, YamlText, parse, parse_paths};

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

/// A warning of `code` about `file`: `<file>: <message>`. The position in the file is not part of
/// the line (§10.3 names an `allowed.yaml` entry by its number and a `folders` key by its text).
pub(super) fn warning(code: Code, file: &Where, message: Message) -> Diagnostic {
    Diagnostic::warning(code, Some(file.clone()), message)
}

/// A warning whose message names the file itself, so the line holds no place.
pub(super) fn unplaced(code: Code, message: Message) -> Diagnostic {
    Diagnostic::warning(code, None, message)
}

/// The warning for a file that is not valid YAML or is over a limit: the file is ignored. It
/// keeps the YAML error's code and its line and column, and holds no input but the one quoted key
/// a duplicate key may carry.
pub(super) fn unreadable(error: &YamlError, file: &Where) -> Diagnostic {
    let at = error
        .position()
        .map_or_else(|| file.clone(), |position| file.clone().at(position));
    Diagnostic::warning(
        Code::AccountFileIgnored,
        Some(at),
        error.kind().message().then("; the file is ignored"),
    )
}

/// The tree of an account file's bytes, through the one YAML entry point: strict UTF-8, at most
/// 1 MiB and the alias limits (spec §7.13 "YAML limits").
///
/// # Errors
///
/// The YAML error when the bytes are not valid UTF-8, are over a limit or are not valid YAML.
pub fn parse_file(bytes: &[u8]) -> Result<Node, YamlError> {
    tree(bytes)
}

/// The tree of the bytes of a file that holds file-system paths: as [`tree`], and a path may hold
/// the characters that alz writes escaped (U+007F to U+009F, U+2028 and U+2029).
pub(super) fn path_tree(bytes: &[u8]) -> Result<Node, YamlError> {
    YamlText::from_bytes(bytes).and_then(|text| parse_paths(&text))
}

/// The tree of the bytes of a file that holds file-system paths (`allowed.yaml`): as
/// [`parse_file`], and a key or value may hold what a path may hold and the emitter escapes, U+007F
/// to U+009F, U+2028 and U+2029.
///
/// # Errors
///
/// As [`parse_file`].
pub fn parse_path_file(bytes: &[u8]) -> Result<Node, YamlError> {
    path_tree(bytes)
}

/// The tree of a file's bytes, through the one YAML entry point.
pub(super) fn tree(bytes: &[u8]) -> Result<Node, YamlError> {
    YamlText::from_bytes(bytes).and_then(|text| parse(&text))
}
