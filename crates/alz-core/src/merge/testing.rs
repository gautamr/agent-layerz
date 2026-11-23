//! Helpers for the tests of the merge: layers and stacks built from text, so a test states its
//! stack as the files the acceptance fixtures hold.

use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::merge::{Merging, Placements, merge};
use crate::path::SafeRelPath;
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack, Source};

/// A layer called `name` of kind `kind`, with the files `(path, text)`. Every layer is private,
/// so a stack of them is a valid stack (§3.1).
pub(crate) fn layer(name: &str, kind: &str, files: &[(&str, &str)]) -> ResolvedLayer {
    let yaml = format!("name: {name}\nkind: {kind}\nvisibility: private\n");
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    ResolvedLayer::new(
        Source::try_from(format!("path:../{name}").as_str()).expect("a source"),
        LayerMeta::from_yaml(&node).expect("a layer"),
        files
            .iter()
            .map(|(path, text)| {
                (
                    SafeRelPath::try_from(*path).expect("a safe path"),
                    text.as_bytes().to_vec(),
                )
            })
            .collect::<LayerContents>(),
        None,
    )
}

/// The result of merging `layers`, most general first.
pub(crate) fn merged(layers: Vec<ResolvedLayer>) -> Merging {
    let node = parse_document(b"stack:\n  - source: path:./.layer\n").expect("valid YAML");
    merge(
        &ResolvedStack::validate(layers).expect("a stack"),
        &Manifest::from_yaml(&node).expect("a manifest"),
        &Placements::default(),
    )
}

/// The diagnostics of a merge, one line each.
pub(crate) fn lines(found: &Merging) -> Vec<String> {
    found
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect()
}
