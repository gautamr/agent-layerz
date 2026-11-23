//! A value that only a stricter one may replace.

/// The strictest value any layer has set, for an ordered type whose larger values are stricter
/// (`Allow < Ask < Deny`, a permission mode, a level). Merging is the join, `max`, so it is
/// associative, commutative and idempotent, and monotone: a later layer can raise the value and
/// never lower it. "Unset" is below every value and is the identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Floor<T>(Option<T>);

impl<T> Default for Floor<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Ord> Floor<T> {
    /// A floor at `value`.
    #[must_use]
    pub const fn at(value: T) -> Self {
        Self(Some(value))
    }

    /// The stricter of the two floors.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        Self(self.0.max(later.0))
    }

    /// The floor, when any layer set one.
    #[must_use]
    pub const fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Whether `candidate` is at least as strict as the floor: always so when no floor is set.
    #[must_use]
    pub fn admits(&self, candidate: &T) -> bool {
        self.0.as_ref().is_none_or(|floor| candidate >= floor)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::Floor;

    /// The permission modes, loosest first, as a stand-in for any ordered scale.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Mode {
        Allow,
        Ask,
        Deny,
    }

    fn modes() -> impl Strategy<Value = Mode> {
        proptest::prop_oneof![
            proptest::strategy::Just(Mode::Allow),
            proptest::strategy::Just(Mode::Ask),
            proptest::strategy::Just(Mode::Deny),
        ]
    }

    fn floors() -> impl Strategy<Value = Floor<Mode>> {
        proptest::option::of(modes()).prop_map(|mode| mode.map_or_else(Floor::default, Floor::at))
    }

    #[test]
    fn a_later_layer_cannot_loosen_the_floor() {
        let merged = Floor::at(Mode::Deny).merge(Floor::at(Mode::Allow));
        assert_eq!(merged.get(), Some(&Mode::Deny));
    }

    #[test]
    fn a_later_layer_can_raise_the_floor() {
        let merged = Floor::at(Mode::Ask).merge(Floor::at(Mode::Deny));
        assert_eq!(merged.get(), Some(&Mode::Deny));
    }

    proptest! {
        #[test]
        fn merging_is_associative(a in floors(), b in floors(), c in floors()) {
            prop_assert_eq!(
                a.clone().merge(b.clone()).merge(c.clone()),
                a.merge(b.merge(c)),
            );
        }

        #[test]
        fn merging_is_commutative(a in floors(), b in floors()) {
            prop_assert_eq!(a.clone().merge(b.clone()), b.merge(a));
        }

        #[test]
        fn merging_is_idempotent(a in floors()) {
            prop_assert_eq!(a.clone().merge(a.clone()), a);
        }

        #[test]
        fn unset_is_the_identity(a in floors()) {
            prop_assert_eq!(Floor::default().merge(a.clone()), a.clone());
            prop_assert_eq!(a.clone().merge(Floor::default()), a);
        }

        #[test]
        fn the_merge_is_monotone_and_never_below_the_floor_it_started_from(
            floor in floors(),
            later in floors(),
        ) {
            let merged = floor.clone().merge(later.clone());
            prop_assert!(merged >= floor);
            prop_assert!(merged >= later);
        }

        #[test]
        fn a_value_the_floor_admits_is_admitted_by_every_looser_floor(
            floor in floors(),
            looser in floors(),
            candidate in modes(),
        ) {
            let merged = floor.clone().merge(looser);
            prop_assert!(!merged.admits(&candidate) || floor.admits(&candidate));
        }

        #[test]
        fn an_unset_floor_admits_everything(candidate in any::<u8>()) {
            prop_assert!(Floor::<u8>::default().admits(&candidate));
        }
    }
}
