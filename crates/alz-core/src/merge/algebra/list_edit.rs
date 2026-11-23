//! The edits of a list a layer may write: `+[..]`, `-[..]` and `[..]` (spec §7.2).

use super::OrderedSet;

/// What one layer does to an inherited list. `[..]` replaces it, and `-[..]` and `+[..]`, which
/// may come together, remove items and then append items.
///
/// Edits compose ([`ListEdit::merge`]): applying the composition of two edits equals applying
/// them one after the other, and composition is associative, so a stack of layers can be folded
/// into one edit before it meets the base list. The identity is [`ListEdit::default`], the edit
/// that removes and adds nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListEdit<T> {
    /// `[..]`: the list is exactly these items.
    Replace(OrderedSet<T>),
    /// `-[..] +[..]`: drop the items in `remove`, then append those of `add` that are not there.
    Adjust {
        /// Items to drop.
        remove: OrderedSet<T>,
        /// Items to append.
        add: OrderedSet<T>,
    },
}

impl<T> Default for ListEdit<T> {
    /// The edit that changes nothing.
    fn default() -> Self {
        Self::Adjust {
            remove: OrderedSet::default(),
            add: OrderedSet::default(),
        }
    }
}

impl<T: Eq + Clone> ListEdit<T> {
    /// The list after the edit.
    #[must_use]
    pub fn apply(&self, base: &OrderedSet<T>) -> OrderedSet<T> {
        match self {
            Self::Replace(items) => items.clone(),
            Self::Adjust { remove, add } => base
                .iter()
                .filter(|item| !remove.contains(item))
                .chain(add.iter())
                .cloned()
                .collect(),
        }
    }

    /// The single edit that does what `self` and then `later` do.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        match (self, later) {
            (_, replacement @ Self::Replace(_)) => replacement,
            (Self::Replace(items), adjustment @ Self::Adjust { .. }) => {
                Self::Replace(adjustment.apply(&items))
            }
            (
                Self::Adjust {
                    remove: first_remove,
                    add: first_add,
                },
                Self::Adjust {
                    remove: later_remove,
                    add: later_add,
                },
            ) => Self::Adjust {
                add: first_add.without(&later_remove).merge(later_add),
                remove: first_remove.merge(later_remove),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, prop_assert_eq, proptest};

    use super::ListEdit;
    use crate::merge::algebra::OrderedSet;

    fn sets() -> impl Strategy<Value = OrderedSet<u8>> {
        proptest::collection::vec(0_u8..6, 0..6).prop_map(OrderedSet::from_iter)
    }

    fn edits() -> impl Strategy<Value = ListEdit<u8>> {
        proptest::prop_oneof![
            sets().prop_map(ListEdit::Replace),
            (sets(), sets()).prop_map(|(remove, add)| ListEdit::Adjust { remove, add }),
        ]
    }

    fn set(items: &[u8]) -> OrderedSet<u8> {
        items.iter().copied().collect()
    }

    #[test]
    fn remove_then_add_edits_a_list_in_order() {
        let edit = ListEdit::Adjust {
            remove: set(&[2]),
            add: set(&[9]),
        };
        assert_eq!(edit.apply(&set(&[1, 2, 3])).as_slice(), [1, 3, 9]);
    }

    #[test]
    fn a_replacement_ignores_the_inherited_list() {
        assert_eq!(
            ListEdit::Replace(set(&[7])).apply(&set(&[1, 2])).as_slice(),
            [7]
        );
    }

    #[test]
    fn a_removal_of_what_an_earlier_layer_added_leaves_nothing_to_add() {
        let added = ListEdit::Adjust {
            remove: OrderedSet::default(),
            add: set(&[5]),
        };
        let removed = ListEdit::Adjust {
            remove: set(&[5]),
            add: OrderedSet::default(),
        };
        assert_eq!(added.merge(removed).apply(&set(&[1])).as_slice(), [1],);
    }

    proptest! {
        #[test]
        fn composition_is_associative(a in edits(), b in edits(), c in edits()) {
            prop_assert_eq!(
                a.clone().merge(b.clone()).merge(c.clone()),
                a.merge(b.merge(c)),
            );
        }

        #[test]
        fn applying_a_composition_equals_applying_the_edits_in_turn(
            first in edits(),
            second in edits(),
            base in sets(),
        ) {
            let in_turn = second.apply(&first.apply(&base));
            prop_assert_eq!(first.merge(second).apply(&base), in_turn);
        }

        #[test]
        fn the_edit_that_changes_nothing_is_the_identity_of_composition(a in edits(), base in sets()) {
            let identity = ListEdit::default();
            prop_assert_eq!(identity.clone().merge(a.clone()).apply(&base), a.apply(&base));
            prop_assert_eq!(a.clone().merge(identity).apply(&base), a.apply(&base));
        }

        #[test]
        fn the_edit_that_changes_nothing_leaves_a_list_as_it_is(base in sets()) {
            prop_assert_eq!(ListEdit::default().apply(&base), base);
        }
    }
}
