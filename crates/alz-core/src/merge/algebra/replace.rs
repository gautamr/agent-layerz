//! A map where the most specific value wins per key.

use std::collections::BTreeMap;

/// Items with a name, such as skills, commands and MCP servers, where a later layer's item
/// replaces an earlier one of the same name and the rest are kept. The map is ordered, so output
/// never depends on hashing. Merging is associative, with the empty map as identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Replace<K, V>(BTreeMap<K, V>);

impl<K, V> Default for Replace<K, V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<K: Ord, V> Replace<K, V> {
    /// `self` with every entry of `later` added, an entry of `later` replacing one of the same key.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        self.0.into_iter().chain(later.0).collect()
    }

    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    /// The entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.0.iter()
    }

    /// How many entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is no entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for Replace<K, V> {
    /// The map of the entries, a later one replacing an earlier one of the same key.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use proptest::collection::btree_map;
    use proptest::prelude::{Strategy, prop_assert_eq, proptest};

    use super::Replace;

    fn maps() -> impl Strategy<Value = Replace<u8, u8>> {
        btree_map(0_u8..6, 0_u8..4, 0..6).prop_map(|map| map.into_iter().collect())
    }

    #[test]
    fn the_later_value_wins_per_key_and_other_keys_stay() {
        let earlier: Replace<u8, &str> = [(1, "a"), (2, "b")].into_iter().collect();
        let later: Replace<u8, &str> = [(2, "c"), (3, "d")].into_iter().collect();
        let merged = earlier.merge(later);
        assert_eq!(
            merged.iter().collect::<Vec<_>>(),
            [(&1, &"a"), (&2, &"c"), (&3, &"d")]
        );
    }

    proptest! {
        #[test]
        fn merging_is_associative(a in maps(), b in maps(), c in maps()) {
            prop_assert_eq!(
                a.clone().merge(b.clone()).merge(c.clone()),
                a.merge(b.merge(c)),
            );
        }

        #[test]
        fn the_empty_map_is_the_identity_on_both_sides(a in maps()) {
            prop_assert_eq!(Replace::default().merge(a.clone()), a.clone());
            prop_assert_eq!(a.clone().merge(Replace::default()), a);
        }
    }
}
