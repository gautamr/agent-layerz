use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use super::{Accumulator, Merging, merge};
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::layer::{LayerContents, LayerMeta, LayerName};
use crate::lint::parse_document;
use crate::merge::{Merged, Placement, Placements};
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack, Source};

fn layer(name: &str) -> ResolvedLayer {
    let yaml = format!("name: {name}\nkind: personal\nvisibility: public\n");
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    ResolvedLayer::new(
        Source::try_from(format!("path:../{name}").as_str()).expect("a source"),
        LayerMeta::from_yaml(&node).expect("a layer"),
        LayerContents::default(),
        None,
    )
}

fn stack(names: &[&str]) -> ResolvedStack {
    ResolvedStack::validate(names.iter().map(|name| layer(name)).collect()).expect("a stack")
}

fn manifest() -> Manifest {
    let node = parse_document(b"stack:\n  - source: path:./.layer\n").expect("valid YAML");
    Manifest::from_yaml(&node).expect("a manifest")
}

fn names(merged: &Merged) -> Vec<&str> {
    merged.layers().iter().map(LayerName::as_str).collect()
}

#[test]
fn the_merged_model_lists_the_layers_in_stack_order() {
    let found = merge(
        &stack(&["one", "two", "three"]),
        &manifest(),
        &Placements::default(),
    );
    assert!(found.diagnostics().iter().next().is_none());
    let merged = found.merged().expect("no error was found");
    assert_eq!(names(merged), ["one", "two", "three"]);
}

#[test]
fn merging_the_same_input_twice_gives_equal_results() {
    let stack = stack(&["one", "two"]);
    let placements: Placements = [(
        LayerName::try_from("one").expect("a name"),
        Placement::Outside,
    )]
    .into_iter()
    .collect();
    assert_eq!(
        merge(&stack, &manifest(), &placements),
        merge(&stack, &manifest(), &placements)
    );
}

#[test]
fn an_error_leaves_no_merged_model_and_keeps_every_diagnostic() {
    let diagnostics = Diagnostics::new()
        .with(Diagnostic::warning(
            Code::Frontmatter,
            None,
            Message::new("a warning"),
        ))
        .with(Diagnostic::error(
            Code::Frontmatter,
            None,
            Message::new("an error"),
        ));
    let state = Accumulator {
        layers: Vec::new(),
        diagnostics,
        ..Accumulator::default()
    };
    let found: Merging = state.finish(true);
    assert!(found.merged().is_none());
    assert_eq!(found.diagnostics().error_count(), 1);
    assert_eq!(found.diagnostics().warning_count(), 1);
}

#[test]
fn a_warning_alone_keeps_the_merged_model() {
    let state = Accumulator {
        layers: Vec::new(),
        diagnostics: Diagnostics::new().with(Diagnostic::warning(
            Code::Frontmatter,
            None,
            Message::new("a warning"),
        )),
        ..Accumulator::default()
    };
    assert!(state.finish(true).merged().is_some());
}

proptest! {
    #[test]
    fn merging_is_deterministic_and_keeps_the_stack_order(
        count in 1_usize..6,
        outside in proptest::collection::vec(proptest::bool::ANY, 6),
    ) {
        let all = ["a", "b", "c", "d", "e", "f"];
        let chosen: Vec<&str> = all.iter().copied().take(count).collect();
        let placements: Placements = chosen
            .iter()
            .zip(&outside)
            .map(|(name, outside)| {
                (
                    LayerName::try_from(*name).expect("a name"),
                    if *outside { Placement::Outside } else { Placement::Inside },
                )
            })
            .collect();
        let stack = stack(&chosen);
        let first = merge(&stack, &manifest(), &placements);
        prop_assert_eq!(&first, &merge(&stack, &manifest(), &placements));
        let merged = first.merged().expect("no error was found");
        prop_assert!(names(merged) == chosen);
    }
}
