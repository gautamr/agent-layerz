//! A generated Markdown file with a frontmatter block: the header as the first line of the block,
//! the YAML of the fields, and the body (spec §4.3, §8.1).

use alz_adapter::Source;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::render::tree::GeneratedFile;
use crate::untrusted_text::Stripped;

use super::{Output, output_path};
use crate::render::header::{SourceRef, Sources, frontmatter_header};
use crate::render::sources::refs_of_sources;
use crate::render::yaml::{DuplicateKey, EmitMap, EmitValue, emit};

/// The sources of a header from the model's sources, or the errors `layer` for each that cannot
/// stand in a comment.
pub(super) fn sources(named: &[Source]) -> Result<Sources, Diagnostics> {
    refs_of_sources(named).map(|refs| {
        let mut refs = refs.into_iter();
        refs.next().map_or_else(
            || Sources::single(SourceRef::NoInstructions),
            |first| Sources::new(first, refs),
        )
    })
}

/// `---`, the header comment, the fields as YAML, `---`, then a blank line and the body when it
/// has any, and a line feed.
///
/// # Errors
///
/// [`DuplicateKey`] when two fields have one key.
pub(super) fn document(
    sources: &Sources,
    fields: Vec<(String, EmitValue)>,
    body: &str,
) -> Result<String, DuplicateKey> {
    EmitMap::from_entries(fields).map(|map| {
        let body = if body.is_empty() {
            String::new()
        } else {
            format!("\n{body}\n")
        };
        format!(
            "---\n{}\n{}---\n{body}",
            frontmatter_header(sources),
            emit(&EmitValue::Map(map))
        )
    })
}

/// A text field.
pub(super) fn text(value: &str) -> EmitValue {
    EmitValue::Text(value.to_owned())
}

/// Where an item's problems are reported: the file of its first source.
pub(super) fn place(named: &[Source]) -> Option<Where> {
    named.first().map(|first| {
        Where::layer_file(
            Stripped::from_untrusted(first.layer.as_str()),
            Stripped::from_untrusted(&first.path),
        )
    })
}

/// The Markdown file at `path`, its header from `sources`, `found` added to the diagnostics.
pub(super) fn file(
    path: &str,
    sources: Result<Sources, Diagnostics>,
    fields: Vec<(String, EmitValue)>,
    body: &str,
    found: Diagnostics,
) -> Output {
    match sources {
        Err(problems) => Output {
            files: Vec::new(),
            diagnostics: found.merge(problems),
        },
        Ok(sources) => match document(&sources, fields, body) {
            Ok(text) => Output::one(output_path(path), GeneratedFile::text(&text), found),
            Err(twice) => Output {
                files: Vec::new(),
                diagnostics: found.with(Diagnostic::error(
                    Code::ClaudeAgent,
                    None,
                    Message::new("the file ")
                        .then(Stripped::from_untrusted(path))
                        .then(" would hold the field ")
                        .then(Stripped::from_untrusted(twice.key()))
                        .then(" twice"),
                )),
            },
        },
    }
}
