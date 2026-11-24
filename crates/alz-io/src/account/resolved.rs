//! The two files' content checked again once the paths in it are real (spec §7.13 "User config",
//! "Allow state"): a folder key or a root that names an account folder, two that resolve alike,
//! a root that is not its own real path. Account folders are never opened; the real path of a
//! name is found by looking at names only.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use alz_core::account::{
    AccountDeclaration, AccountFolders, AccountName, AllowEntry, AllowFile, FolderKey, FolderRule,
    Reading, Relation, UserConfig,
};
use alz_core::path::AbsolutePath;
use alz_core::{Code, Diagnostic, Message, QuotedKey, Where};

use crate::resolve::folders::real_path;

/// A `folders` entry of `accounts.yaml` whose folder exists: the real path, and the account it
/// stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFolder {
    real: AbsolutePath,
    choice: AccountDeclaration,
}

impl ResolvedFolder {
    /// The folder's real path.
    #[must_use]
    pub const fn real(&self) -> &AbsolutePath {
        &self.real
    }

    /// The account the folder and everything below it stands for.
    #[must_use]
    pub const fn choice(&self) -> &AccountDeclaration {
        &self.choice
    }
}

/// `accounts.yaml` after its paths are real. A key that does not resolve matches nothing and is
/// not here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedUserConfig {
    default_name: Option<AccountName>,
    folders: Vec<ResolvedFolder>,
}

impl ResolvedUserConfig {
    /// What the default account is called on this machine, when the file says so.
    #[must_use]
    pub const fn default_name(&self) -> Option<&AccountName> {
        self.default_name.as_ref()
    }

    /// The folders that apply, in the order of the file.
    #[must_use]
    pub fn folders(&self) -> &[ResolvedFolder] {
        &self.folders
    }

    pub(super) fn resolve(
        reading: Reading<UserConfig>,
        home: &AbsolutePath,
        folders: &AccountFolders,
        file: &Where,
    ) -> Reading<Self> {
        let (config, warnings) = reading.into_parts();
        let fates: Vec<Fate> = config
            .folders()
            .iter()
            .map(|rule| fate(rule, home, folders, file))
            .collect();
        let kept: Vec<ResolvedFolder> = fates
            .iter()
            .filter_map(|fate| match fate {
                Fate::Kept(folder) => Some(folder.clone()),
                Fate::Dropped | Fate::Refused(_) => None,
            })
            .collect();
        let clashing = clashing(&kept);
        let clash_warning = (!clashing.is_empty()).then(|| {
            Diagnostic::warning(
                Code::AccountConfig,
                Some(file.clone()),
                Message::new(
                    "two folders keys resolve to the same directory with different accounts; \
                     both are ignored",
                ),
            )
        });
        let refusals = fates.into_iter().filter_map(|fate| match fate {
            Fate::Refused(warning) => Some(warning),
            Fate::Kept(_) | Fate::Dropped => None,
        });
        let resolved = Self {
            default_name: config.default_name().cloned(),
            folders: kept
                .into_iter()
                .filter(|folder| !clashing.contains(&folder.real))
                .collect(),
        };
        rebuild(resolved, &warnings, refusals.chain(clash_warning))
    }
}

/// What one `folders` rule became.
enum Fate {
    Kept(ResolvedFolder),
    /// The folder does not resolve: it matches nothing, silently.
    Dropped,
    Refused(Diagnostic),
}

fn fate(rule: &FolderRule, home: &AbsolutePath, folders: &AccountFolders, file: &Where) -> Fate {
    let refuse = || {
        Fate::Refused(Diagnostic::warning(
            Code::AccountConfig,
            Some(file.clone()),
            Message::new("the folders key ")
                .then(QuotedKey::from_untrusted(&key_text(rule.key())))
                .then(" lies inside an account folder, or is $HOME or above it, so it is ignored"),
        ))
    };
    let Some(path) = expand(rule.key(), home) else {
        return Fate::Dropped;
    };
    if refused_for_a_folder(folders.relation(&path)) {
        return refuse();
    }
    match real_path(&path) {
        None => Fate::Dropped,
        Some(real) if refused_for_a_folder(folders.relation(&real)) => refuse(),
        Some(real) => Fate::Kept(ResolvedFolder {
            real,
            choice: rule.choice().clone(),
        }),
    }
}

/// A key is refused when it lies inside an account folder or is `$HOME` or above it; one that
/// only holds a configured folder is not.
const fn refused_for_a_folder(relation: Option<Relation>) -> bool {
    matches!(
        relation,
        Some(Relation::AccountFolder | Relation::Configured | Relation::Home)
    )
}

fn key_text(key: &FolderKey) -> String {
    match key {
        FolderKey::Absolute(path) => path.to_string(),
        FolderKey::Home(rest) => format!("~/{}", rest.as_str()),
    }
}

fn expand(key: &FolderKey, home: &AbsolutePath) -> Option<AbsolutePath> {
    match key {
        FolderKey::Absolute(path) => Some(path.clone()),
        FolderKey::Home(rest) => AbsolutePath::try_from(
            format!("{}/{}", home.as_str().trim_end_matches('/'), rest.as_str()).as_str(),
        )
        .ok(),
    }
}

/// The real paths that two or more entries share with different accounts.
fn clashing(kept: &[ResolvedFolder]) -> Vec<AbsolutePath> {
    let by_path = kept.iter().fold(
        BTreeMap::<&AbsolutePath, Vec<&AccountDeclaration>>::new(),
        |mut map, folder| {
            map.entry(&folder.real).or_default().push(&folder.choice);
            map
        },
    );
    by_path
        .into_iter()
        .filter(|(_, choices)| {
            choices
                .first()
                .is_some_and(|first| choices.iter().any(|other| other != first))
        })
        .map(|(real, _)| real.clone())
        .collect()
}

/// `value` with `warnings` and then `more` as its warnings.
fn rebuild<T>(
    value: T,
    warnings: &alz_core::Diagnostics,
    more: impl IntoIterator<Item = Diagnostic>,
) -> Reading<T> {
    warnings
        .iter()
        .cloned()
        .chain(more)
        .fold(Reading::new(value), Reading::warned)
}

/// `allowed.yaml` after its roots are checked again. Every entry here has a root that is its own
/// real path and is not an account folder or `$HOME` or above, and no other entry has it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedAllowState {
    entries: Vec<AllowEntry>,
    unresolved: Vec<AllowEntry>,
    ignored: Vec<NonZeroUsize>,
    list: bool,
}

impl ResolvedAllowState {
    /// Whether the file's top level is a list. When it is not (or the file is not YAML) the whole
    /// file is ignored and this holds nothing.
    #[must_use]
    pub const fn is_list(&self) -> bool {
        self.list
    }

    /// The approvals that apply, sorted by root.
    #[must_use]
    pub fn entries(&self) -> &[AllowEntry] {
        &self.entries
    }

    /// The approvals whose root does not resolve now: they match nothing, and `alz account allow`
    /// keeps them.
    #[must_use]
    pub fn unresolved(&self) -> &[AllowEntry] {
        &self.unresolved
    }

    /// The number, counted from 1, of each entry the file holds that is ignored, in file order.
    /// `alz account allow` names them and refuses to write while there are any.
    #[must_use]
    pub fn ignored(&self) -> &[NonZeroUsize] {
        &self.ignored
    }

    pub(super) fn resolve(
        reading: Reading<AllowFile>,
        folders: &AccountFolders,
        file: &Where,
    ) -> Reading<Self> {
        let (read, warnings) = reading.into_parts();
        match read {
            AllowFile::Ignored => rebuild(Self::default(), &warnings, []),
            AllowFile::List {
                state,
                ignored,
                numbers,
            } => {
                let fates: Vec<EntryFate> = state
                    .entries()
                    .iter()
                    .cloned()
                    .zip(numbers)
                    .map(|(entry, number)| entry_fate(entry, number, folders))
                    .collect();
                let roots = fates.iter().fold(
                    BTreeMap::<AbsolutePath, Vec<NonZeroUsize>>::new(),
                    |mut map, fate| {
                        if let EntryFate::Kept(entry, number) = fate {
                            map.entry(entry.root().clone()).or_default().push(*number);
                        }
                        map
                    },
                );
                let duplicated =
                    |root: &AbsolutePath| roots.get(root).is_some_and(|numbers| numbers.len() > 1);
                let refusals = fates.iter().filter_map(|fate| match fate {
                    EntryFate::Refused(number, reason) => {
                        Some(entry_warning(file, *number, reason))
                    }
                    EntryFate::Kept(..) | EntryFate::Unresolved(_) => None,
                });
                let duplicate_warnings = roots
                    .values()
                    .filter(|numbers| numbers.len() > 1)
                    .map(|numbers| duplicate_warning(file, numbers));
                let extra: Vec<Diagnostic> = refusals.chain(duplicate_warnings).collect();
                let mut all_ignored: Vec<NonZeroUsize> = ignored
                    .into_iter()
                    .chain(fates.iter().filter_map(|fate| match fate {
                        EntryFate::Refused(number, _) => Some(*number),
                        EntryFate::Kept(entry, number) if duplicated(entry.root()) => Some(*number),
                        EntryFate::Kept(..) | EntryFate::Unresolved(_) => None,
                    }))
                    .collect();
                all_ignored.sort();
                let resolved = Self {
                    entries: fates
                        .iter()
                        .filter_map(|fate| match fate {
                            EntryFate::Kept(entry, _) if !duplicated(entry.root()) => {
                                Some(entry.clone())
                            }
                            EntryFate::Kept(..)
                            | EntryFate::Refused(..)
                            | EntryFate::Unresolved(_) => None,
                        })
                        .collect(),
                    unresolved: fates
                        .iter()
                        .filter_map(|fate| match fate {
                            EntryFate::Unresolved(entry) => Some(entry.clone()),
                            EntryFate::Kept(..) | EntryFate::Refused(..) => None,
                        })
                        .collect(),
                    ignored: all_ignored,
                    list: true,
                };
                rebuild(resolved, &warnings, extra)
            }
        }
    }
}

/// What one entry of `allowed.yaml` became.
enum EntryFate {
    Kept(AllowEntry, NonZeroUsize),
    /// The root does not resolve: kept, and matches nothing.
    Unresolved(AllowEntry),
    Refused(NonZeroUsize, &'static str),
}

const NAMES_AN_ACCOUNT_FOLDER: &str =
    "its root lies inside an account folder, or is $HOME or above it";

fn entry_fate(entry: AllowEntry, number: NonZeroUsize, folders: &AccountFolders) -> EntryFate {
    let refused = |relation: Option<Relation>| {
        matches!(relation, Some(Relation::AccountFolder | Relation::Home))
    };
    if refused(folders.relation(entry.root())) {
        return EntryFate::Refused(number, NAMES_AN_ACCOUNT_FOLDER);
    }
    match real_path(entry.root()) {
        None => EntryFate::Unresolved(entry),
        Some(real) if &real != entry.root() => EntryFate::Refused(
            number,
            "its root is not a real path (alz account allow records the real path)",
        ),
        Some(real) if refused(folders.relation(&real)) => {
            EntryFate::Refused(number, NAMES_AN_ACCOUNT_FOLDER)
        }
        Some(_) => EntryFate::Kept(entry, number),
    }
}

/// A warning that holds an entry's position only.
fn entry_warning(file: &Where, number: NonZeroUsize, reason: &'static str) -> Diagnostic {
    Diagnostic::warning(
        Code::AccountAllowed,
        Some(file.clone()),
        Message::new("entry ")
            .then(number.get())
            .then(" is ignored (")
            .then(reason)
            .then(")"),
    )
}

fn duplicate_warning(file: &Where, numbers: &[NonZeroUsize]) -> Diagnostic {
    let listed =
        numbers
            .iter()
            .enumerate()
            .fold(Message::new("entries "), |message, (index, number)| {
                let separated = if index == 0 {
                    message
                } else {
                    message.then(", ")
                };
                separated.then(number.get())
            });
    Diagnostic::warning(
        Code::AccountAllowed,
        Some(file.clone()),
        listed.then(" are ignored (they have the same root)"),
    )
}
