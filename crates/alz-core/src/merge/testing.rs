//! Helpers for the tests of the merge: layers and stacks built from text, so a test states its
//! stack as the files the acceptance fixtures hold.

use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::lock::ObjectId;
use crate::merge::{Merging, Placement, Placements, merge};
use crate::path::SafeRelPath;
use crate::stack::{GitRef, Manifest, Pin, ResolvedLayer, ResolvedStack, Source};

/// A layer called `name` of kind `kind`, with the files `(path, text)`. It is private, so a stack
/// of such layers is a valid stack (§3.1).
pub(crate) fn layer(name: &str, kind: &str, files: &[(&str, &str)]) -> ResolvedLayer {
    layer_visible(name, kind, "private", files)
}

/// A layer like [`layer`] with the visibility `visibility`.
pub(crate) fn layer_visible(
    name: &str,
    kind: &str,
    visibility: &str,
    files: &[(&str, &str)],
) -> ResolvedLayer {
    layer_from_yaml(
        &format!("name: {name}\nkind: {kind}\nvisibility: {visibility}\n"),
        files,
    )
}

/// A layer whose `layer.yaml` is `yaml`, with the files `(path, text)`: for a layer that declares
/// slots, as a fixture's `layer.yaml` does.
pub(crate) fn layer_from_yaml(yaml: &str, files: &[(&str, &str)]) -> ResolvedLayer {
    layer_pinned(yaml, files, None)
}

/// A layer like [`layer_from_yaml`] that came from the pin `rev`, as a stack entry with a `ref`
/// does: the revision of its headers is `rev` and not its version.
pub(crate) fn layer_pinned(yaml: &str, files: &[(&str, &str)], rev: Option<&str>) -> ResolvedLayer {
    layer_executable(yaml, files, rev, &[])
}

/// A layer like [`layer_pinned`] where the files `executable` have the owner-execute bit, as a
/// hook script of a fixture has.
pub(crate) fn layer_executable(
    yaml: &str,
    files: &[(&str, &str)],
    rev: Option<&str>,
    executable: &[&str],
) -> ResolvedLayer {
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    let meta = LayerMeta::from_yaml(&node).expect("a layer");
    ResolvedLayer::new(
        Source::try_from(format!("path:../{}", meta.name()).as_str()).expect("a source"),
        meta,
        files
            .iter()
            .map(|(path, text)| {
                (
                    SafeRelPath::try_from(*path).expect("a safe path"),
                    text.as_bytes().to_vec(),
                )
            })
            .collect::<LayerContents>()
            .with_executable(
                executable
                    .iter()
                    .map(|path| SafeRelPath::try_from(*path).expect("a safe path")),
            ),
        rev.map(|rev| Pin {
            rev: GitRef::try_from(rev).expect("a ref"),
            sha: ObjectId::try_from("0123456789abcdef0123456789abcdef01234567").expect("a commit"),
        }),
    )
}

/// The result of merging `layers`, most general first.
pub(crate) fn merged(layers: Vec<ResolvedLayer>) -> Merging {
    merged_with(layers, "")
}

/// The result of merging `layers` under a manifest that adds `options` (lines of YAML, such as
/// `options:\n  strict_slots: false\n`) to its stack.
pub(crate) fn merged_with(layers: Vec<ResolvedLayer>, options: &str) -> Merging {
    let manifest = format!("stack:\n  - source: path:./.layer\n{options}");
    let node = parse_document(manifest.as_bytes()).expect("valid YAML");
    merge(
        &ResolvedStack::validate(layers).expect("a stack"),
        &Manifest::from_yaml(&node).expect("a manifest"),
        &Placements::default(),
    )
}

/// The result of merging `layers` when the layers called `inside` lie inside the repository and
/// every other layer lies outside it: the tests of who may declare (§7.12).
pub(crate) fn merged_placed(layers: Vec<ResolvedLayer>, inside: &[&str]) -> Merging {
    let placements: Placements = layers
        .iter()
        .map(|layer| {
            let name = layer.meta().name();
            (
                name.clone(),
                if inside.contains(&name.as_str()) {
                    Placement::Inside
                } else {
                    Placement::Outside
                },
            )
        })
        .collect();
    let manifest = "stack:\n  - source: path:./.layer\n";
    let node = parse_document(manifest.as_bytes()).expect("valid YAML");
    merge(
        &ResolvedStack::validate(layers).expect("a stack"),
        &Manifest::from_yaml(&node).expect("a manifest"),
        &placements,
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

/// The result of merging `layers` when the skills an agent lists need not exist: the tests of the
/// agents use made-up skill names, and the slots are the skills' business (`merge::skills`).
/// Pair it with [`lines_but_slots`].
pub(crate) fn merged_without_slots(layers: Vec<ResolvedLayer>) -> Merging {
    merged_with(layers, "options:\n  strict_slots: false\n")
}

/// [`lines`] without the `slot-missing` warnings of [`merged_without_slots`].
pub(crate) fn lines_but_slots(found: &Merging) -> Vec<String> {
    lines(found)
        .into_iter()
        .filter(|line| !line.starts_with("warning [slot-missing]"))
        .collect()
}

/// An origin for a unit test: a layer of that name, without a revision, and the file
/// `layer.yaml`.
pub(crate) fn origin_of(layer: crate::layer::LayerName) -> crate::merge::Origin {
    crate::merge::Origin::in_test(layer)
}
