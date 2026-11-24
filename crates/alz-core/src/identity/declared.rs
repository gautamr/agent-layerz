//! The identity a layer declares and the layer that declared it, as a report names them
//! (spec §9.4): `layer 'client' (path:../client)`.

use crate::identity::CommitIdentity;
use crate::layer::LayerName;
use crate::merge::Declared;
use crate::stack::Source;
use crate::untrusted_text::Stripped;

/// The layer that declared the commit identity: its name and the source the stack gives for it,
/// stripped of control and format characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredBy {
    pub(super) layer: LayerName,
    pub(super) source: Stripped,
}

impl DeclaredBy {
    /// The layer `layer`, listed in the stack as `source`.
    #[must_use]
    pub fn new(layer: &LayerName, source: &Source) -> Self {
        Self {
            layer: layer.clone(),
            source: Stripped::from_untrusted(&source.to_string()),
        }
    }
}

/// The winning declaration of `commit_identity` and who made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredIdentity {
    pub(super) by: DeclaredBy,
    pub(super) identity: CommitIdentity,
}

impl DeclaredIdentity {
    /// A declaration.
    #[must_use]
    pub const fn new(by: DeclaredBy, identity: CommitIdentity) -> Self {
        Self { by, identity }
    }

    /// The winner of the merge, or `None` when no layer declares or the declarations conflict.
    /// `source_of` gives the stack source of the winning layer.
    #[must_use]
    pub fn from_winner<'a>(
        declared: &Declared<CommitIdentity>,
        source_of: impl FnOnce(&LayerName) -> Option<&'a Source>,
    ) -> Option<Self> {
        let (layer, identity) = declared.winner()?;
        let source = source_of(layer)?;
        Some(Self::new(DeclaredBy::new(layer, source), identity.clone()))
    }

    /// The declared identity.
    #[must_use]
    pub const fn identity(&self) -> &CommitIdentity {
        &self.identity
    }

    /// The declaring layer.
    #[must_use]
    pub const fn by(&self) -> &DeclaredBy {
        &self.by
    }
}
