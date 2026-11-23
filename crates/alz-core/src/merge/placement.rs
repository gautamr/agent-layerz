//! Where a layer lies in relation to the repository being built (spec §7.12), which decides who
//! may declare a commit identity or an account. The file system tells it, in `alz-io`; this crate
//! only holds the answer.

use std::collections::BTreeMap;

use crate::layer::LayerName;

/// Whether a layer's real directory lies inside the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Placement {
    /// Inside the repository: the repository's own content, which is untrusted for declarations.
    Inside,
    /// Outside the repository.
    Outside,
}

/// The [`Placement`] of each layer of a stack.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Placements(BTreeMap<LayerName, Placement>);

impl Placements {
    /// The placement of a layer. A layer that was not placed counts as inside, the stricter
    /// answer, so a missing entry can never let a layer declare.
    #[must_use]
    pub fn of(&self, layer: &LayerName) -> Placement {
        self.0.get(layer).copied().unwrap_or(Placement::Inside)
    }
}

impl FromIterator<(LayerName, Placement)> for Placements {
    fn from_iter<I: IntoIterator<Item = (LayerName, Placement)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{Placement, Placements};
    use crate::layer::LayerName;

    fn layer(name: &str) -> LayerName {
        LayerName::try_from(name).expect("a layer name")
    }

    #[test]
    fn a_layer_with_no_placement_counts_as_inside_the_repository() {
        assert_eq!(Placements::default().of(&layer("x")), Placement::Inside);
    }

    #[test]
    fn a_placed_layer_has_its_placement() {
        let placements: Placements = [(layer("personal"), Placement::Outside)]
            .into_iter()
            .collect();
        assert_eq!(placements.of(&layer("personal")), Placement::Outside);
    }
}
