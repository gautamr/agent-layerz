//! A generated Markdown file with a frontmatter block: the header as the first line of the block,
//! the YAML of the fields, and the body (spec §4.3, §8.1).

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::merge::{Origin, Provenance};
use crate::render::tree::GeneratedFile;
use crate::untrusted_text::Stripped;

use super::{Output, output_path};
use crate::render::header::{SourceRef, Sources, frontmatter_header};
use crate::render::sources::refs_of;
use crate::render::yaml::{DuplicateKey, EmitMap, EmitValue, emit};

/// The sources of a header from the origins, or the errors `layer` for each that cannot stand in
/// a comment.
pub(super) fn sources<'a>(
    origins: impl IntoIterator<Item = &'a Origin>,
) -> Result<Sources, Diagnostics> {
    refs_of(origins).map(|refs| {
        let mut refs = refs.into_iter();
        refs.next().map_or_else(
            || Sources::single(SourceRef::NoInstructions),
            |first| Sources::new(first, refs),
        )
    })
}

/// The origins of all contributions of `provenance`, most general first.
pub(super) fn all_origins(provenance: &Provenance) -> impl Iterator<Item = &Origin> + '_ {
    provenance
        .contributions()
        .iter()
        .map(crate::merge::Contribution::origin)
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

/// Where an item's problems are reported: the file of its first contribution.
pub(super) fn place(provenance: &Provenance) -> Option<Where> {
    provenance.contributions().first().map(|first| {
        Where::layer_file(
            Stripped::from_untrusted(first.layer().as_str()),
            Stripped::from_untrusted(first.file().as_str()),
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
