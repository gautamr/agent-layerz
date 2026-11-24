//! Where each item of the merged model came from, for `alz explain` (spec §10).

use crate::layer::{ItemPath, LayerName};
use crate::merge::action::Action;
use crate::path::SafeRelPath;
use crate::stack::{ResolvedLayer, Revision};

/// Where a contribution came from: the layer, the layer's revision and the layer file. These are
/// the parts of a header source (§4.3) and of `alz explain`'s source column (§10.3); the text is
/// made from them at rendering, which can refuse an untrusted revision or path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    layer: LayerName,
    revision: Option<Revision>,
    file: SafeRelPath,
}

impl Origin {
    /// The origin of `file`, a file of `layer`.
    #[must_use]
    pub fn new(layer: &ResolvedLayer, file: SafeRelPath) -> Self {
        Self {
            layer: layer.meta().name().clone(),
            revision: layer.revision(),
            file,
        }
    }

    /// The origin of the layer's `layer.yaml`, which a commit identity or an account is read from.
    ///
    /// # Panics
    ///
    /// Never: the literal is a safe relative path.
    #[must_use]
    pub fn layer_yaml(layer: &ResolvedLayer) -> Self {
        Self::new(
            layer,
            SafeRelPath::try_from("layer.yaml").expect("layer.yaml is a safe relative path"),
        )
    }

    /// The origin of the layer's file of kind `item`, such as `mcp.yaml`, when the layer has one.
    #[must_use]
    pub fn of_item(layer: &ResolvedLayer, item: ItemPath<'_>) -> Option<Self> {
        layer
            .contents()
            .paths()
            .find(|path| ItemPath::of(path) == item)
            .map(|path| Self::new(layer, path.clone()))
    }

    /// An origin in a unit test: the layer of that name, without a revision, file `layer.yaml`.
    #[cfg(test)]
    pub(crate) fn in_test(layer: LayerName) -> Self {
        Self {
            layer,
            revision: None,
            file: SafeRelPath::try_from("layer.yaml").expect("a safe relative path"),
        }
    }

    /// The layer's name.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        &self.layer
    }

    /// The layer's revision: the pin's `rev`, else its `version`.
    #[must_use]
    pub const fn revision(&self) -> Option<&Revision> {
        self.revision.as_ref()
    }

    /// The layer file, such as `rules/testing.md` or `layer.yaml`.
    #[must_use]
    pub const fn file(&self) -> &SafeRelPath {
        &self.file
    }
}

/// One layer's part in an item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Contribution {
    origin: Origin,
    action: Action,
}

impl Contribution {
    /// Where it came from.
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The layer.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        self.origin.layer()
    }

    /// The layer's revision.
    #[must_use]
    pub const fn revision(&self) -> Option<&Revision> {
        self.origin.revision()
    }

    /// The layer file that contributed.
    #[must_use]
    pub const fn file(&self) -> &SafeRelPath {
        self.origin.file()
    }

    /// What the layer did.
    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }
}

/// The contributions to an item in stack order, most general first. It is never empty: an item
/// exists because a layer defined it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provenance(Vec<Contribution>);

impl Provenance {
    /// The provenance of an item that `origin` did `action` to first.
    #[must_use]
    pub fn first(origin: Origin, action: Action) -> Self {
        Self(vec![Contribution { origin, action }])
    }

    /// The same provenance with one more contribution, from a more specific layer.
    #[must_use]
    pub fn then(self, origin: Origin, action: Action) -> Self {
        Self(
            self.0
                .into_iter()
                .chain([Contribution { origin, action }])
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
    /// A value that `origin` did `action` to first.
    #[must_use]
    pub fn new(value: T, origin: Origin, action: Action) -> Self {
        Self {
            value,
            provenance: Provenance::first(origin, action),
        }
    }

    /// A value with the contributions that made it, for a merge that builds the provenance itself.
    #[must_use]
    pub(crate) const fn with_provenance(value: T, provenance: Provenance) -> Self {
        Self { value, provenance }
    }

    /// The value after a more specific layer changed it to `value`.
    #[must_use]
    pub fn revised(self, value: T, origin: Origin, action: Action) -> Self {
        Self {
            value,
            provenance: self.provenance.then(origin, action),
        }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The same contributions with the value changed by `change`, which may refuse it: `None`
    /// when it does. For the finished form of a value that was built up while layers were
    /// absorbed.
    #[must_use]
    pub fn filter_map<U>(self, change: impl FnOnce(T) -> Option<U>) -> Option<Traced<U>> {
        change(self.value).map(|value| Traced {
            value,
            provenance: self.provenance,
        })
    }

    /// The contributions that made the value.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

#[cfg(test)]
mod tests;
