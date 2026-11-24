//! The heading that wraps a layer's text in a composed file: `## <Layer kind>: <layer name>`
//! (spec §7.1, §7.10).

use std::fmt;

use crate::layer::{Kind, LayerMeta, LayerName};
use crate::untrusted_text::Stripped;

/// A layer's heading, `## Personal: personal`. The heading holds the kind as the layer wrote it,
/// capitalised by [`Kind::title`]; it is text from the layer, so it prints without its control and
/// format characters (class 1 of §10.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerHeading {
    kind: Kind,
    name: LayerName,
}

impl LayerHeading {
    /// The heading of the layer `meta` describes.
    #[must_use]
    pub fn of(meta: &LayerMeta) -> Self {
        Self {
            kind: meta.kind().clone(),
            name: meta.name().clone(),
        }
    }

    /// The layer's kind.
    #[must_use]
    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    /// The layer's name.
    #[must_use]
    pub const fn name(&self) -> &LayerName {
        &self.name
    }
}

impl fmt::Display for LayerHeading {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "## {}: {}",
            Stripped::from_untrusted(&self.kind.title()),
            self.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LayerHeading;
    use crate::layer::LayerMeta;
    use crate::lint::parse_document;

    #[test]
    fn the_heading_names_the_kind_in_capitals_and_the_layer() {
        let node =
            parse_document(b"name: shop-api\nkind: repo\nvisibility: private\n").expect("YAML");
        let meta = LayerMeta::from_yaml(&node).expect("a layer");
        assert_eq!(LayerHeading::of(&meta).to_string(), "## Repo: shop-api");
    }
}
