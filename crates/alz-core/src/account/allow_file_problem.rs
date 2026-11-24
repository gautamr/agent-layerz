//! What is wrong with `allowed.yaml` as a whole (spec §7.13 "Both files", §10.3): one value, one
//! text. `alz account allow` refuses with it and the account note of `alz build` and `alz check`
//! names it, so a change to the words is a change in one place.

use std::num::NonZeroUsize;

use crate::diagnostic::Message;

/// What is wrong with `allowed.yaml` as a whole. No value holds a word of the file but the reason
/// it was ignored, which alz worded itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowFileProblem {
    /// A check of "Both files" failed, or the file is not valid YAML; the reason as the report
    /// words it.
    Ignored(Message),
    /// The top level is not a list.
    NotAList,
    /// These entries, counted from 1 and never empty, are ignored.
    Entries(Vec<NonZeroUsize>),
}

impl AllowFileProblem {
    /// The problem as `alz account allow` states it, without the closing `nothing was changed`:
    /// the one text of the refusal (§10.3) and of the account note (§10.3 "The account note").
    #[must_use]
    pub fn state(&self) -> Message {
        match self {
            Self::Ignored(reason) => Message::new("allowed.yaml is ignored (")
                .and(reason.clone())
                .then("); fix it or delete it, alz does not repair it"),
            Self::NotAList => Message::new(
                "allowed.yaml is not a list; fix it or delete it, alz does not repair it",
            ),
            Self::Entries(numbers) => {
                let listed = numbers.iter().enumerate().fold(
                    Message::new(""),
                    |message, (index, number)| {
                        let separated = if index == 0 {
                            message
                        } else {
                            message.then(", ")
                        };
                        separated.then(number.get())
                    },
                );
                let noun = if numbers.len() == 1 {
                    "entry "
                } else {
                    "entries "
                };
                Message::new("allowed.yaml has ")
                    .then(noun)
                    .and(listed)
                    .then(
                        " that alz ignores (alz account which warns about them); fix them or delete the file, alz does not repair it",
                    )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(problem: &AllowFileProblem) -> String {
        problem.state().to_string()
    }

    #[test]
    fn an_ignored_file_gives_the_readers_reason() {
        assert_eq!(
            text(&AllowFileProblem::Ignored(Message::new(
                "group or others can write it"
            ))),
            "allowed.yaml is ignored (group or others can write it); fix it or delete it, alz does not repair it"
        );
    }

    #[test]
    fn a_file_that_is_not_a_list_is_named_so() {
        assert_eq!(
            text(&AllowFileProblem::NotAList),
            "allowed.yaml is not a list; fix it or delete it, alz does not repair it"
        );
    }

    #[test]
    fn ignored_entries_are_named_by_position_in_the_singular_and_the_plural() {
        let numbers = |counted: &[usize]| {
            AllowFileProblem::Entries(
                counted
                    .iter()
                    .map(|number| NonZeroUsize::new(*number).expect("counted from 1"))
                    .collect(),
            )
        };
        assert_eq!(
            text(&numbers(&[3])),
            "allowed.yaml has entry 3 that alz ignores (alz account which warns about them); fix them or delete the file, alz does not repair it"
        );
        assert_eq!(
            text(&numbers(&[1, 2])),
            "allowed.yaml has entries 1, 2 that alz ignores (alz account which warns about them); fix them or delete the file, alz does not repair it"
        );
    }
}
