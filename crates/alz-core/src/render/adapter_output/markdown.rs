//! A `Markdown` file (spec §8.0 **Output.**, §4.3): the core writes the header, from the sources
//! the adapter names, as an HTML comment line first when there is no frontmatter and as a YAML
//! comment line right after the opening `---` when there is. The adapter has no way to write or
//! change it: its text only ever comes after the core's header line.

use alz_adapter::{Frontmatter, HeaderSource, JsonObject, JsonValue, Markdown, TargetId};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart};
use crate::render::header::{SourceRef, Sources, frontmatter_header, html_header};
use crate::render::sources::refs_of_sources;
use crate::render::tree::{GeneratedFile, OutputPath};
use crate::render::yaml::{EmitFloat, EmitMap, EmitValue, emit};
use crate::untrusted_text::Stripped;

/// The separator of the header and the body when there is no frontmatter.
const PART_SEPARATOR: &str = "\n\n";

/// The file for `markdown` at `path`.
pub(super) fn file(
    target: &TargetId,
    path: &OutputPath,
    markdown: &Markdown,
) -> Result<GeneratedFile, Diagnostics> {
    let sources = sources(target, &markdown.sources)?;
    let text = match &markdown.frontmatter {
        None => plain(&sources, &markdown.body),
        Some(Frontmatter::Mapping(mapping)) => {
            let fields = mapping_value(mapping).ok_or_else(|| unwritable(target, path))?;
            mapped(&sources, &fields, &markdown.body)
        }
        Some(Frontmatter::Kept(kept)) => kept_text(&sources, kept, &markdown.body),
    };
    Ok(GeneratedFile::text(&text))
}

/// The header's sources, or the errors `layer` for each that cannot stand in a comment.
fn sources(target: &TargetId, named: &[HeaderSource]) -> Result<Sources, Diagnostics> {
    let layer_files: Vec<_> = named
        .iter()
        .filter_map(|source| match source {
            HeaderSource::LayerFile(file) => Some(file.clone()),
            HeaderSource::GeneratedFile(_) | HeaderSource::SlotStub => None,
        })
        .collect();
    let layer_refs = refs_of_sources(&layer_files)?;
    let mut layer_refs = layer_refs.into_iter();
    let refs = named
        .iter()
        .map(|source| match source {
            HeaderSource::LayerFile(_) => layer_refs
                .next()
                .ok_or_else(|| not_a_path(target, "a layer file")),
            HeaderSource::GeneratedFile(path) => OutputPath::try_from(path.as_str())
                .map(SourceRef::Output)
                .map_err(|_refused| not_a_path(target, path)),
            HeaderSource::SlotStub => Ok(SourceRef::SlotStub),
        })
        .collect::<Result<Vec<_>, Diagnostics>>()?;
    let mut refs = refs.into_iter();
    Ok(refs.next().map_or_else(
        || Sources::single(SourceRef::NoInstructions),
        |first| Sources::new(first, refs),
    ))
}

/// The error `target` for a header source that is not a path alz can write.
fn not_a_path(target: &TargetId, text: &str) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        Code::Target,
        None,
        Message::new("the '")
            .then(MessagePart::printable(target))
            .then("' adapter named the header source ")
            .then(Stripped::from_untrusted(text))
            .then(", which is not a safe relative path"),
    ))
}

/// The error `target` for a frontmatter value the YAML emitter cannot write.
fn unwritable(target: &TargetId, path: &OutputPath) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        Code::Target,
        None,
        Message::new("the '")
            .then(MessagePart::printable(target))
            .then("' adapter returned frontmatter for ")
            .then(Stripped::from_untrusted(path.as_str()))
            .then(" that holds a number the YAML emitter cannot write"),
    ))
}

/// No frontmatter: the header, a blank line, the body and a line feed.
fn plain(sources: &Sources, body: &str) -> String {
    if body.is_empty() {
        format!("{}\n", html_header(sources))
    } else {
        format!("{}{PART_SEPARATOR}{body}\n", html_header(sources))
    }
}

/// `---`, the header comment, the fields as YAML, `---`, then a blank line, the body and a line
/// feed when there is a body.
fn mapped(sources: &Sources, fields: &EmitValue, body: &str) -> String {
    let body = if body.is_empty() {
        String::new()
    } else {
        format!("\n{body}\n")
    };
    format!(
        "---\n{}\n{}---\n{body}",
        frontmatter_header(sources),
        emit(fields)
    )
}

/// `---`, the header comment, the layer file's frontmatter as it is, `---`, then the rest of the
/// layer file as it is.
fn kept_text(sources: &Sources, kept: &str, body: &str) -> String {
    let separator = if kept.is_empty() || kept.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!(
        "---\n{}\n{kept}{separator}---\n{body}",
        frontmatter_header(sources)
    )
}

/// The frontmatter mapping as the YAML emitter takes it; `None` for a number it cannot write.
fn mapping_value(mapping: &JsonObject) -> Option<EmitValue> {
    mapping
        .entries()
        .map(|(key, value)| yaml_value(value).map(|value| (key.to_owned(), value)))
        .collect::<Option<Vec<_>>>()
        .and_then(|entries| EmitMap::from_entries(entries).ok())
        .map(EmitValue::Map)
}

fn yaml_value(value: &JsonValue) -> Option<EmitValue> {
    match value {
        JsonValue::Null => Some(EmitValue::Null),
        JsonValue::Bool(flag) => Some(EmitValue::Bool(*flag)),
        JsonValue::Int(number) => i64::try_from(*number).ok().map(EmitValue::Int),
        JsonValue::Float(number) => Some(EmitValue::Float(EmitFloat::new(number.get()))),
        JsonValue::Text(text) => Some(EmitValue::Text(text.clone())),
        JsonValue::Array(items) => items
            .iter()
            .map(yaml_value)
            .collect::<Option<Vec<_>>>()
            .map(EmitValue::Seq),
        JsonValue::Object(_) => mapping_of(value),
    }
}

fn mapping_of(value: &JsonValue) -> Option<EmitValue> {
    match value {
        JsonValue::Object(object) => mapping_value(object),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Int(_)
        | JsonValue::Float(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => None,
    }
}
