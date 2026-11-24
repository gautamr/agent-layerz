//! What the layers read from the cache declare (spec §7.13 "Who may declare it", "No fetch on
//! resolve"): the same rule as the build's, with one difference. A layer inside the repository
//! takes no part in the decision. It can neither win nor create a conflict; it is only reported
//! as ignored (decision-18).

use super::{Miss, MissReason};
use crate::account::AccountDeclaration;
use crate::layer::LayerName;
use crate::lock::ObjectId;
use crate::merge::{Placement, Reason, refusal_reasons};
use crate::stack::{GitRef, ResolvedLayer, ResolvedStack, Source};

/// A layer that was read, with the facts the file system gave: where it lies, where `allowed.yaml`
/// records it from, and the pin of the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedLayer {
    /// The position in the stack, counted from 0.
    pub position: usize,
    /// Inside or outside the repository.
    pub placement: Placement,
    /// The layer: for one inside the repository only `layer.yaml` was read, so it holds no files.
    pub layer: ResolvedLayer,
    /// The ref the manifest names, as written.
    pub git_ref: Option<GitRef>,
    /// The commit the lock pins; an unpinned `path:` layer has none.
    pub revision: Option<ObjectId>,
    /// The source as `allowed.yaml` records it: a local layer's real path, a remote URL without
    /// its ref, or a local repository's canonical path.
    pub recorded_source: String,
}

/// A declaration that is reported and not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ignored {
    /// The layer that declared.
    pub layer: LayerName,
    /// Its position in the stack, counted from 0.
    pub position: usize,
    /// Its source as the stack entry writes it.
    pub source: Source,
    /// Why it may not, never empty.
    pub reasons: Vec<Reason>,
}

/// The winning declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredAccount {
    /// The per-target map as declared.
    pub map: AccountDeclaration,
    /// The layer that declared it.
    pub layer: LayerName,
    /// Its position in the stack, counted from 0.
    pub position: usize,
    /// Its source as the stack entry writes it.
    pub source: Source,
    /// The ref the manifest names, as written.
    pub git_ref: Option<GitRef>,
    /// The lock's commit, the revision of §5.3.
    pub revision: Option<ObjectId>,
    /// The source in the form `allowed.yaml` records.
    pub recorded_source: String,
}

/// The result of reading a stack's account declaration. `Conflict` is a case of its own, so that a
/// caller that only looks for a declaration cannot read it as "no declaration".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedDeclaration {
    /// Something could not be verified: no account applies from the layers.
    Miss(Miss),
    /// The stack was read and nobody declares.
    None {
        /// Declarations that are reported and not applied.
        ignored: Vec<Ignored>,
    },
    /// The stack was read and one layer's declaration applies.
    Declared {
        /// The winner.
        declaration: Box<DeclaredAccount>,
        /// Declarations that are reported and not applied.
        ignored: Vec<Ignored>,
    },
    /// Several client layers declare: no declaration applies.
    Conflict {
        /// The client layers, in stack order.
        clients: Vec<LayerName>,
        /// Declarations that are reported and not applied.
        ignored: Vec<Ignored>,
    },
}

/// The declaration of layers in stack order. The layers outside the repository must make a valid
/// stack (§6 step 2: no name twice, visibility only grows), or the miss is
/// [`MissReason::StackInvalid`]; layers inside the repository are not validated, so editing
/// `.layer/` never blocks the account.
#[must_use]
pub fn decide(layers: &[CachedLayer]) -> CachedDeclaration {
    let outside = layers
        .iter()
        .filter(|layer| layer.placement == Placement::Outside)
        .map(|layer| layer.layer.clone())
        .collect();
    if ResolvedStack::validate(outside).is_err() {
        return CachedDeclaration::Miss(Miss::of(MissReason::StackInvalid));
    }
    let verdicts: Vec<_> = layers
        .iter()
        .filter_map(|layer| {
            let meta = layer.layer.meta();
            meta.account().map(|map| {
                let reasons = refusal_reasons(
                    layer.position == 0,
                    meta.kind(),
                    meta.visibility(),
                    layer.placement,
                );
                (layer, map, reasons)
            })
        })
        .collect();
    let ignored = verdicts
        .iter()
        .filter(|(_, _, reasons)| !reasons.is_empty())
        .map(|(layer, _, reasons)| Ignored {
            layer: layer.layer.meta().name().clone(),
            position: layer.position,
            source: layer.layer.source().clone(),
            reasons: reasons.clone(),
        })
        .collect();
    let (firsts, clients): (Vec<_>, Vec<_>) = verdicts
        .into_iter()
        .filter(|(_, _, reasons)| reasons.is_empty())
        .partition(|(layer, _, _)| layer.position == 0);
    if clients.len() > 1 {
        return CachedDeclaration::Conflict {
            clients: clients
                .iter()
                .map(|(layer, _, _)| layer.layer.meta().name().clone())
                .collect(),
            ignored,
        };
    }
    match clients.into_iter().chain(firsts).next() {
        None => CachedDeclaration::None { ignored },
        Some((layer, map, _)) => CachedDeclaration::Declared {
            declaration: Box::new(DeclaredAccount {
                map: map.clone(),
                layer: layer.layer.meta().name().clone(),
                position: layer.position,
                source: layer.layer.source().clone(),
                git_ref: layer.git_ref.clone(),
                revision: layer.revision,
                recorded_source: layer.recorded_source.clone(),
            }),
            ignored,
        },
    }
}
