//! The tree of an agent's native fields (spec §7.2), which keeps the order the keys were written
//! in. A generated agent file writes them as YAML frontmatter, and a person reads it in the order
//! the source gave. Settings do not need this: their JSON output sorts keys (§6), so they use the
//! sorted [`Tree`].
//!
//! The deep merge is that of §7.9 in the order of the stack: a key keeps the place of its first
//! appearance, a key that a later layer adds comes after the existing ones, and a later value
//! replaces or merges in place without moving its key. A whole-number key is an ordinary key.

use std::collections::BTreeMap;

use crate::merge::{FloatBits, Tree};

/// A mapping with unique keys, in the order they first appeared. Equality is order-sensitive:
/// two mappings are equal when they hold the same entries in the same order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeMap(Vec<(String, NativeValue)>);

/// A value of a native field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeValue {
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
    List(Vec<NativeValue>),
    /// A mapping.
    Map(NativeMap),
}

impl NativeMap {
    /// The entries in order. A key that comes twice keeps the place of its first appearance and
    /// the value of its last.
    #[must_use]
    pub fn from_entries(entries: impl IntoIterator<Item = (String, NativeValue)>) -> Self {
        entries
            .into_iter()
            .fold(Self::default(), |map, (key, value)| {
                map.put(key, value, |_, later| later)
            })
    }

    /// The entries, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &NativeValue)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&NativeValue> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    /// How many entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The mapping with `key` set: in place when it is there, combining the old and new values
    /// with `combine`, at the end when it is not.
    fn put(
        self,
        key: String,
        value: NativeValue,
        combine: fn(NativeValue, NativeValue) -> NativeValue,
    ) -> Self {
        match self.0.iter().position(|(candidate, _)| *candidate == key) {
            Some(index) => Self(
                self.0
                    .into_iter()
                    .enumerate()
                    .map(|(at, (name, before))| {
                        if at == index {
                            (name, combine(before, value.clone()))
                        } else {
                            (name, before)
                        }
                    })
                    .collect(),
            ),
            None => Self(self.0.into_iter().chain([(key, value)]).collect()),
        }
    }

    /// The deep merge of §7.9, with `later` the more specific mapping.
    #[must_use]
    pub fn deep_merge(self, later: Self) -> Self {
        later.0.into_iter().fold(self, |merged, (key, value)| {
            merged.put(key, value, NativeValue::deep_merge)
        })
    }

    /// The same entries in key order, as values of a sorted tree.
    #[must_use]
    pub fn sorted(&self) -> BTreeMap<String, Tree> {
        self.0
            .iter()
            .map(|(key, value)| (key.clone(), value.to_tree()))
            .collect()
    }

    /// The same entries as a sorted tree, which compares mappings without regard to key order.
    #[must_use]
    pub fn to_tree(&self) -> Tree {
        Tree::Map(self.sorted())
    }
}

impl NativeValue {
    /// The merge of §7.9: two mappings merge key by key in place; two lists are concatenated and
    /// de-duplicated, keeping the first occurrence, where items are the same when they are equal
    /// as trees (mappings that differ only in key order are one item); otherwise `later` wins.
    #[must_use]
    pub fn deep_merge(self, later: Self) -> Self {
        match (self, later) {
            (Self::Map(earlier), Self::Map(later)) => Self::Map(earlier.deep_merge(later)),
            (Self::List(earlier), Self::List(later)) => Self::List(
                earlier
                    .into_iter()
                    .chain(later)
                    .fold(Vec::<(Tree, Self)>::new(), |kept, item| {
                        let tree = item.to_tree();
                        if kept.iter().any(|(seen, _)| *seen == tree) {
                            kept
                        } else {
                            kept.into_iter().chain([(tree, item)]).collect()
                        }
                    })
                    .into_iter()
                    .map(|(_, item)| item)
                    .collect(),
            ),
            (_, later) => later,
        }
    }

    /// The same value as a sorted tree.
    #[must_use]
    pub fn to_tree(&self) -> Tree {
        match self {
            Self::Null => Tree::Null,
            Self::Bool(flag) => Tree::Bool(*flag),
            Self::Int(number) => Tree::Int(*number),
            Self::Float(number) => Tree::Float(*number),
            Self::Text(text) => Tree::Text(text.clone()),
            Self::List(items) => Tree::List(items.iter().map(Self::to_tree).collect()),
            Self::Map(map) => map.to_tree(),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, prop_assert_eq, proptest};

    use super::{NativeMap, NativeValue};

    /// A mapping whose keys always hold the same kind: `a` and `b` numbers, `m` a mapping of
    /// numbers, `l` a list of numbers, so the merge is associative (as for [`Tree`](crate::merge::Tree)).
    fn maps() -> impl Strategy<Value = NativeMap> {
        let numbers =
            || proptest::collection::vec((0_u8..4).prop_map(|n| (n, i128::from(n))), 0..4);
        (
            proptest::option::of(0_i128..3),
            proptest::option::of(0_i128..3),
            proptest::option::of(numbers()),
            proptest::option::of(numbers()),
            proptest::sample::select(vec![0_u8, 1, 2, 3, 4, 5]),
        )
            .prop_map(|(a, b, inner, list, rotate)| {
                let mut entries: Vec<(String, NativeValue)> = [
                    a.map(|n| ("a".to_owned(), NativeValue::Int(n))),
                    b.map(|n| ("b".to_owned(), NativeValue::Int(n))),
                    inner.map(|items| {
                        (
                            "m".to_owned(),
                            NativeValue::Map(NativeMap::from_entries(
                                items
                                    .into_iter()
                                    .map(|(key, n)| (format!("k{key}"), NativeValue::Int(n))),
                            )),
                        )
                    }),
                    list.map(|items| {
                        (
                            "l".to_owned(),
                            NativeValue::List(
                                items
                                    .into_iter()
                                    .map(|(_, n)| NativeValue::Int(n))
                                    .collect(),
                            ),
                        )
                    }),
                ]
                .into_iter()
                .flatten()
                .collect();
                let turn = usize::from(rotate) % entries.len().max(1);
                entries.rotate_left(turn);
                NativeMap::from_entries(entries)
            })
    }

    proptest! {
        #[test]
        fn the_deep_merge_is_associative_in_value_and_in_key_order(a in maps(), b in maps(), c in maps()) {
            let left = a.clone().deep_merge(b.clone()).deep_merge(c.clone());
            let right = a.deep_merge(b.deep_merge(c));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn a_key_keeps_the_place_of_its_first_appearance(a in maps(), b in maps()) {
            let merged = a.clone().deep_merge(b);
            let first: Vec<&str> = a.iter().map(|(key, _)| key).collect();
            let after: Vec<&str> = merged.iter().map(|(key, _)| key).take(first.len()).collect();
            prop_assert_eq!(first, after);
        }
    }
}
