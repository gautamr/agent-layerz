use proptest::collection::vec;
use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use super::{Origin, Provenance, Traced};
use crate::layer::{ItemPath, LayerName};
use crate::merge::action::Action;
use crate::merge::testing::{layer_from_yaml, merged, origin_of};
use crate::render::header::SourceRef;
use crate::stack::ResolvedLayer;

const CLIENT_YAML: &str = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\n";

fn layer_name(name: &str) -> LayerName {
    LayerName::try_from(name).expect("a layer name")
}

#[test]
fn a_revised_value_lists_every_layer_that_touched_it_most_general_first() {
    let traced = Traced::new(1, origin_of(layer_name("personal")), Action::Define)
        .revised(2, origin_of(layer_name("client")), Action::Replace)
        .revised(3, origin_of(layer_name("repo")), Action::Extend);
    let found: Vec<(&str, Action)> = traced
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action().clone()))
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
    let provenance = Provenance::first(origin_of(layer_name("personal")), Action::Define);
    assert_eq!(provenance.contributions().len(), 1);
}

// Acceptance case 26: the source of the winning layer's `layer.yaml` in the header of AGENTS.md
// carries the layer's version.
#[test]
fn an_origin_carries_the_layer_file_and_the_versions_of_its_layer() {
    let client = layer_from_yaml(CLIENT_YAML, &[("layer.yaml", CLIENT_YAML)]);
    let origin = Origin::of_item(&client, ItemPath::LayerYaml).expect("layer.yaml is a file");
    let provenance = Provenance::first(origin, Action::Define);
    let part = &provenance.contributions()[0];
    assert_eq!(
        SourceRef::of_contribution(part)
            .expect("a source")
            .to_string(),
        "client@0.1.0:layer.yaml"
    );
}

#[test]
fn a_layer_without_a_version_or_pin_is_named_by_its_name_alone() {
    let yaml = "name: shop-api\nkind: repo\nvisibility: private\n";
    let repo = layer_from_yaml(yaml, &[("layer.yaml", yaml)]);
    let origin = Origin::of_item(&repo, ItemPath::LayerYaml).expect("layer.yaml is a file");
    let part = Provenance::first(origin, Action::Define);
    assert_eq!(
        SourceRef::of_contribution(&part.contributions()[0])
            .expect("a source")
            .to_string(),
        "shop-api:layer.yaml"
    );
}

/// A stack of `count` layers `l0`, `l1`, …, each with the rule file `rules/shared.md`, the first
/// one defining it and the others replacing or extending it as `modes` say.
fn rule_stack(modes: &[bool]) -> Vec<ResolvedLayer> {
    modes
        .iter()
        .enumerate()
        .map(|(index, extend)| {
            let text = if *extend && index > 0 {
                format!("---\nmode: extend\npaths:\n  - p{index}/**\n---\nbody {index}\n")
            } else {
                format!("---\npaths:\n  - p{index}/**\n---\nbody {index}\n")
            };
            layer_from_yaml(
                &format!(
                    "name: l{index}\nkind: personal\nversion: 1.{index}.0\nvisibility: private\n"
                ),
                &[
                    ("rules/shared.md", &text),
                    (
                        "skills/s/SKILL.md",
                        &format!("---\nname: s\ndescription: d{index}\n---\nbody\n"),
                    ),
                ],
            )
        })
        .collect()
}

proptest! {
    // Provenance stays in stack order, and every contribution names a file its layer has.
    #[test]
    fn provenance_follows_the_stack_and_names_files_of_the_layers(modes in vec(proptest::bool::ANY, 1..6)) {
        let layers = rule_stack(&modes);
        let files: Vec<(String, Vec<String>)> = layers
            .iter()
            .map(|layer| {
                (
                    layer.meta().name().to_string(),
                    layer.contents().paths().map(|path| path.as_str().to_owned()).collect(),
                )
            })
            .collect();
        let found = merged(layers);
        let model = found.merged().expect("no error was found");
        let provenances: Vec<&Provenance> = model
            .rules()
            .values()
            .map(Traced::provenance)
            .chain(model.skills().iter().map(|(_, skill)| skill.provenance()))
            .collect();
        prop_assert!(!provenances.is_empty());
        for provenance in provenances {
            let position = |layer: &LayerName| files.iter().position(|(name, _)| name == layer.as_str());
            let order: Vec<usize> = provenance
                .contributions()
                .iter()
                .map(|part| position(part.layer()).expect("a layer of the stack"))
                .collect();
            prop_assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "stack order: {order:?}");
            for part in provenance.contributions() {
                let held = position(part.layer()).and_then(|index| files.get(index)).map(|(_, held)| held);
                // A skill is named by its folder, which a file of the layer lies under.
                let file = part.file().as_str();
                let under = format!("{file}/");
                let named = held.is_some_and(|held| {
                    held.iter().any(|path| path == file || path.starts_with(&under))
                });
                prop_assert!(named, "{file} is not a file or folder of its layer");
                let expected = format!("1.{}.0", position(part.layer()).expect("a layer"));
                prop_assert_eq!(
                    part.revision().map(crate::stack::Revision::as_str),
                    Some(expected.as_str())
                );
            }
        }
    }
}
