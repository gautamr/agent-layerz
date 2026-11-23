//! Spec §6 step 2, over the layers of a stack in order: no two layers share a name, and a layer is
//! never more public than a layer before it, because it extends every layer before it (§3.1).

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::stack::resolved::ResolvedLayer;
use crate::untrusted_text::Stripped;

fn source_text(layer: &ResolvedLayer) -> Stripped {
    Stripped::from_untrusted(&layer.source().to_string())
}

/// The error `duplicate-layer` for each layer whose name an earlier layer has: it names that
/// layer's source and this one's, with no location.
pub(crate) fn duplicate_names(layers: &[ResolvedLayer]) -> Diagnostics {
    layers
        .iter()
        .scan(BTreeMap::new(), |seen, layer| {
            let previous = seen.insert(layer.meta().name().as_str(), layer);
            Some(previous.map(|previous| {
                Diagnostic::error(
                    Code::DuplicateLayer,
                    None,
                    Message::new("layer name '")
                        .then(MessagePart::printable(layer.meta().name()))
                        .then("' is used by both '")
                        .then(source_text(previous))
                        .then("' and '")
                        .then(source_text(layer))
                        .then("'"),
                )
            }))
        })
        .flatten()
        .collect()
}

/// The error `visibility` for each pair of layers where the later is more public than the earlier,
/// at the later layer's source: for each layer in order, each earlier layer in order.
pub(crate) fn visibility_order(layers: &[ResolvedLayer]) -> Diagnostics {
    layers
        .iter()
        .enumerate()
        .flat_map(|(index, layer)| {
            layers
                .iter()
                .take(index)
                .filter(|general| layer.meta().visibility() < general.meta().visibility())
                .map(move |general| {
                    Diagnostic::error(
                        Code::Visibility,
                        Some(Where::given(source_text(layer))),
                        Message::new(layer.meta().visibility().as_str())
                            .then(" layer '")
                            .then(MessagePart::printable(layer.meta().name()))
                            .then("' cannot extend ")
                            .then(general.meta().visibility().as_str())
                            .then(" layer '")
                            .then(MessagePart::printable(general.meta().name()))
                            .then("'; order the stack from most public to most private"),
                    )
                })
        })
        .collect()
}
