//! What an adapter returns (spec §8.0 **Output.**): files, diagnostics and a coverage report.
//!
//! Everything here is untrusted by the core until it has checked it (§8.0 **Output checks.**): a
//! path is text, not a safe relative path, and a diagnostic code is text, not a code of the
//! tables. The adapter has no way to write or change a header.

mod content;
mod coverage;
mod diagnostic;

pub use content::{Bytes, Content, Frontmatter, HeaderSource, Markdown};
pub use coverage::{Coverage, CoverageEntry, Enforcement};
pub use diagnostic::{Diagnostic, Level};

/// One file of an adapter's output.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OutputFile {
    /// The path relative to the repository. The core refuses a path that is not a safe relative
    /// path, or that lies outside the adapter's territory.
    pub path: String,
    /// What the file holds.
    pub content: Content,
}

impl OutputFile {
    /// A file at `path`.
    #[must_use]
    pub fn new(path: impl Into<String>, content: Content) -> Self {
        Self {
            path: path.into(),
            content,
        }
    }
}

/// The output of one rendering: files, diagnostics and the coverage report.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Output {
    /// The files, in the order the adapter chose; the core orders what it writes.
    pub files: Vec<OutputFile>,
    /// The diagnostics. An error fails the build like any other error.
    pub diagnostics: Vec<Diagnostic>,
    /// How the output enforces each security item of the model.
    pub coverage: Coverage,
}

impl Output {
    /// An output.
    #[must_use]
    pub const fn new(
        files: Vec<OutputFile>,
        diagnostics: Vec<Diagnostic>,
        coverage: Coverage,
    ) -> Self {
        Self {
            files,
            diagnostics,
            coverage,
        }
    }
}
