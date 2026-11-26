//! A Markdown file with a frontmatter mapping (spec §8.0 **Output.**, §8.1): the fields, the body
//! and the sources the core writes the header from. The adapter never writes the header (§4.3).

use alz_adapter::{
    Content, Frontmatter, HeaderSource, JsonObject, JsonValue, Level, Markdown, Source,
};

use super::{Made, found};

/// A text field.
pub(super) fn text(value: &str) -> JsonValue {
    JsonValue::Text(value.to_owned())
}

/// The sources of a header from the model's sources.
pub(super) fn header_sources(named: &[Source]) -> Vec<HeaderSource> {
    named.iter().cloned().map(HeaderSource::from).collect()
}

/// The Markdown file at `path`: the `fields` as its frontmatter, then the body. Two fields with
/// one key are the error `claude-agent`, and no file.
pub(super) fn file(
    path: &str,
    sources: Vec<HeaderSource>,
    fields: Vec<(String, JsonValue)>,
    body: &str,
) -> Made {
    match JsonObject::from_entries(fields) {
        Ok(mapping) => Made::file(
            path,
            Content::Markdown(
                Markdown::new(body)
                    .with_frontmatter(Frontmatter::Mapping(mapping))
                    .with_header_sources(sources),
            ),
        ),
        Err(twice) => Made::problem(found(
            Level::Error,
            "claude-agent",
            None,
            format!("the file {path} would hold the field {} twice", twice.key()),
        )),
    }
}
