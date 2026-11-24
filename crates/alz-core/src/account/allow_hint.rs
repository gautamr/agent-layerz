//! The account note of `alz build` and `alz check` (spec §7.13 "Trust", §10.3 "The account
//! note"): the one line that tells a person to run `alz account allow`, decided from what the
//! layers declare, what `allowed.yaml` says for this root and whether the file can be trusted.
//!
//! The decision is pure. Whether it is printed at all (a person at a terminal, a root where an
//! account can apply) is the caller's.

use crate::account::{
    AccountChoice, AllowEntry, AllowFileProblem, CachedDeclaration, DeclaredAccount, MissReason,
};
use crate::diagnostic::{Code, Diagnostic, Message};
use crate::layer::LayerName;
use crate::target::TargetName;
use crate::untrusted_text::{Escaped, Stripped};

/// What the note is told besides the layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintFacts<'a> {
    /// What the layers read from the cache declare.
    pub declaration: &'a CachedDeclaration,
    /// The entry of `allowed.yaml` for this repository's root.
    pub entry: Option<&'a AllowEntry>,
    /// A problem that makes `allowed.yaml` unusable as a whole.
    pub problem: Option<&'a AllowFileProblem>,
    /// The merged configuration names an account. Without it the feature is off here and a cache
    /// miss prints nothing.
    pub declares: bool,
}

/// A map of targets to account names as the note shows it, in target order.
type Shown = Vec<(TargetName, String)>;

/// The note, one case for each line of §10.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowHint {
    /// The layers name an account that this repository is not allowed.
    NotAllowed {
        /// The layer that declares.
        layer: LayerName,
        /// What it declares, per target.
        map: Shown,
    },
    /// An allowed entry no longer matches the declaration.
    Changed {
        /// The recorded map and source.
        was: (Shown, String),
        /// The declared map and source.
        now: (Shown, String),
    },
    /// This repository is allowed an account that its layers no longer name.
    Withdraw,
    /// The layers name an account, but `allowed.yaml` cannot be used.
    Unusable(AllowFileProblem),
    /// The cache cannot say what the layers declare.
    NotChecked(MissReason),
}

/// The note to print, or `None` when there is nothing to say.
#[must_use]
pub fn allow_hint(facts: &HintFacts<'_>) -> Option<AllowHint> {
    match facts.declaration {
        CachedDeclaration::Miss(miss) => {
            facts.declares.then(|| AllowHint::NotChecked(miss.reason()))
        }
        CachedDeclaration::Conflict { .. } => None,
        CachedDeclaration::None { .. } => match (facts.problem, facts.entry) {
            (None, Some(_)) => Some(AllowHint::Withdraw),
            (Some(_), _) | (None, None) => None,
        },
        CachedDeclaration::Declared { declaration, .. } => match (facts.problem, facts.entry) {
            (Some(problem), _) => Some(AllowHint::Unusable(problem.clone())),
            (None, None) => Some(AllowHint::NotAllowed {
                layer: declaration.layer.clone(),
                map: declared_map(declaration),
            }),
            (None, Some(entry)) => changed(entry, declaration),
        },
    }
}

fn declared_map(declaration: &DeclaredAccount) -> Shown {
    TargetName::ALL
        .into_iter()
        .filter_map(|target| {
            declaration
                .map
                .choice_for(target)
                .map(|choice| (target, choice_text(choice)))
        })
        .collect()
}

fn recorded_map(entry: &AllowEntry) -> Shown {
    entry
        .account()
        .iter()
        .map(|(target, name)| (target, name.to_string()))
        .collect()
}

fn choice_text(choice: &AccountChoice) -> String {
    match choice {
        AccountChoice::Default => "default".to_owned(),
        AccountChoice::Named(name) => name.to_string(),
    }
}

fn changed(entry: &AllowEntry, declaration: &DeclaredAccount) -> Option<AllowHint> {
    let was = recorded_map(entry);
    let now = declared_map(declaration);
    let same = was == now && entry.source().as_str() == declaration.recorded_source;
    (!same).then(|| AllowHint::Changed {
        was: (was, entry.source().as_str().to_owned()),
        now: (now, declaration.recorded_source.clone()),
    })
}

/// `claude=client-a, codex=client-a-api`. Account names are validated, so they print as they are.
fn map_text(map: &[(TargetName, String)]) -> Message {
    map.iter()
        .enumerate()
        .fold(Message::new(""), |message, (index, (target, name))| {
            let separated = if index == 0 {
                message
            } else {
                message.then(", ")
            };
            separated
                .then(target.as_str())
                .then("=")
                .then(Stripped::from_untrusted(name))
        })
}

impl AllowHint {
    /// The note as a diagnostic: `note [account-allow] <text>`, every text of the repository
    /// written as §10.1 class 2, and no place.
    #[must_use]
    pub fn into_diagnostic(self) -> Diagnostic {
        Diagnostic::note(Code::AccountAllow, None, self.text())
    }

    fn text(self) -> Message {
        match self {
            Self::NotAllowed { layer, map } => Message::new("the layer ")
                .then(Escaped::from_untrusted(layer.as_str()))
                .then(" names ")
                .and(map_text(&map))
                .then("; it applies only after you run: alz account allow"),
            Self::Changed { was, now } => {
                Message::new("the account declaration changed since it was allowed: was ")
                    .and(map_text(&was.0))
                    .then(" from ")
                    .then(Escaped::from_untrusted(&was.1))
                    .then(", now ")
                    .and(map_text(&now.0))
                    .then(" from ")
                    .then(Escaped::from_untrusted(&now.1))
                    .then("; run: alz account allow")
            }
            Self::Withdraw => Message::new(
                "this repository is allowed an account that its layers no longer name; run: alz account allow --remove",
            ),
            Self::Unusable(problem) => Message::new("the layers name an account, but ")
                .and(problem.state())
                .then("; then run: alz account allow"),
            Self::NotChecked(miss) => {
                Message::new("the account declaration was not checked: ").then(miss.phrase())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::account::{AccountName, AllowedSource, Miss, NonEmptyTargetMap};
    use crate::path::AbsolutePath;

    fn entry() -> AllowEntry {
        AllowEntry::new(
            AbsolutePath::try_from("/work/shop").expect("absolute"),
            NonEmptyTargetMap::from_entries([(
                TargetName::Claude,
                AccountName::try_from("client-a").expect("a name"),
            )])
            .expect("one target"),
            AllowedSource::try_from("/hub/layer").expect("a source"),
        )
    }

    fn none() -> CachedDeclaration {
        CachedDeclaration::None {
            ignored: Vec::new(),
        }
    }

    fn line(hint: AllowHint) -> String {
        hint.into_diagnostic().to_string()
    }

    #[test]
    fn a_cache_miss_is_named_only_when_the_layers_name_an_account() {
        let miss = CachedDeclaration::Miss(Miss::of(MissReason::LockMissing));
        let facts = |declares| HintFacts {
            declaration: &miss,
            entry: None,
            problem: None,
            declares,
        };
        assert_eq!(allow_hint(&facts(false)), None);
        assert_eq!(
            allow_hint(&facts(true)).map(line).as_deref(),
            Some(
                "note [account-allow] the account declaration was not checked: layer.lock is missing"
            )
        );
    }

    #[test]
    fn an_allowed_root_whose_layers_declare_nothing_is_told_to_withdraw() {
        let declaration = none();
        let allowed = entry();
        let facts = HintFacts {
            declaration: &declaration,
            entry: Some(&allowed),
            problem: None,
            declares: false,
        };
        assert_eq!(
            allow_hint(&facts).map(line).as_deref(),
            Some(
                "note [account-allow] this repository is allowed an account that its layers no longer name; run: alz account allow --remove"
            )
        );
    }

    #[test]
    fn nothing_is_said_when_nobody_declares_and_nothing_is_allowed() {
        let declaration = none();
        let facts = HintFacts {
            declaration: &declaration,
            entry: None,
            problem: None,
            declares: false,
        };
        assert_eq!(allow_hint(&facts), None);
    }

    #[test]
    fn a_conflict_prints_nothing() {
        let declaration = CachedDeclaration::Conflict {
            clients: Vec::new(),
            ignored: Vec::new(),
        };
        let allowed = entry();
        let facts = HintFacts {
            declaration: &declaration,
            entry: Some(&allowed),
            problem: None,
            declares: true,
        };
        assert_eq!(allow_hint(&facts), None);
    }

    #[test]
    fn an_unusable_file_names_its_positions_only() {
        let problem = AllowFileProblem::Entries(vec![
            NonZeroUsize::new(1).expect("one"),
            NonZeroUsize::new(2).expect("two"),
        ]);
        assert_eq!(
            line(AllowHint::Unusable(problem)),
            "note [account-allow] the layers name an account, but allowed.yaml has entries 1, 2 that alz ignores (alz account which warns about them); fix them or delete the file, alz does not repair it; then run: alz account allow"
        );
        assert_eq!(
            line(AllowHint::Unusable(AllowFileProblem::NotAList)),
            "note [account-allow] the layers name an account, but allowed.yaml is not a list; fix it or delete it, alz does not repair it; then run: alz account allow"
        );
    }

    #[test]
    fn a_changed_declaration_shows_both_maps_and_sources_escaped() {
        let hint = AllowHint::Changed {
            was: (
                vec![(TargetName::Claude, "client-a".to_owned())],
                "/hub/a".to_owned(),
            ),
            now: (
                vec![(TargetName::Claude, "client-c".to_owned())],
                "/hub/\u{200B}b".to_owned(),
            ),
        };
        assert_eq!(
            line(hint),
            "note [account-allow] the account declaration changed since it was allowed: was claude=client-a from \"/hub/a\", now claude=client-c from \"/hub/\\u{200B}b\"; run: alz account allow"
        );
    }
}
