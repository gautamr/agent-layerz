//! A list that holds each item once, in the order it first appeared.

/// A union of lists that keeps the first occurrence of each item, as the allow, ask and deny
/// lists, the paths of a rule and the order of hooks need. The field is private and every
/// constructor removes duplicates, so an `OrderedSet` never holds an item twice.
///
/// Merging is associative and idempotent, and commutative as a set ([`OrderedSet::same_members`]),
/// though not as a sequence: the earlier list sets the order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderedSet<T>(Vec<T>);

impl<T> Default for OrderedSet<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> OrderedSet<T> {
    /// The items in order.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// The items in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.0.iter()
    }

    /// How many items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is no item.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The items in order.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T: Eq> OrderedSet<T> {
    /// Whether `item` is in the set.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool {
        self.0.contains(item)
    }

    /// The items of `self` then those of `later` that are not already there.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        self.0.into_iter().chain(later.0).collect()
    }

    /// The items of `self` that are not in `other`, in order.
    #[must_use]
    pub fn without(self, other: &Self) -> Self {
        Self(
            self.0
                .into_iter()
                .filter(|item| !other.contains(item))
                .collect(),
        )
    }

    /// Whether both hold the same items, whatever their order.
    #[must_use]
    pub fn same_members(&self, other: &Self) -> bool {
        self.0.iter().all(|item| other.contains(item))
            && other.0.iter().all(|item| self.contains(item))
    }
}

impl<T: Eq> FromIterator<T> for OrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self(items.into_iter().fold(Vec::new(), |mut kept, item| {
            if !kept.contains(&item) {
                kept.push(item);
            }
            kept
        }))
    }
}

impl<T> IntoIterator for OrderedSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use proptest::collection::vec;
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::OrderedSet;

    pub(crate) fn sets() -> impl Strategy<Value = OrderedSet<u8>> {
        vec(0_u8..8, 0..8).prop_map(OrderedSet::from_iter)
    }

    #[test]
    fn the_first_occurrence_keeps_its_place() {
        let set: OrderedSet<u8> = [3, 1, 3, 2, 1].into_iter().collect();
        assert_eq!(set.as_slice(), [3, 1, 2]);
    }

    #[test]
    fn the_earlier_list_sets_the_order_of_a_merge() {
        let earlier: OrderedSet<u8> = [1, 2].into_iter().collect();
        let later: OrderedSet<u8> = [2, 3, 0].into_iter().collect();
        assert_eq!(earlier.merge(later).as_slice(), [1, 2, 3, 0]);
    }

    proptest! {
        #[test]
        fn merging_is_associative(a in sets(), b in sets(), c in sets()) {
            prop_assert_eq!(
                a.clone().merge(b.clone()).merge(c.clone()),
                a.merge(b.merge(c)),
            );
        }

        #[test]
        fn merging_is_idempotent(a in sets()) {
            prop_assert_eq!(a.clone().merge(a.clone()), a);
        }

        #[test]
        fn merging_is_commutative_as_a_set(a in sets(), b in sets()) {
            prop_assert!(a.clone().merge(b.clone()).same_members(&b.merge(a)));
        }

        #[test]
        fn the_empty_set_is_the_identity(a in sets()) {
            prop_assert_eq!(OrderedSet::default().merge(a.clone()), a.clone());
            prop_assert_eq!(a.clone().merge(OrderedSet::default()), a);
        }

        #[test]
        fn no_item_is_held_twice(items in vec(any::<u8>(), 0..32)) {
            let set: OrderedSet<u8> = items.into_iter().collect();
            let distinct = set.iter().enumerate().all(|(index, item)| {
                !set.iter().skip(index + 1).any(|other| other == item)
            });
            prop_assert!(distinct);
        }
    }
}
