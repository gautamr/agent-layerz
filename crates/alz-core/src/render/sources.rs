//! The sources of a generated file's header, from the origins of the contributions that made it
//! or from the sources an adapter named (spec §4.3): each becomes a [`SourceRef`], or an error
//! `layer` when its layer file or revision cannot stand in a comment.

use alz_adapter::Source;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::merge::Origin;
use crate::render::header::{SourceRef, SourceRefError};
use crate::untrusted_text::Stripped;

/// The source each origin names, or an error `layer` for each origin whose source cannot stand in
/// a comment.
pub(crate) fn refs_of<'a>(
    origins: impl IntoIterator<Item = &'a Origin>,
) -> Result<Vec<SourceRef>, Diagnostics> {
    collected(origins.into_iter().map(|origin| {
        SourceRef::of_origin(origin).map_err(|problem| {
            source_error(origin.layer().as_str(), origin.file().as_str(), problem)
        })
    }))
}

/// The source each of `sources` names, or an error `layer` for each one that cannot stand in a
/// comment.
pub(crate) fn refs_of_sources(sources: &[Source]) -> Result<Vec<SourceRef>, Diagnostics> {
    collected(sources.iter().map(|source| {
        SourceRef::of_source(source)
            .map_err(|problem| source_error(source.layer.as_str(), source.path.as_str(), problem))
    }))
}

/// All the refs, or all the problems when there is one.
fn collected(
    made: impl Iterator<Item = Result<SourceRef, Diagnostic>>,
) -> Result<Vec<SourceRef>, Diagnostics> {
    let (refs, problems): (Vec<_>, Vec<_>) = made.partition(Result::is_ok);
    if problems.is_empty() {
        Ok(refs.into_iter().flatten().collect())
    } else {
        Err(problems.into_iter().filter_map(Result::err).collect())
    }
}

/// The error for a source that cannot stand in the header (§4.3).
fn source_error(layer: &str, file: &str, problem: SourceRefError) -> Diagnostic {
    let message = match problem {
        SourceRefError::Unprintable | SourceRefError::ClosesComment => Message::from(
            "a source of the generated header holds a control or format character or '-->', which would end its comment; rename the file or change the version or ref",
        ),
        SourceRefError::NotSafePath => {
            Message::from("a source of the generated header is not a safe relative path")
        }
    };
    Diagnostic::error(
        Code::Layer,
        Some(Where::layer_file(
            Stripped::from_untrusted(layer),
            Stripped::from_untrusted(file),
        )),
        message,
    )
}
