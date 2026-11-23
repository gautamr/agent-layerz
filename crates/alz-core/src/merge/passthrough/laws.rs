//! The laws of the per-path merge of passthrough files (spec §7.11): associative, with the empty
//! state as identity, and a fold of layers keeps, per path, the file of the most specific layer
//! and every layer that had the path in the provenance.

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

use crate::item::PassthroughItems;
use crate::layer::LayerName;
use crate::merge::Replace;
use crate::merge::provenance::{Action, Contribution};
use crate::merge::testing::layer;

use super::PassthroughState;

/// What a generated layer holds: files as (target, path number, content number).
type Files = Vec<(u8, u8, u8)>;

fn files() -> impl Strategy<Value = Files> {
    vec((0_u8..2, 0_u8..3, 0_u8..4), 0..5)
}

fn target(number: u8) -> &'static str {
    if number == 0 { "claude" } else { "codex" }
}

fn path(target_number: u8, path_number: u8) -> String {
    format!("targets/{}/dir/f{path_number}.md", target(target_number))
}

/// The state of the layer called `name` with `files`, read the way the merge reads a layer.
fn state(name: &str, files: &Files) -> PassthroughState {
    let texts: Vec<(String, String)> = files
        .iter()
        .map(|(t, p, c)| (path(*t, *p), format!("content {c}")))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let layer = layer(name, "client", &refs);
    let (items, found) = PassthroughItems::read(&layer);
    assert_eq!(found.error_count(), 0);
    PassthroughState::of_layer(items, &LayerName::try_from(name).expect("a layer name"))
}

proptest! {
    #[test]
    fn merging_is_associative(a in files(), b in files(), c in files()) {
        let (a, b, c) = (state("la", &a), state("lb", &b), state("lc", &c));
        prop_assert_eq!(
            a.clone().merge(b.clone()).merge(c.clone()),
            a.merge(b.merge(c)),
        );
    }

    #[test]
    fn the_empty_state_is_the_identity_on_both_sides(a in files()) {
        let a = state("la", &a);
        prop_assert_eq!(PassthroughState::default().merge(a.clone()), a.clone());
        prop_assert_eq!(a.clone().merge(PassthroughState::default()), a);
    }

    #[test]
    fn a_fold_keeps_the_most_specific_file_per_path_and_every_layer_that_had_it(
        stack in vec(files(), 0..4),
    ) {
        let folded = stack
            .iter()
            .enumerate()
            .fold(PassthroughState::default(), |state_so_far, (index, files)| {
                state_so_far.merge(state(&format!("l{index}"), files))
            })
            .finish();
        // What each (target, path) should hold: the last layer's content, and the layers in order.
        let expected: BTreeMap<(String, String), (String, Vec<String>)> = stack
            .iter()
            .enumerate()
            .flat_map(|(index, files)| {
                files.iter().map(move |(t, p, c)| {
                    (
                        (target(*t).to_owned(), path(*t, *p)),
                        (format!("content {c}"), format!("l{index}")),
                    )
                })
            })
            .fold(BTreeMap::new(), |mut all, (key, (content, layer_name))| {
                let layers = all
                    .remove(&key)
                    .map_or_else(Vec::new, |(_, layers): (String, Vec<String>)| layers);
                let layers = if layers.last() == Some(&layer_name) {
                    layers
                } else {
                    layers.into_iter().chain([layer_name]).collect()
                };
                all.insert(key, (content, layers));
                all
            });
        let found: BTreeMap<(String, String), (String, Vec<String>)> = folded
            .iter()
            .flat_map(|(target, files)| {
                files.iter().map(move |(_, traced)| {
                    (
                        (target.as_str().to_owned(), traced.value().source().as_str().to_owned()),
                        (
                            String::from_utf8_lossy(traced.value().bytes()).into_owned(),
                            traced
                                .provenance()
                                .contributions()
                                .iter()
                                .map(|part| part.layer().as_str().to_owned())
                                .collect(),
                        ),
                    )
                })
            })
            .collect();
        prop_assert_eq!(found, expected);
        // The first layer to have a path defined it; every later one replaced it.
        let actions_ok = folded.values().flat_map(Replace::iter).all(|(_, traced)| {
            let actions: Vec<Action> = traced
                .provenance()
                .contributions()
                .iter()
                .map(Contribution::action)
                .collect();
            actions.first() == Some(&Action::Define)
                && actions.iter().skip(1).all(|action| *action == Action::Replace)
        });
        prop_assert_eq!(actions_ok, true);
    }
}
