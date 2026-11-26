//! A source of a generated file (spec §4.3 **The header and its sources.**).

use crate::layer_name::LayerName;

/// A layer file that a generated file came from. The core writes the header from its sources:
/// `<layer>@<rev>:<path>`, or `<layer>:<path>` when there is no revision. An adapter names the
/// sources and has no way to write or change the header.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct Source {
    /// The layer.
    pub layer: LayerName,
    /// The stack entry's `ref`, or else the layer's `version`; none when there is neither.
    pub revision: Option<String>,
    /// The path of the file inside the layer.
    pub path: String,
}

impl Source {
    /// A source without a revision.
    #[must_use]
    pub fn new(layer: LayerName, path: impl Into<String>) -> Self {
        Self {
            layer,
            revision: None,
            path: path.into(),
        }
    }

    /// The same source, with a revision.
    #[must_use]
    pub fn at_revision(self, revision: impl Into<String>) -> Self {
        Self {
            revision: Some(revision.into()),
            ..self
        }
    }
}
