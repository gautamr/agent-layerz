//! A Markdown file of a layer: YAML frontmatter and a body (spec §4.1, §7.13 "YAML limits").
//!
//! The file is parsed once, here, into a [`MarkdownDoc`]. The block between the two `---` lines
//! goes through [`crate::yaml::parse`], the one entry point of every YAML file, so a block obeys
//! the limits, the tag rule and the control-character rule of any other YAML file; the 1 MiB cap
//! counts the whole file that holds the block (§7.13).

use thiserror::Error;

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::yaml::{self, Mapping, Node, Value, YamlError, YamlText};

/// The first line of a file with frontmatter, after line endings are normalised.
const OPENING: &str = "---\n";

/// The line that closes the block.
const CLOSING: &str = "---";

/// A Markdown file: its frontmatter and its body.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownDoc {
    front: Frontmatter,
    body: Body,
}

/// The mapping of a file's frontmatter: empty for a file without a block or with an empty one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frontmatter(Mapping);

/// The text of a file after its frontmatter block, with every CRLF written as LF.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Body(String);

/// Why a Markdown file is invalid. The error holds no text of the file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MarkdownError {
    /// A file without frontmatter whose bytes are not valid UTF-8.
    #[error("the file is not valid UTF-8")]
    NotUtf8,
    /// The file opens a block and never closes it with a line of `---`.
    #[error("frontmatter is not closed with '---'")]
    NotClosed,
    /// The block holds YAML that is not a mapping, such as a list or a scalar.
    #[error("frontmatter must be a YAML mapping")]
    NotMapping(Option<crate::Position>),
    /// The file or its block breaks a rule of every YAML file (§7.13).
    #[error(transparent)]
    Yaml(#[from] YamlError),
}

impl MarkdownError {
    /// The error as a diagnostic about `file`: the code `frontmatter` for a block that is not
    /// closed or not a mapping, and the code of the YAML error for a YAML problem (`yaml` or
    /// `yaml-tag`), at the line and column of the file.
    #[must_use]
    pub fn into_diagnostic(self, file: Where) -> Diagnostic {
        match self {
            Self::Yaml(error) => error.into_diagnostic(file),
            Self::NotMapping(Some(at)) => {
                Diagnostic::error(Code::Frontmatter, Some(file.at(at)), self.message())
            }
            Self::NotUtf8 | Self::NotClosed | Self::NotMapping(None) => {
                Diagnostic::error(Code::Frontmatter, Some(file), self.message())
            }
        }
    }

    fn message(&self) -> Message {
        Message::new(match self {
            Self::NotUtf8 => "the file is not valid UTF-8",
            Self::NotClosed => "frontmatter is not closed with '---'",
            Self::NotMapping(_) => "frontmatter must be a YAML mapping",
            Self::Yaml(_) => "the file is not valid YAML",
        })
    }
}

impl MarkdownDoc {
    /// Splits a file into frontmatter and body.
    ///
    /// A file that does not start with `---` and a line feed has an empty mapping and the whole
    /// text as its body. Otherwise the block runs to the first line that is exactly `---`; what
    /// follows that line is the body. CRLF counts as LF everywhere. An empty block is an empty
    /// mapping.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::NotClosed`] and [`MarkdownError::NotMapping`] for the two block faults,
    /// [`MarkdownError::Yaml`] when the file or the block breaks a YAML limit or rule, and
    /// [`MarkdownError::NotUtf8`] for a file without a block that is not UTF-8.
    pub fn parse(bytes: &[u8]) -> Result<Self, MarkdownError> {
        if has_opening(bytes) {
            with_block(bytes)
        } else {
            std::str::from_utf8(bytes)
                .map_err(|_not_utf8| MarkdownError::NotUtf8)
                .map(|text| Self {
                    front: Frontmatter::default(),
                    body: Body(lf(text)),
                })
        }
    }

    /// The frontmatter.
    #[must_use]
    pub const fn front(&self) -> &Frontmatter {
        &self.front
    }

    /// The body.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// The frontmatter and the body, for a caller that takes the document apart.
    #[must_use]
    pub fn into_parts(self) -> (Frontmatter, Body) {
        (self.front, self.body)
    }
}

impl Frontmatter {
    /// The mapping.
    #[must_use]
    pub const fn mapping(&self) -> &Mapping {
        &self.0
    }

    /// The value of the entry whose key is the text `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.0.get(key)
    }

    /// Whether there is no entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Body {
    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn has_opening(bytes: &[u8]) -> bool {
    bytes.starts_with(b"---\n") || bytes.starts_with(b"---\r\n")
}

fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// A file that opens a block: the whole file is YAML text first (cap, UTF-8, control characters),
/// then it is split into the block and the body.
fn with_block(bytes: &[u8]) -> Result<MarkdownDoc, MarkdownError> {
    let whole = YamlText::from_bytes(bytes)?;
    let text = lf(whole.as_str());
    let (block, body) = text
        .strip_prefix(OPENING)
        .and_then(split_at_closing)
        .ok_or(MarkdownError::NotClosed)?;
    // The block is parsed after a line feed that stands for the opening line, so every position
    // in the tree and in an error is a line and column of the file itself.
    let node = YamlText::from_bytes(format!("\n{block}").as_bytes())
        .and_then(|yaml_text| yaml::parse(&yaml_text))?;
    Ok(MarkdownDoc {
        front: frontmatter_of(node)?,
        body: Body(body.to_owned()),
    })
}

/// The block and the body of the text after the opening line, split at the first line that is
/// exactly `---`; `None` when no line is.
fn split_at_closing(rest: &str) -> Option<(&str, &str)> {
    let line_starts = std::iter::once(0).chain(rest.match_indices('\n').map(|(at, _)| at + 1));
    line_starts.into_iter().find_map(|start| {
        let tail = rest.get(start..)?;
        let line = tail.split('\n').next()?;
        let after = tail.get(line.len()..)?;
        (line == CLOSING).then_some(())?;
        Some((
            rest.get(..start)?,
            after.strip_prefix('\n').unwrap_or(after),
        ))
    })
}

fn frontmatter_of(node: Node) -> Result<Frontmatter, MarkdownError> {
    match node.into_parts() {
        (Value::Map(mapping), _) => Ok(Frontmatter(mapping)),
        (Value::Null, _) => Ok(Frontmatter::default()),
        (Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Seq(_), at) => {
            Err(MarkdownError::NotMapping(Some(at)))
        }
    }
}

#[cfg(test)]
mod tests;
