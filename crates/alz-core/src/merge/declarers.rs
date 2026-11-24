//! Who may declare `commit_identity` and `account` (spec §7.12, §7.13, decision-13): one rule
//! for both fields.
//!
//! Only two layers may declare: the first layer of the stack, or one later layer of `kind:
//! client` that is not public, and both outside the repository. The client's whole value wins; the
//! two values are never mixed. Several client declarations, or a declaration of the first layer
//! that is refused, leave no winner: the result is [`Declared::Conflict`], so nothing picks between
//! layers that disagree about who may declare. Every refused declaration, and the several clients,
//! are also reported as build errors.
//!
//! The rule is pure. Each layer's position, kind, visibility and [`Placement`] are the facts it
//! looks at; the file system told `alz-io` where each layer lies.
//!
//! The spec says the same; the consumers (the git identity setup, `alz account which`
//! and `alz account allow`) are R6's and read the winner from [`Merged`](super::Merged).

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::layer::{Kind, LayerMeta, LayerName, Visibility};
use crate::merge::placement::Placement;
use crate::merge::provenance::Origin;
use crate::merge::stack::LayerContext;
use crate::untrusted_text::Stripped;

/// The outcome of the declarations of one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declared<T> {
    /// No layer that may declare did, and the first layer did not either.
    None,
    /// One layer's value applies.
    Winner {
        /// The layer that declared it, and its `layer.yaml`.
        origin: Origin,
        /// Its whole value.
        value: T,
    },
    /// Several client layers declare, or the first layer declared and may not: no value applies.
    Conflict,
}

impl<T> Declared<T> {
    /// The winning layer and its value, `None` for no declaration and for a conflict.
    #[must_use]
    pub const fn winner(&self) -> Option<(&LayerName, &T)> {
        match self {
            Self::Winner { origin, value } => Some((origin.layer(), value)),
            Self::None | Self::Conflict => None,
        }
    }

    /// Where the winning value is declared: its layer, the layer's revision and `layer.yaml`.
    #[must_use]
    pub const fn origin(&self) -> Option<&Origin> {
        match self {
            Self::Winner { origin, .. } => Some(origin),
            Self::None | Self::Conflict => None,
        }
    }

    /// Whether the declarations conflict.
    #[must_use]
    pub const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// The two fields that follow the rule; each names itself and its diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeclaredField {
    CommitIdentity,
    Account,
}

impl DeclaredField {
    const fn name(self) -> &'static str {
        match self {
            Self::CommitIdentity => "commit_identity",
            Self::Account => "account",
        }
    }

    const fn code(self) -> Code {
        match self {
            Self::CommitIdentity => Code::Identity,
            Self::Account => Code::Account,
        }
    }
}

/// Why a layer may not declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The layer lies inside the repository, whatever its kind says.
    InsideRepository,
    /// A layer after the first whose kind is not `client`.
    KindNotClient(Kind),
    /// A layer after the first that is public.
    Public,
}

/// A layer that declared and may not, with every reason, in the order of [`Reason`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    layer: LayerName,
    reasons: Vec<Reason>,
}

impl Refused {
    /// The layer.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        &self.layer
    }

    /// Why it may not declare; never empty.
    #[must_use]
    pub fn reasons(&self) -> &[Reason] {
        &self.reasons
    }
}

/// What the rule looks at for a layer that declares a value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate<T> {
    layer: LayerName,
    origin: Origin,
    is_first: bool,
    kind: Kind,
    visibility: Visibility,
    placement: Placement,
    value: T,
}

impl<T> Candidate<T> {
    /// Why the layer may not declare; empty when it may (§7.12).
    fn problems(&self) -> Vec<Reason> {
        refusal_reasons(self.is_first, &self.kind, self.visibility, self.placement)
    }
}

/// Why a layer with this position, kind, visibility and placement may not declare `commit_identity`
/// or `account`, in the order of [`Reason`]; empty when it may (§7.12). The first layer needs only
/// to be outside the repository, a later one must also be a `client` that is not public.
#[must_use]
pub fn refusal_reasons(
    is_first: bool,
    kind: &Kind,
    visibility: Visibility,
    placement: Placement,
) -> Vec<Reason> {
    [
        (placement == Placement::Inside).then_some(Reason::InsideRepository),
        (!is_first && !kind.is_client()).then(|| Reason::KindNotClient(kind.clone())),
        (!is_first && visibility < Visibility::Private).then_some(Reason::Public),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The layers that declare a field, as the fold meets them, most general first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarerState<T>(Vec<Candidate<T>>);

impl<T> Default for DeclarerState<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: Clone> DeclarerState<T> {
    /// The state after one more, more specific, layer: a layer that does not declare changes
    /// nothing.
    pub(super) fn absorb(
        self,
        context: &LayerContext<'_>,
        pick: impl FnOnce(&LayerMeta) -> Option<&T>,
    ) -> Self {
        let meta = context.layer().meta();
        Self(
            self.0
                .into_iter()
                .chain(pick(meta).map(|value| Candidate {
                    layer: meta.name().clone(),
                    origin: Origin::layer_yaml(context.layer()),
                    is_first: context.is_first(),
                    kind: meta.kind().clone(),
                    visibility: meta.visibility(),
                    placement: context.placement(),
                    value: value.clone(),
                }))
                .collect(),
        )
    }

    /// The outcome, and the build errors for `field`.
    pub(super) fn finish(self, field: DeclaredField) -> (Declarations<T>, Diagnostics) {
        let declarations = Declarations::evaluate(self.0);
        let found = declarations.report(field);
        (declarations, found)
    }
}

/// The declarations of one field: the outcome, the layers that declared and may not, and the
/// client layers that declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declarations<T> {
    declared: Declared<T>,
    refused: Vec<Refused>,
    clients: Vec<LayerName>,
}

impl<T> Declarations<T> {
    /// The outcome: the winner, no declaration, or a conflict.
    #[must_use]
    pub const fn declared(&self) -> &Declared<T> {
        &self.declared
    }

    /// The layers that declared and may not, in stack order, each with its reasons. The first
    /// layer is among them when it lies inside the repository; whether that blocks the field in
    /// resolution (§7.13) is the consumer's rule, and [`declared`](Self::declared) is a
    /// [`Declared::Conflict`] in that case.
    #[must_use]
    pub fn refused(&self) -> &[Refused] {
        &self.refused
    }

    /// The client layers that declared and may, in stack order. More than one is an error.
    #[must_use]
    pub fn clients(&self) -> &[LayerName] {
        &self.clients
    }

    /// Applies the rule to the layers that declared, in stack order.
    fn evaluate(candidates: Vec<Candidate<T>>) -> Self {
        let (eligible, refused): (Vec<_>, Vec<_>) = candidates
            .into_iter()
            .map(|candidate| (candidate.problems(), candidate))
            .partition(|(problems, _)| problems.is_empty());
        let first_refused = refused.iter().any(|(_, candidate)| candidate.is_first);
        let (clients, firsts): (Vec<_>, Vec<_>) = eligible
            .into_iter()
            .map(|(_, candidate)| candidate)
            .partition(|candidate| !candidate.is_first);
        let client_layers = clients
            .iter()
            .map(|candidate| candidate.layer.clone())
            .collect();
        let declared = if first_refused || clients.len() > 1 {
            Declared::Conflict
        } else {
            clients
                .into_iter()
                .chain(firsts)
                .next()
                .map_or(Declared::None, |winner| Declared::Winner {
                    origin: winner.origin,
                    value: winner.value,
                })
        };
        Self {
            declared,
            refused: refused
                .into_iter()
                .map(|(reasons, candidate)| Refused {
                    layer: candidate.layer,
                    reasons,
                })
                .collect(),
            clients: client_layers,
        }
    }

    /// One build error per refused layer, and one naming every client when there are several.
    fn report(&self, field: DeclaredField) -> Diagnostics {
        let refusals = self
            .refused
            .iter()
            .map(|refused| refusal_error(field, refused));
        let several = match self.clients.as_slice() {
            [_, second, ..] => Some(several_clients_error(field, &self.clients, second)),
            [] | [_] => None,
        };
        refusals.chain(several).collect()
    }
}

fn layer_yaml(layer: &LayerName) -> Where {
    Where::layer_file(
        Stripped::from_untrusted(layer.as_str()),
        Stripped::from_untrusted("layer.yaml"),
    )
}

/// `message` with the words for `reason` added.
fn with_reason(message: Message, reason: &Reason) -> Message {
    match reason {
        Reason::InsideRepository => message.then("it is inside the repository"),
        Reason::KindNotClient(kind) => message
            .then("its kind is '")
            .then(Stripped::from_untrusted(kind.as_str()))
            .then("', not 'client'"),
        Reason::Public => message.then("it is public"),
    }
}

/// `message` with `layer` in single quotes added.
fn with_quoted(message: Message, layer: &LayerName) -> Message {
    message
        .then("'")
        .then(MessagePart::printable(layer))
        .then("'")
}

fn refusal_error(field: DeclaredField, refused: &Refused) -> Diagnostic {
    let head = Message::new("layer ")
        .then("'")
        .then(MessagePart::printable(&refused.layer))
        .then("' cannot declare ")
        .then(field.name())
        .then(": ");
    let message = refused
        .reasons
        .iter()
        .enumerate()
        .fold(head, |message, (index, reason)| {
            with_reason(
                if index == 0 { message } else { message.then("; ") },
                reason,
            )
        })
        .then(". Only the first layer, or one private layer with kind: client outside the repository, may");
    Diagnostic::error(field.code(), Some(layer_yaml(&refused.layer)), message)
}

/// The error for several clients: they are listed as `'a', 'b' and 'c' all declare`, and it is
/// placed at the second.
fn several_clients_error(
    field: DeclaredField,
    clients: &[LayerName],
    second: &LayerName,
) -> Diagnostic {
    let (before, last) = clients.split_at(clients.len().saturating_sub(1));
    let listed =
        before
            .iter()
            .enumerate()
            .fold(Message::new("layers "), |message, (index, layer)| {
                with_quoted(
                    if index == 0 {
                        message
                    } else {
                        message.then(", ")
                    },
                    layer,
                )
            });
    let with_last = last.iter().fold(listed.then(" and "), |message, layer| {
        with_quoted(message, layer)
    });
    let message = with_last
        .then(if clients.len() == 2 { " both" } else { " all" })
        .then(" declare ")
        .then(field.name())
        .then("; only one client layer may");
    Diagnostic::error(field.code(), Some(layer_yaml(second)), message)
}

#[cfg(test)]
mod tests;
