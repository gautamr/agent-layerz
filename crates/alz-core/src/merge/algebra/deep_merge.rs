//! The deep merge of settings and native fields (spec §7.9): a left fold in stack order over
//! alz's own ordered tree.

use std::collections::BTreeMap;

use super::OrderedSet;

/// A float compared by its bits, so a tree is [`Eq`]: `NaN` equals `NaN`, and `0.0` differs from
/// `-0.0`. Two items of a list are the same item when they are equal in this sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FloatBits(u64);

impl FloatBits {
    /// The float, by its bits.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value.to_bits())
    }

    /// The float.
    #[must_use]
    pub const fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// A settings value. A mapping is a [`BTreeMap`], so two mappings that differ only in the order
/// their keys were written in are equal, and output never depends on hashing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tree {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i128),
    /// Any other number.
    Float(FloatBits),
    /// Text.
    Text(String),
    /// A list.
    List(Vec<Tree>),
    /// A mapping.
    Map(BTreeMap<String, Tree>),
}

impl Tree {
    /// The left fold of §7.9, with `later` the more specific tree:
    ///
    /// - two mappings merge key by key, and a key in only one stays;
    /// - two lists are concatenated and de-duplicated, keeping the first occurrence of an item,
    ///   where items are the same when they are equal as trees;
    /// - in every other case, a change of kind included, `later` replaces `self`.
    ///
    /// The merge is associative over kind-consistent trees, those where every path holds a list,
    /// a mapping or a scalar in all the trees merged. It is not associative when a path changes
    /// kind (a list, then a scalar, then a list): the fold gives the last list, and merging the
    /// last two first gives both lists. The fold is what the spec says and what the owner
    /// kept (TASK-55.5, answer 5), so the law is stated for kind-consistent trees only.
    ///
    /// The recursion is as deep as the tree, which the YAML and JSON limits bound.
    #[must_use]
    pub fn deep_merge(self, later: Self) -> Self {
        match (self, later) {
            (Self::Map(earlier), Self::Map(later)) => {
                Self::Map(later.into_iter().fold(earlier, |mut merged, (key, value)| {
                    let combined = match merged.remove(&key) {
                        Some(before) => before.deep_merge(value),
                        None => value,
                    };
                    merged.insert(key, combined);
                    merged
                }))
            }
            (Self::List(earlier), Self::List(later)) => Self::List(
                earlier
                    .into_iter()
                    .chain(later)
                    .collect::<OrderedSet<Self>>()
                    .into_vec(),
            ),
            (_, later) => later,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::{BoxedStrategy, Just, Strategy, prop_assert_eq, proptest};

    use super::{FloatBits, Tree};

    fn text(value: &str) -> Tree {
        Tree::Text(value.to_owned())
    }

    fn map(entries: Vec<(&str, Tree)>) -> Tree {
        Tree::Map(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    #[test]
    fn a_later_scalar_replaces_an_earlier_one() {
        assert_eq!(text("a").deep_merge(Tree::Int(2)), Tree::Int(2));
    }

    #[test]
    fn a_later_kind_replaces_an_earlier_one() {
        let list = Tree::List(vec![Tree::Int(1)]);
        let scalar = Tree::Int(2);
        let mapping = map(vec![("k", Tree::Null)]);
        assert_eq!(list.clone().deep_merge(scalar.clone()), scalar);
        assert_eq!(scalar.clone().deep_merge(list.clone()), list);
        assert_eq!(mapping.clone().deep_merge(list.clone()), list);
        assert_eq!(list.deep_merge(mapping.clone()), mapping);
    }

    #[test]
    fn mappings_merge_key_by_key_and_keep_keys_found_in_one_only() {
        let earlier = map(vec![
            ("keep", Tree::Int(1)),
            ("both", map(vec![("x", Tree::Int(1)), ("y", Tree::Int(2))])),
        ]);
        let later = map(vec![
            ("both", map(vec![("y", Tree::Int(3)), ("z", Tree::Int(4))])),
            ("new", Tree::Bool(true)),
        ]);
        assert_eq!(
            earlier.deep_merge(later),
            map(vec![
                ("keep", Tree::Int(1)),
                (
                    "both",
                    map(vec![
                        ("x", Tree::Int(1)),
                        ("y", Tree::Int(3)),
                        ("z", Tree::Int(4)),
                    ])
                ),
                ("new", Tree::Bool(true)),
            ])
        );
    }

    #[test]
    fn two_lists_concatenate_and_keep_the_first_occurrence_of_an_item() {
        let earlier = Tree::List(vec![Tree::Int(1), Tree::Int(2)]);
        let later = Tree::List(vec![Tree::Int(2), Tree::Int(3)]);
        assert_eq!(
            earlier.deep_merge(later),
            Tree::List(vec![Tree::Int(1), Tree::Int(2), Tree::Int(3)])
        );
    }

    #[test]
    fn list_items_equal_as_ordered_trees_appear_once_whatever_the_order_of_their_keys() {
        let written_one_way = Tree::Map(
            [
                ("a".to_owned(), Tree::Int(1)),
                ("b".to_owned(), Tree::Int(2)),
            ]
            .into_iter()
            .collect(),
        );
        let written_the_other = Tree::Map(
            [
                ("b".to_owned(), Tree::Int(2)),
                ("a".to_owned(), Tree::Int(1)),
            ]
            .into_iter()
            .collect(),
        );
        let merged = Tree::List(vec![written_one_way.clone()])
            .deep_merge(Tree::List(vec![written_the_other]));
        assert_eq!(merged, Tree::List(vec![written_one_way]));
    }

    #[test]
    fn a_float_is_the_same_item_as_itself_even_when_it_is_not_a_number() {
        let nan = Tree::Float(FloatBits::new(f64::NAN));
        let merged = Tree::List(vec![nan.clone()]).deep_merge(Tree::List(vec![nan.clone()]));
        assert_eq!(merged, Tree::List(vec![nan]));
    }

    #[test]
    fn a_path_that_changes_kind_is_not_associative_which_is_why_the_law_is_for_consistent_trees() {
        let list = |item: i128| map(vec![("k", Tree::List(vec![Tree::Int(item)]))]);
        let scalar = map(vec![("k", Tree::Int(0))]);
        let left = list(1).deep_merge(scalar.clone()).deep_merge(list(3));
        let right = list(1).deep_merge(scalar.deep_merge(list(3)));
        assert_eq!(left, list(3));
        assert_eq!(
            right,
            map(vec![("k", Tree::List(vec![Tree::Int(1), Tree::Int(3)]))])
        );
    }

    /// The kinds along every path of a family of trees: each path is a scalar, a list or a mapping
    /// in all of them.
    #[derive(Debug, Clone)]
    enum Shape {
        Scalar,
        List,
        Map(BTreeMap<String, Shape>),
    }

    fn shapes() -> impl Strategy<Value = Shape> {
        proptest::prop_oneof![Just(Shape::Scalar), Just(Shape::List)].prop_recursive(
            3,
            12,
            3,
            |inner| proptest::collection::btree_map("[a-c]", inner, 0..4).prop_map(Shape::Map),
        )
    }

    fn scalars() -> BoxedStrategy<Tree> {
        proptest::prop_oneof![
            Just(Tree::Null),
            proptest::bool::ANY.prop_map(Tree::Bool),
            (0_i128..4).prop_map(Tree::Int),
            "[a-b]{0,2}".prop_map(Tree::Text),
        ]
        .boxed()
    }

    fn items() -> BoxedStrategy<Tree> {
        scalars()
            .prop_recursive(2, 8, 3, |inner| {
                proptest::prop_oneof![
                    proptest::collection::vec(inner.clone(), 0..3).prop_map(Tree::List),
                    proptest::collection::btree_map("[a-b]", inner, 0..3).prop_map(Tree::Map),
                ]
            })
            .boxed()
    }

    /// A tree of the given shape, where any key of a mapping may be missing.
    fn tree_of(shape: &Shape) -> BoxedStrategy<Tree> {
        match shape {
            Shape::Scalar => scalars(),
            Shape::List => proptest::collection::vec(items(), 0..4)
                .prop_map(Tree::List)
                .boxed(),
            Shape::Map(fields) => fields
                .iter()
                .map(|(key, field)| {
                    let key = key.clone();
                    proptest::option::of(tree_of(field))
                        .prop_map(move |value| value.map(|value| (key.clone(), value)))
                })
                .collect::<Vec<_>>()
                .prop_map(|entries| Tree::Map(entries.into_iter().flatten().collect()))
                .boxed(),
        }
    }

    fn consistent_triples() -> impl Strategy<Value = (Tree, Tree, Tree)> {
        shapes().prop_flat_map(|shape| (tree_of(&shape), tree_of(&shape), tree_of(&shape)))
    }

    proptest! {
        #[test]
        fn merging_kind_consistent_trees_is_associative((a, b, c) in consistent_triples()) {
            prop_assert_eq!(
                a.clone().deep_merge(b.clone()).deep_merge(c.clone()),
                a.deep_merge(b.deep_merge(c)),
            );
        }

        #[test]
        fn a_tree_merged_with_itself_is_settled_so_merging_it_again_changes_nothing(
            (tree, _, _) in consistent_triples(),
        ) {
            let settled = tree.clone().deep_merge(tree);
            prop_assert_eq!(settled.clone().deep_merge(settled.clone()), settled);
        }
    }
}
