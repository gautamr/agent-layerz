//! The laws of the deep merge of settings (spec §7.9): associative over kind-consistent trees,
//! with the empty settings as identity, and idempotent.

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::{BoxedStrategy, Strategy, prop_assert, prop_assert_eq, proptest};

use crate::diagnostic::Where;
use crate::item::{Settings, SettingsTree};
use crate::layer::LayerName;
use crate::merge::Tree;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

use super::SettingsState;

fn here() -> Where {
    Where::layer_file(
        Stripped::from_untrusted("layer"),
        Stripped::from_untrusted("settings.yaml"),
    )
}

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
        let (state, _) = SettingsState::default().apply(&first, &layer("one"), &here());
        let (state, _) = state.apply(&second, &layer("two"), &here());
        let folded = state.finish();
        let claude = folded.get(&TargetName::Claude).map(|settings| settings.value().clone());
        prop_assert_eq!(claude, Some(a.deep_merge(b)));
        prop_assert_eq!(folded.len(), 1);
    }
}

// No floor key loosens down the stack (§7.9, the table of native keys).

/// What a layer sets of one key of each kind of floor: a boolean where true is stricter, a boolean
/// where false is stricter, a list that may not gain entries, a list that may only grow, and an
/// ordered text.
#[derive(Debug, Clone)]
struct Floors {
    enabled: Option<bool>,
    auto_allow: Option<bool>,
    allowed: Option<Vec<u8>>,
    denied: Option<Vec<u8>>,
    web_search: Option<&'static str>,
}

fn floors() -> impl Strategy<Value = Floors> {
    (
        proptest::option::of(proptest::bool::ANY),
        proptest::option::of(proptest::bool::ANY),
        proptest::option::of(vec(0_u8..4, 0..4)),
        proptest::option::of(vec(0_u8..4, 0..4)),
        proptest::option::of(proptest::sample::select(WEB_SEARCH.to_vec())),
    )
        .prop_map(
            |(enabled, auto_allow, allowed, denied, web_search)| Floors {
                enabled,
                auto_allow,
                allowed,
                denied,
                web_search,
            },
        )
}

/// The values of `web_search` from the loosest to the strictest, then one that is in no rank.
const WEB_SEARCH: [&str; 5] = ["live", "indexed", "cached", "disabled", "novel"];

fn names(items: &[u8]) -> Tree {
    Tree::List(
        items
            .iter()
            .map(|item| Tree::Text(format!("d{item}")))
            .collect(),
    )
}

fn nest(path: &[&str], value: Tree) -> Tree {
    path.iter().rev().fold(value, |inner, key| {
        Tree::Map(BTreeMap::from([((*key).to_owned(), inner)]))
    })
}

impl Floors {
    fn settings(&self) -> Settings {
        let claude = [
            self.enabled
                .map(|flag| nest(&["sandbox", "enabled"], Tree::Bool(flag))),
            self.auto_allow
                .map(|flag| nest(&["sandbox", "autoAllowBashIfSandboxed"], Tree::Bool(flag))),
            self.allowed
                .as_deref()
                .map(|items| nest(&["sandbox", "network", "allowedDomains"], names(items))),
            self.denied
                .as_deref()
                .map(|items| nest(&["sandbox", "network", "deniedDomains"], names(items))),
        ]
        .into_iter()
        .flatten()
        .fold(Tree::Map(BTreeMap::new()), Tree::deep_merge);
        let codex = self.web_search.map_or_else(
            || Tree::Map(BTreeMap::new()),
            |value| nest(&["web_search"], Tree::Text(value.to_owned())),
        );
        let tree = |tree: Tree| match tree {
            Tree::Map(entries) => SettingsTree::from(entries),
            Tree::Null
            | Tree::Bool(_)
            | Tree::Int(_)
            | Tree::Float(_)
            | Tree::Text(_)
            | Tree::List(_) => SettingsTree::default(),
        };
        Settings::new([
            (TargetName::Claude, tree(claude)),
            (TargetName::Codex, tree(codex)),
        ])
    }
}

/// The value at `path` of the merged settings of `target`.
fn held(state: &SettingsState, target: TargetName, path: &[&str]) -> Option<Tree> {
    let merged = state.clone().finish();
    path.iter().try_fold(
        merged.get(&target)?.value().to_tree(),
        |tree, key| match tree {
            Tree::Map(mut entries) => entries.remove(*key),
            Tree::Null
            | Tree::Bool(_)
            | Tree::Int(_)
            | Tree::Float(_)
            | Tree::Text(_)
            | Tree::List(_) => None,
        },
    )
}

fn members(tree: &Tree) -> Vec<Tree> {
    match tree {
        Tree::List(items) => items.clone(),
        Tree::Null
        | Tree::Bool(_)
        | Tree::Int(_)
        | Tree::Float(_)
        | Tree::Text(_)
        | Tree::Map(_) => Vec::new(),
    }
}

fn rank(tree: &Tree) -> Option<usize> {
    match tree {
        Tree::Text(text) => WEB_SEARCH[..4].iter().position(|value| value == text),
        Tree::Null
        | Tree::Bool(_)
        | Tree::Int(_)
        | Tree::Float(_)
        | Tree::List(_)
        | Tree::Map(_) => None,
    }
}

proptest! {
    #[test]
    fn no_floor_key_loosens_down_the_stack(stack in vec(floors(), 0..6)) {
        let claude = TargetName::Claude;
        let enabled = ["sandbox", "enabled"];
        let auto_allow = ["sandbox", "autoAllowBashIfSandboxed"];
        let allowed = ["sandbox", "network", "allowedDomains"];
        let denied = ["sandbox", "network", "deniedDomains"];
        let web_search = ["web_search"];
        stack.iter().enumerate().try_fold(SettingsState::default(), |before, (index, layer)| {
            let (after, _) = before.clone().apply(&layer.settings(), &name(index), &here());
            // A boolean that was at its stricter value stays there.
            if held(&before, claude, &enabled) == Some(Tree::Bool(true)) {
                prop_assert_eq!(held(&after, claude, &enabled), Some(Tree::Bool(true)));
            }
            if held(&before, claude, &auto_allow) == Some(Tree::Bool(false)) {
                prop_assert_eq!(held(&after, claude, &auto_allow), Some(Tree::Bool(false)));
            }
            // A list that may not gain entries holds no entry that it did not hold.
            if let Some(was) = held(&before, claude, &allowed) {
                let now = held(&after, claude, &allowed).map_or_else(Vec::new, |tree| members(&tree));
                prop_assert!(now.iter().all(|item| members(&was).contains(item)));
            }
            // A list that may only grow keeps every entry.
            if let Some(was) = held(&before, claude, &denied) {
                let now = held(&after, claude, &denied).map_or_else(Vec::new, |tree| members(&tree));
                prop_assert!(members(&was).iter().all(|item| now.contains(item)));
            }
            // An ordered value never moves down its ranks, and one in no rank never changes.
            if let Some(was) = held(&before, TargetName::Codex, &web_search) {
                let now = held(&after, TargetName::Codex, &web_search);
                prop_assert!(now.is_some());
                let now = now.unwrap_or(Tree::Null);
                if rank(&was).is_none_or(|rank| rank == 3) {
                    prop_assert_eq!(now, was);
                } else {
                    prop_assert!(rank(&now) >= rank(&was));
                }
            }
            Ok(after)
        })?;
    }
}

fn name(index: usize) -> LayerName {
    LayerName::try_from(format!("l{index}").as_str()).expect("a layer name")
}
