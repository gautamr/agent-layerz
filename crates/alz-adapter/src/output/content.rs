//! The four kinds of content of an output file (spec §8.0 **Output.**).

use crate::json::{JsonObject, JsonValue};
use crate::source::Source;
use crate::toml::TomlValue;

/// The frontmatter of a Markdown file.
#[derive(Debug, Clone, PartialEq)]
pub enum Frontmatter {
    /// A YAML mapping, which the core writes with the emitter of §10.2.
    Mapping(JsonObject),
    /// The frontmatter text of a layer file, kept byte for byte (a skill's `SKILL.md`, §8.1): the
    /// lines between the opening and the closing `---`.
    Kept(String),
}

/// One source of a generated header (§4.3): what the file is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderSource {
    /// A file of a layer.
    LayerFile(Source),
    /// Another generated file of the repository, as `AGENTS.md` for the header of `CLAUDE.md`:
    /// a path relative to the repository.
    GeneratedFile(String),
    /// A slot stub, which has no file (§7.3).
    SlotStub,
}

impl From<Source> for HeaderSource {
    fn from(source: Source) -> Self {
        Self::LayerFile(source)
    }
}

/// A Markdown file: a body, an optional frontmatter, and the sources of its header.
///
/// The core writes the file from these parts and puts the header in (§4.3); there is no field in
/// which an adapter could write or change it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Markdown {
    /// The frontmatter, when the file has one.
    pub frontmatter: Option<Frontmatter>,
    /// The text after the frontmatter. With a [`Frontmatter::Mapping`] or no frontmatter, the text
    /// without its final line feed, which the core adds; with [`Frontmatter::Kept`], the text of
    /// the layer file after its closing `---` line, byte for byte.
    pub body: String,
    /// The sources the core writes the header from (§4.3): an HTML comment line first when there
    /// is no frontmatter, and a YAML comment line right after the opening `---` when there is.
    pub sources: Vec<HeaderSource>,
}

impl Markdown {
    /// A Markdown file without frontmatter and without sources.
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            frontmatter: None,
            body: body.into(),
            sources: Vec::new(),
        }
    }

    /// The same file with a frontmatter.
    #[must_use]
    pub fn with_frontmatter(self, frontmatter: Frontmatter) -> Self {
        Self {
            frontmatter: Some(frontmatter),
            ..self
        }
    }

    /// The same file with the sources of its header.
    #[must_use]
    pub fn with_sources(self, sources: Vec<Source>) -> Self {
        Self {
            sources: sources.into_iter().map(HeaderSource::from).collect(),
            ..self
        }
    }

    /// The same file with these sources of its header, which may also name generated files and
    /// the slot stub.
    #[must_use]
    pub fn with_header_sources(self, sources: Vec<HeaderSource>) -> Self {
        Self { sources, ..self }
    }
}

/// Bytes kept as they are, with an executable bit, and no header (copied skill files, hook
/// scripts).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Bytes {
    /// The bytes.
    pub content: Vec<u8>,
    /// Whether the file is executable.
    pub executable: bool,
}

impl Bytes {
    /// Bytes that are not executable.
    #[must_use]
    pub const fn new(content: Vec<u8>) -> Self {
        Self {
            content,
            executable: false,
        }
    }

    /// The same bytes with the executable bit set as given.
    #[must_use]
    pub fn executable(self, executable: bool) -> Self {
        Self { executable, ..self }
    }
}

/// What an output file holds: one of four kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// A JSON value, written by the core's stable JSON emitter with keys in name order (§6); no
    /// header.
    Json(JsonValue),
    /// A TOML value. The core's TOML emitter and the header of a TOML file come with the first
    /// adapter that writes TOML (TASK-126).
    Toml(TomlValue),
    /// Markdown, to which the core writes the header.
    Markdown(Markdown),
    /// Bytes as they are, with an executable bit; no header.
    Bytes(Bytes),
}
