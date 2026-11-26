//! The sources a generated header names (spec §4.3), from the origins of the contributions.

use alz_adapter::Source;

use crate::merge::Origin;

/// The source an origin names: its layer, the layer's revision and the layer file.
pub(super) fn of_origin(origin: &Origin) -> Source {
    let source = Source::new(origin.layer().to_contract(), origin.file().as_str());
    match origin.revision() {
        Some(revision) => source.at_revision(revision.as_str()),
        None => source,
    }
}

/// The sources of `origins`, in the order given.
pub(super) fn of_origins<'a>(origins: impl IntoIterator<Item = &'a Origin>) -> Vec<Source> {
    origins.into_iter().map(of_origin).collect()
}
