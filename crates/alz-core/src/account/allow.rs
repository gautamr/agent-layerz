//! What `alz account allow` does with the declaration it read (spec §7.13 "Trust and `alz account
//! allow`", §10.3): the same declaration resolution applies, turned into the entry that would be
//! recorded, or into the reason nothing can be offered. Pure: the cache, the file and the terminal
//! are `alz-io`'s.

use crate::account::AccountFamily;
use crate::account::cached::{CachedDeclaration, DeclaredAccount, Miss, MissReason};
use crate::account::declaration::NonEmptyTargetMap;
use crate::account::resolution::{BLOCKED, ignored_warnings};
use crate::account::{AccountChoice, AccountName, AllowEntry, AllowedSource};
use crate::diagnostic::Diagnostics;
use crate::layer::LayerName;
use crate::lock::ObjectId;
use crate::path::AbsolutePath;
use crate::stack::{GitRef, Source};

/// The declaration that applies, as `allow` would record it and show it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Declared {
    entry: AllowEntry,
    revision: Option<ObjectId>,
    layer: LayerName,
    position: usize,
    source: Source,
    git_ref: Option<GitRef>,
}

/// What `allow` shows and asks before it records an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    declared: Declared,
    current: Option<AllowEntry>,
}

impl Offer {
    /// The entry that would be recorded.
    #[must_use]
    pub const fn entry(&self) -> &AllowEntry {
        &self.declared.entry
    }

    /// The entry this root has now, when it differs from the one offered.
    #[must_use]
    pub const fn current(&self) -> Option<&AllowEntry> {
        self.current.as_ref()
    }

    /// The declaring layer's name.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        &self.declared.layer
    }

    /// The declaring layer's position in the stack, counted from 0.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.declared.position
    }

    /// The declaring layer's source as the stack entry writes it.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.declared.source
    }

    /// The ref the manifest names, as written.
    #[must_use]
    pub const fn git_ref(&self) -> Option<&GitRef> {
        self.declared.git_ref.as_ref()
    }

    /// The commit of `layer.lock`; none for an unpinned `path:` layer.
    #[must_use]
    pub const fn revision(&self) -> Option<&ObjectId> {
        self.declared.revision.as_ref()
    }

    /// One name for each distinct account, in target order: one confirmation is asked for each.
    #[must_use]
    pub fn names(&self) -> Vec<AccountName> {
        distinct_names(self.declared.entry.account())
    }

    /// Whether `declaration`, read again after the confirmation, still holds what was shown: the
    /// map, the recorded source, the revision and the layer. What this root's entry says now does
    /// not matter, because the writer decides that under its lock.
    #[must_use]
    pub fn is_still_declared_by(
        &self,
        root: &AbsolutePath,
        declaration: &CachedDeclaration,
    ) -> bool {
        match declaration {
            CachedDeclaration::Declared { declaration, .. } => {
                matches!(declared(root, declaration), Ok(Offered::Offer(again)) if *again == self.declared)
            }
            CachedDeclaration::Miss(_)
            | CachedDeclaration::None { .. }
            | CachedDeclaration::Conflict { .. } => false,
        }
    }
}

/// The distinct names of a map, in target order.
#[must_use]
pub fn distinct_names(account: &NonEmptyTargetMap<AccountName>) -> Vec<AccountName> {
    account.iter().fold(
        Vec::new(),
        |mut names, (_, name): (AccountFamily, &AccountName)| {
            if !names.contains(name) {
                names.push(name.clone());
            }
            names
        },
    )
}

/// What `alz account allow` can do with what the layers declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowDecision {
    /// This root's entry equals the declaration: nothing to ask or write.
    AlreadyAllowed,
    /// No layer names an account here; `--remove` is named when this root has an entry.
    NothingDeclared {
        /// Whether this root has an entry in `allowed.yaml`.
        has_entry: bool,
    },
    /// Something could not be verified: run `alz build`.
    CacheMiss(Miss),
    /// Several client layers declare.
    Conflict(Vec<LayerName>),
    /// A layer names the reserved `alz-blocked`.
    Reserved,
    /// Show the offer and ask.
    Ask(Box<Offer>),
}

/// The decision and the warnings about declarations that are reported and not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowance {
    /// What to do.
    pub decision: AllowDecision,
    /// `account-declaration-ignored`, as `alz account which` prints them.
    pub warnings: Diagnostics,
}

/// What a declaration is, short of the state of the file.
enum Offered {
    Offer(Box<Declared>),
    Reserved,
}

/// Why a declaration cannot become an entry; both are impossible for a declaration the reader
/// produced and are a miss rather than a panic.
struct Unusable;

fn declared(root: &AbsolutePath, declaration: &DeclaredAccount) -> Result<Offered, Unusable> {
    let choices: Vec<(AccountFamily, AccountChoice)> = AccountFamily::ALL
        .into_iter()
        .filter_map(|target| {
            declaration
                .map
                .choice_for(target)
                .map(|choice| (target, choice.clone()))
        })
        .collect();
    if choices
        .iter()
        .any(|(_, choice)| matches!(choice, AccountChoice::Named(name) if name.as_str() == BLOCKED))
    {
        return Ok(Offered::Reserved);
    }
    let account = NonEmptyTargetMap::from_entries(
        choices
            .into_iter()
            .map(|(target, choice)| (target, choice.to_name())),
    )
    .map_err(|_empty| Unusable)?;
    let source =
        AllowedSource::try_from(declaration.recorded_source.as_str()).map_err(|_empty| Unusable)?;
    Ok(Offered::Offer(Box::new(Declared {
        entry: AllowEntry::new(root.clone(), account, source),
        revision: declaration.revision,
        layer: declaration.layer.clone(),
        position: declaration.position,
        source: declaration.source.clone(),
        git_ref: declaration.git_ref.clone(),
    })))
}

/// What `allow` does for `root` with `declaration`, as read from the cache, and `current`, the
/// entry this root has in `allowed.yaml`.
#[must_use]
pub fn allowance(
    root: &AbsolutePath,
    declaration: &CachedDeclaration,
    current: Option<&AllowEntry>,
) -> Allowance {
    let (decision, ignored) = match declaration {
        CachedDeclaration::Miss(miss) => (AllowDecision::CacheMiss(*miss), &[][..]),
        CachedDeclaration::None { ignored } => (
            AllowDecision::NothingDeclared {
                has_entry: current.is_some(),
            },
            ignored.as_slice(),
        ),
        CachedDeclaration::Conflict { clients, ignored } => {
            (AllowDecision::Conflict(clients.clone()), ignored.as_slice())
        }
        CachedDeclaration::Declared {
            declaration,
            ignored,
        } => (decide(root, declaration, current), ignored.as_slice()),
    };
    Allowance {
        decision,
        warnings: ignored_warnings(ignored)
            .into_iter()
            .fold(Diagnostics::new(), Diagnostics::with),
    }
}

fn decide(
    root: &AbsolutePath,
    declaration: &DeclaredAccount,
    current: Option<&AllowEntry>,
) -> AllowDecision {
    match declared(root, declaration) {
        Err(Unusable) => AllowDecision::CacheMiss(Miss::of(MissReason::SourceInvalid)),
        Ok(Offered::Reserved) => AllowDecision::Reserved,
        Ok(Offered::Offer(declared)) if current == Some(&declared.entry) => {
            AllowDecision::AlreadyAllowed
        }
        Ok(Offered::Offer(declared)) => AllowDecision::Ask(Box::new(Offer {
            declared: *declared,
            current: current.cloned(),
        })),
    }
}

#[cfg(test)]
mod tests;
