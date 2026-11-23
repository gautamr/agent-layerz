//! The laws of the deep merge of settings (spec §7.9): associative over kind-consistent trees,
//! with the empty settings as identity, and idempotent.

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::{BoxedStrategy, Strategy, prop_assert_eq, proptest};

use crate::item::{Settings, SettingsTree};
use crate::layer::LayerName;
use crate::merge::Tree;
use crate::target::TargetName;

use super::SettingsState;

fn numbers() -> impl Strategy<Value = Vec<i128>> {
    vec(0_i128..4, 0..4)
}

/// The numbers of a list that holds none twice: what a list is once a merge de-duplicated it.
fn distinct_numbers() -> impl Strategy<Value = Vec<i128>> {
    numbers().prop_map(|items| {
        items.into_iter().fold(Vec::new(), |kept, item| {
            if kept.contains(&item) {
                kept
            } else {
                kept.into_iter().chain([item]).collect()
            }
        })
    })
}

/// Settings whose keys always hold the same kind: `a` and `b` numbers, `m` a mapping of numbers,
/// `l` a list of numbers, `o` a list of one-key mappings, so the merge is associative.
fn trees() -> impl Strategy<Value = SettingsTree> {
    trees_with(numbers().boxed())
}

/// [`trees`] whose list `l` holds no item twice.
fn deduplicated_trees() -> impl Strategy<Value = SettingsTree> {
    trees_with(distinct_numbers().boxed())
}

fn trees_with(lists: BoxedStrategy<Vec<i128>>) -> impl Strategy<Value = SettingsTree> {
    (
        proptest::option::of(0_i128..3),
        proptest::option::of(0_i128..3),
        proptest::option::of(numbers()),
        proptest::option::of(lists),
        proptest::option::of(distinct_pairs()),
    )
        .prop_map(|(a, b, inner, list, objects)| {
            let entries: BTreeMap<String, Tree> = [
                a.map(|n| ("a", Tree::Int(n))),
                b.map(|n| ("b", Tree::Int(n))),
                inner.map(|items| {
                    (
                        "m",
                        Tree::Map(
                            items
                                .into_iter()
                                .map(|n| (format!("k{n}"), Tree::Int(n)))
                                .collect(),
                        ),
                    )
                }),
                list.map(|items| ("l", Tree::List(items.into_iter().map(Tree::Int).collect()))),
                objects.map(|items| {
                    (
                        "o",
                        Tree::List(
                            items
                                .into_iter()
                                .map(|(key, value)| {
                                    Tree::Map(
                                        [(format!("j{key}"), Tree::Int(value))]
                                            .into_iter()
                                            .collect(),
                                    )
                                })
                                .collect(),
                        ),
                    )
                }),
            ]
            .into_iter()
            .flatten()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
            SettingsTree::from(entries)
        })
}

fn distinct_pairs() -> impl Strategy<Value = Vec<(i128, i128)>> {
    vec((0_i128..2, 0_i128..2), 0..3).prop_map(|pairs| {
        pairs.into_iter().fold(Vec::new(), |kept, pair| {
            if kept.contains(&pair) {
                kept
            } else {
                kept.into_iter().chain([pair]).collect()
            }
        })
    })
}

fn layer(name: &str) -> LayerName {
    LayerName::try_from(name).expect("a layer name")
}

proptest! {
    #[test]
    fn the_deep_merge_is_associative(a in trees(), b in trees(), c in trees()) {
        let left = a.clone().deep_merge(b.clone()).deep_merge(c.clone());
        let right = a.deep_merge(b.deep_merge(c));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn the_empty_settings_are_the_identity(a in trees()) {
        prop_assert_eq!(SettingsTree::default().deep_merge(a.clone()), a.clone());
        prop_assert_eq!(a.clone().deep_merge(SettingsTree::default()), a);
    }

    #[test]
    fn merging_a_layer_twice_changes_nothing_more(a in deduplicated_trees(), b in deduplicated_trees()) {
        let once = a.clone().deep_merge(b.clone());
        prop_assert_eq!(once.clone().deep_merge(b), once);
    }

    #[test]
    fn the_fold_of_the_layers_is_the_deep_merge_of_their_settings_per_target(
        a in trees(),
        b in trees(),
    ) {
        let first = Settings::new([(TargetName::Claude, a.clone())]);
        let second = Settings::new([(TargetName::Claude, b.clone())]);
        let folded = SettingsState::default()
            .apply(&first, &layer("one"))
            .apply(&second, &layer("two"))
            .finish();
        let claude = folded.get(&TargetName::Claude).map(|settings| settings.value().clone());
        prop_assert_eq!(claude, Some(a.deep_merge(b)));
        prop_assert_eq!(folded.len(), 1);
    }
}
