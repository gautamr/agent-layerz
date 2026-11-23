//! A scalar where the most specific value wins.

/// A value that a layer may leave unset. Merging keeps the later value when it is set and the
/// earlier one when not: the last-writer-wins monoid, associative, with "unset" as identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Last<T>(Option<T>);

impl<T> Default for Last<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> Last<T> {
    /// A value that is set.
    #[must_use]
    pub const fn set(value: T) -> Self {
        Self(Some(value))
    }

    /// `later` when it is set, otherwise `self`.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        Self(later.0.or(self.0))
    }

    /// The value, when one layer set it.
    #[must_use]
    pub const fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// The value, when one layer set it.
    #[must_use]
    pub fn into_inner(self) -> Option<T> {
        self.0
    }
}

impl<T> From<Option<T>> for Last<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert_eq, proptest};

    use super::Last;

    #[test]
    fn the_later_value_wins_when_both_are_set() {
        assert_eq!(Last::set(1).merge(Last::set(2)).get(), Some(&2));
    }

    #[test]
    fn an_unset_later_value_keeps_the_earlier_one() {
        assert_eq!(Last::set(1).merge(Last::default()).get(), Some(&1));
    }

    proptest! {
        #[test]
        fn merging_is_associative(
            a in any::<Option<u8>>(),
            b in any::<Option<u8>>(),
            c in any::<Option<u8>>(),
        ) {
            let (a, b, c) = (Last::from(a), Last::from(b), Last::from(c));
            prop_assert_eq!(
                a.clone().merge(b.clone()).merge(c.clone()),
                a.merge(b.merge(c)),
            );
        }

        #[test]
        fn unset_is_the_identity_on_both_sides(a in any::<Option<u8>>()) {
            let a = Last::from(a);
            prop_assert_eq!(Last::default().merge(a.clone()), a.clone());
            prop_assert_eq!(a.clone().merge(Last::default()), a);
        }
    }
}
