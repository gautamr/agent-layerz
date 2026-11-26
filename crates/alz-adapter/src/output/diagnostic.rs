//! A diagnostic of an adapter (spec §8.0 **Output.**).

use crate::source::Source;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// A problem that fails the build.
    Error,
    /// A problem that is reported and does not fail the build.
    Warning,
    /// What an adapter did that is no problem; it never changes the exit code.
    Note,
}

/// A diagnostic: a level, a code, optionally the layer file it is about, and a message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// The level.
    pub level: Level,
    /// The code: a shared adapter code, or one of the adapter's own, `<id>-<word>` (§10.1
    /// **Adapter codes.**). The core refuses any other.
    pub code: String,
    /// The layer file it is about, when it is about one.
    pub file: Option<Source>,
    /// The message. It is untrusted text, which the core strips (class 1 of §10.1).
    pub message: String,
}

impl Diagnostic {
    /// A diagnostic that is about no layer file.
    #[must_use]
    pub fn new(level: Level, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            code: code.into(),
            file: None,
            message: message.into(),
        }
    }

    /// The same diagnostic, about a layer file.
    #[must_use]
    pub fn about(self, file: Source) -> Self {
        Self {
            file: Some(file),
            ..self
        }
    }
}
