//! Where each item of the merged model came from, for `alz explain` (spec §10).

use crate::layer::LayerName;

/// What a layer did to an item.
///
/// A kind of §7 that needs a finer verb adds a variant here; the exhaustive matches then show every
/// place that reports the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    /// The layer defined the item.
    Define,
    /// The layer added to an item of a more general layer.
    Extend,
    /// The layer replaced the item, or a part of it, of a more general layer.
    Replace,
    /// The layer removed something from an item of a more general layer.
    Remove,
}

/// One layer's part in an item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Contribution {
    layer: LayerName,
    action: Action,
}

impl Contribution {
    /// The layer.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        &self.layer
    }

    /// What the layer did.
    #[must_use]
    pub const fn action(&self) -> Action {
        self.action
    }
}

/// The contributions to an item in stack order, most general first. It is never empty: an item
/// exists because a layer defined it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provenance(Vec<Contribution>);

impl Provenance {
    /// The provenance of an item that `layer` did `action` to first.
    #[must_use]
    pub fn first(layer: LayerName, action: Action) -> Self {
        Self(vec![Contribution { layer, action }])
    }

    /// The same provenance with one more contribution, from a more specific layer.
    #[must_use]
    pub fn then(self, layer: LayerName, action: Action) -> Self {
        Self(
            self.0
                .into_iter()
                .chain([Contribution { layer, action }])
                .collect(),
        )
    }

    /// The contributions, most general layer first.
    #[must_use]
    pub fn contributions(&self) -> &[Contribution] {
        &self.0
    }
}

/// A value of the merged model with the contributions that made it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Traced<T> {
    value: T,
    provenance: Provenance,
}

impl<T> Traced<T> {
    /// A value that `layer` did `action` to first.
    #[must_use]
    pub fn new(value: T, layer: LayerName, action: Action) -> Self {
        Self {
            value,
            provenance: Provenance::first(layer, action),
        }
    }

    /// The value after a more specific layer changed it to `value`.
    #[must_use]
    pub fn revised(self, value: T, layer: LayerName, action: Action) -> Self {
        Self {
            value,
            provenance: self.provenance.then(layer, action),
        }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The contributions that made the value.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Provenance, Traced};
    use crate::layer::LayerName;

    fn layer(name: &str) -> LayerName {
        LayerName::try_from(name).expect("a layer name")
    }

    #[test]
    fn a_revised_value_lists_every_layer_that_touched_it_most_general_first() {
        let traced = Traced::new(1, layer("personal"), Action::Define)
            .revised(2, layer("client"), Action::Replace)
            .revised(3, layer("repo"), Action::Extend);
        let found: Vec<(&str, Action)> = traced
            .provenance()
            .contributions()
            .iter()
            .map(|part| (part.layer().as_str(), part.action()))
            .collect();
        assert_eq!(
            found,
            [
                ("personal", Action::Define),
                ("client", Action::Replace),
                ("repo", Action::Extend)
            ]
        );
        assert_eq!(traced.value(), &3);
    }

    #[test]
    fn provenance_starts_with_one_contribution() {
        let provenance = Provenance::first(layer("personal"), Action::Define);
        assert_eq!(provenance.contributions().len(), 1);
    }
}
