//! The merge of permissions (spec §7.6).
//!
//! `allow`, `ask` and `deny` are lists of rules, combined across layers in stack order, a rule once
//! ([`OrderedSet`]). A layer may edit a list with `-[..]` and `[..]` as well as add to it, but
//! never the `deny` list of a more general layer: a deny rule that an edit would drop is a
//! `permission-floor` error that names the layer, the rule and the layer that set it, and the rule
//! stays. So the `deny` list only ever grows, which is the "security only gets stricter" law for
//! rules.
//!
//! `default_mode` is the most specific one set, but a layer's mode must be at least as strict as
//! the strictest mode that a more general layer declared with `floor: true` ([`Floor`]). A mode
//! that is looser is a `permission-floor` error, and the layer's mode and floor are then ignored.
//! A floor binds the layers after the one that declares it, never that layer's own mode.
//!
//! The flags `disable_bypass_mode`, `block_reads_outside` and `block_writes_outside`
//! ([`FloorFlag`]) only move to `true`: once a layer sets one, it stays set. A more specific layer
//! that writes `false` for it is a `permission-floor` error naming the layer, the field and the
//! layer that set it, and the flag stays `true`.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{
    Binding, DefaultMode, FloorFlag, PermissionItems, PermissionMode, PermissionRule, Permissions,
    place,
};
use crate::layer::LayerName;
use crate::merge::action::Action;
use crate::merge::algebra::{Floor, ListEdit, OrderedSet};
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::untrusted_text::Stripped;

/// The merged permissions: what the renderer writes to the targets' permission settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergedPermissions {
    allow: Vec<Traced<PermissionRule>>,
    ask: Vec<Traced<PermissionRule>>,
    deny: Vec<Traced<PermissionRule>>,
    default_mode: Option<Traced<PermissionMode>>,
    floor: Option<PermissionMode>,
    flags: BTreeMap<FloorFlag, Traced<bool>>,
    declarers: Declarers,
}

/// The layers that declared each part of the security floor, in stack order, each once. The
/// provenance of a rule or a flag names only the layer that set it first; the model of an adapter
/// names every layer that set it (spec §8.0 **Model.**).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Declarers {
    deny: BTreeMap<PermissionRule, Vec<LayerName>>,
    flags: BTreeMap<FloorFlag, Vec<LayerName>>,
    floor: Vec<LayerName>,
}

/// `layers` and `layer`, which comes last, once.
fn with_layer(layers: &[LayerName], layer: &LayerName) -> Vec<LayerName> {
    layers
        .iter()
        .filter(|known| *known != layer)
        .chain([layer])
        .cloned()
        .collect()
}

/// The rules a layer's edit of a list names itself: the whole list of a replacement, the added
/// rules of an adjustment.
fn declared(edit: &ListEdit<PermissionRule>) -> impl Iterator<Item = &PermissionRule> + '_ {
    match edit {
        ListEdit::Replace(rules) => rules.iter(),
        ListEdit::Adjust { add, .. } => add.iter(),
    }
}

impl MergedPermissions {
    /// Every layer that declares the deny rule `rule` in its `permissions.yaml`, in stack order,
    /// each once. Empty for a rule that no layer declares.
    #[must_use]
    pub fn deny_declared_by(&self, rule: &PermissionRule) -> &[LayerName] {
        self.declarers.deny.get(rule).map_or(&[], Vec::as_slice)
    }

    /// Every layer that sets `flag` to `true`, in stack order, each once.
    #[must_use]
    pub fn flag_set_by(&self, flag: FloorFlag) -> &[LayerName] {
        self.declarers.flags.get(&flag).map_or(&[], Vec::as_slice)
    }

    /// Every layer whose `default_mode` with `floor: true` is in force, in stack order, each once:
    /// a floor that was refused for being looser than an earlier one is not in force.
    #[must_use]
    pub fn floor_set_by(&self) -> &[LayerName] {
        &self.declarers.floor
    }

    /// The layers that set `flag` to `true`, the first as the one that defined it. `None` when no
    /// layer did: the flag is not set, and a flag that is set is never `false`.
    #[must_use]
    pub fn flag(&self, flag: FloorFlag) -> Option<&Traced<bool>> {
        self.flags.get(&flag)
    }

    /// Whether some layer set `flag`.
    #[must_use]
    pub fn is_set(&self, flag: FloorFlag) -> bool {
        self.flags.contains_key(&flag)
    }

    /// The rules that are allowed, in stack order then file order.
    #[must_use]
    pub fn allow(&self) -> &[Traced<PermissionRule>] {
        &self.allow
    }

    /// The rules that ask first.
    #[must_use]
    pub fn ask(&self) -> &[Traced<PermissionRule>] {
        &self.ask
    }

    /// The rules that are denied: every deny rule of every layer, and no layer removed one.
    #[must_use]
    pub fn deny(&self) -> &[Traced<PermissionRule>] {
        &self.deny
    }

    /// The `default_mode` of the most specific layer that set one.
    #[must_use]
    pub const fn default_mode(&self) -> Option<&Traced<PermissionMode>> {
        self.default_mode.as_ref()
    }

    /// The strictest mode a layer declared with `floor: true`: the merged `default_mode` is never
    /// looser.
    #[must_use]
    pub const fn floor(&self) -> Option<PermissionMode> {
        self.floor
    }
}

/// The rules of one list, in order, each with the layer that put it there.
#[derive(Debug, Clone, Default)]
struct RuleList(Vec<Traced<PermissionRule>>);

impl RuleList {
    fn members(&self) -> OrderedSet<PermissionRule> {
        self.0.iter().map(|rule| rule.value().clone()).collect()
    }

    /// The list that holds exactly `members`, in their order: a rule that was there keeps its
    /// provenance and a new one is defined by `layer`.
    fn holding(&self, members: &OrderedSet<PermissionRule>, origin: &Origin) -> Self {
        Self(
            members
                .iter()
                .map(|rule| {
                    self.0
                        .iter()
                        .find(|held| held.value() == rule)
                        .cloned()
                        .unwrap_or_else(|| {
                            Traced::new(rule.clone(), origin.clone(), Action::Define)
                        })
                })
                .collect(),
        )
    }

    /// The list after `edit`, which may drop any rule.
    fn edited(&self, edit: &ListEdit<PermissionRule>, origin: &Origin) -> Self {
        self.holding(&edit.apply(&self.members()), origin)
    }

    /// The list after `edit` for a list that must not shrink: the rules the edit would drop are
    /// kept, and returned too, so the caller can report them.
    fn edited_never_shrinking(
        &self,
        edit: &ListEdit<PermissionRule>,
        origin: &Origin,
    ) -> (Self, Vec<Traced<PermissionRule>>) {
        let members = self.members();
        let next = edit.apply(&members);
        let dropped = self
            .0
            .iter()
            .filter(|held| !next.contains(held.value()))
            .cloned()
            .collect();
        (self.holding(&members.merge(next), origin), dropped)
    }
}

/// The strictest floor declared so far, and the layer that declared it.
#[derive(Debug, Clone, Default)]
struct Held {
    floor: Floor<PermissionMode>,
    by: Option<LayerName>,
}

impl Held {
    /// The floor after `layer` declared `mode` as one: only a stricter mode raises it, so the
    /// first layer to declare the strictest mode stays the one named.
    fn raised(self, mode: PermissionMode, layer: &LayerName) -> Self {
        if self.floor.get() < Some(&mode) {
            Self {
                floor: self.floor.merge(Floor::at(mode)),
                by: Some(layer.clone()),
            }
        } else {
            self
        }
    }
}

/// The floor of the layers before the one being merged: the strictest mode a layer declared with
/// `floor: true`, and the layer that did. Agent overlays may not be looser than it (§7.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredFloor {
    mode: PermissionMode,
    by: LayerName,
}

impl DeclaredFloor {
    /// The strictest mode declared.
    pub(super) const fn mode(&self) -> PermissionMode {
        self.mode
    }

    /// The layer that first declared it.
    pub(super) const fn by(&self) -> &LayerName {
        &self.by
    }
}

/// The state of the fold for permissions.
#[derive(Debug, Clone, Default)]
pub(super) struct PermissionState {
    allow: RuleList,
    ask: RuleList,
    deny: RuleList,
    mode: Option<Traced<PermissionMode>>,
    held: Held,
    flags: BTreeMap<FloorFlag, Traced<bool>>,
    declarers: Declarers,
}

impl PermissionState {
    /// The declarers after `layer` wrote `file`: it declares the deny rules its edit names, sets
    /// each flag it writes as `true`, and, when `floored`, has its floor in force.
    fn declarers_after(&self, file: &Permissions, layer: &LayerName, floored: bool) -> Declarers {
        let deny = declared(file.deny()).fold(self.declarers.deny.clone(), |mut known, rule| {
            let layers = with_layer(known.get(rule).map_or(&[], Vec::as_slice), layer);
            known.insert(rule.clone(), layers);
            known
        });
        let flags = FloorFlag::ALL
            .into_iter()
            .filter(|flag| file.flag(*flag) == Some(true))
            .fold(self.declarers.flags.clone(), |mut known, flag| {
                let layers = with_layer(known.get(&flag).map_or(&[], Vec::as_slice), layer);
                known.insert(flag, layers);
                known
            });
        Declarers {
            deny,
            flags,
            floor: if floored {
                with_layer(&self.declarers.floor, layer)
            } else {
                self.declarers.floor.clone()
            },
        }
    }

    /// The floor declared so far, with the layer that declared it.
    pub(super) fn declared_floor(&self) -> Option<DeclaredFloor> {
        self.held
            .floor
            .get()
            .copied()
            .zip(self.held.by.clone())
            .map(|(mode, by)| DeclaredFloor { mode, by })
    }

    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = PermissionItems::read(context.layer());
        match items.into_file() {
            None => (self, found),
            Some((path, file)) => {
                let origin = Origin::new(context.layer(), path.clone());
                let (state, problems) = self.apply(&file, &origin, &place(layer, &path));
                (state, found.merge(problems))
            }
        }
    }

    /// The state after the permissions `file` of `layer`, and the problems it makes. The `deny`
    /// list and the floor of the state are never lower than before, whatever the file says.
    pub(super) fn apply(
        self,
        file: &Permissions,
        origin: &Origin,
        at: &Where,
    ) -> (Self, Diagnostics) {
        let layer = origin.layer();
        let (deny, dropped) = self.deny.edited_never_shrinking(file.deny(), origin);
        let found = dropped
            .iter()
            .map(|rule| removed_deny(rule, layer, at))
            .collect::<Diagnostics>();
        let (mode, held, found_mode) = self.settle_mode(file.default_mode(), origin, at);
        let (flags, found_flags) = self.settle_flags(file, origin, at);
        let floored = file.default_mode().is_some_and(|requested| {
            matches!(requested.binding(), Binding::Floor)
                && self.held.floor.admits(&requested.mode())
        });
        (
            Self {
                allow: self.allow.edited(file.allow(), origin),
                ask: self.ask.edited(file.ask(), origin),
                deny,
                mode,
                held,
                flags,
                declarers: self.declarers_after(file, layer, floored),
            },
            found.merge(found_mode).merge(found_flags),
        )
    }

    /// The flags after `layer`: a flag set to `true` is kept, or newly set; a flag that is already
    /// set and that `layer` writes as `false` stays set, with an error.
    fn settle_flags(
        &self,
        file: &Permissions,
        origin: &Origin,
        at: &Where,
    ) -> (BTreeMap<FloorFlag, Traced<bool>>, Diagnostics) {
        let layer = origin.layer();
        FloorFlag::ALL.into_iter().fold(
            (self.flags.clone(), Diagnostics::new()),
            |(mut flags, found), flag| match (file.flag(flag), flags.get(&flag)) {
                (Some(true), None) => {
                    flags.insert(flag, Traced::new(true, origin.clone(), Action::Define));
                    (flags, found)
                }
                (Some(false), Some(held)) => {
                    let problem = flag_unset(flag, held, layer, at);
                    (flags, found.with(problem))
                }
                (Some(true) | None, Some(_)) | (Some(false) | None, None) => (flags, found),
            },
        )
    }

    /// The mode and the floor after `layer` sets `requested`: unchanged, with an error, when the
    /// mode is looser than the floor.
    fn settle_mode(
        &self,
        requested: Option<DefaultMode>,
        origin: &Origin,
        at: &Where,
    ) -> (Option<Traced<PermissionMode>>, Held, Diagnostics) {
        let layer = origin.layer();
        match requested {
            None => (self.mode.clone(), self.held.clone(), Diagnostics::new()),
            Some(requested) if !self.held.floor.admits(&requested.mode()) => (
                self.mode.clone(),
                self.held.clone(),
                Diagnostics::new().with(looser_than_floor(requested.mode(), &self.held, layer, at)),
            ),
            Some(requested) => (
                Some(match self.mode.clone() {
                    Some(earlier) => {
                        earlier.revised(requested.mode(), origin.clone(), Action::Replace)
                    }
                    None => Traced::new(requested.mode(), origin.clone(), Action::Define),
                }),
                match requested.binding() {
                    Binding::Floor => self.held.clone().raised(requested.mode(), layer),
                    Binding::Free => self.held.clone(),
                },
                Diagnostics::new(),
            ),
        }
    }

    /// A warning for each allow rule that the deny list holds too: the rule has no effect, because
    /// deny wins. It has no place, as the rule may come from any layer.
    pub(super) fn shadowed(&self) -> Diagnostics {
        let denied = self.deny.members();
        self.allow
            .0
            .iter()
            .filter(|rule| denied.contains(rule.value()))
            .map(|rule| {
                Diagnostic::warning(
                    Code::PermissionShadowed,
                    None,
                    Message::new("allow rule '")
                        .then(Stripped::from_untrusted(&rule.value().to_string()))
                        .then("' is also denied; deny wins"),
                )
            })
            .collect()
    }

    /// The merged permissions.
    pub(super) fn finish(self) -> MergedPermissions {
        MergedPermissions {
            allow: self.allow.0,
            ask: self.ask.0,
            deny: self.deny.0,
            default_mode: self.mode,
            floor: self.held.floor.get().copied(),
            flags: self.flags,
            declarers: self.declarers,
        }
    }
}

/// The layer that first put a rule in its list.
fn set_by(rule: &Traced<PermissionRule>) -> Option<&LayerName> {
    rule.provenance()
        .contributions()
        .first()
        .map(crate::merge::provenance::Contribution::layer)
}

fn removed_deny(rule: &Traced<PermissionRule>, layer: &LayerName, at: &Where) -> Diagnostic {
    let named = Message::new("deny rule '")
        .then(Stripped::from_untrusted(&rule.value().to_string()))
        .then("'");
    let message = set_by(rule)
        .map_or(named.clone(), |owner| {
            named
                .clone()
                .then(" is set by layer ")
                .then(MessagePart::printable(owner))
        })
        .then(" and cannot be removed by layer ")
        .then(MessagePart::printable(layer));
    Diagnostic::error(Code::PermissionFloor, Some(at.clone()), message)
}

fn flag_unset(flag: FloorFlag, held: &Traced<bool>, layer: &LayerName, at: &Where) -> Diagnostic {
    let named = Message::new(flag.field()).then(" is true");
    let message = held
        .provenance()
        .contributions()
        .first()
        .map_or(named.clone(), |first| {
            named
                .clone()
                .then(" as set by layer ")
                .then(MessagePart::printable(first.layer()))
        })
        .then(" and cannot be set to false by layer ")
        .then(MessagePart::printable(layer));
    Diagnostic::error(Code::PermissionFloor, Some(at.clone()), message)
}

fn looser_than_floor(
    mode: PermissionMode,
    held: &Held,
    layer: &LayerName,
    at: &Where,
) -> Diagnostic {
    let message = Message::new("default_mode '")
        .then(mode.as_str())
        .then("' is looser than the floor '")
        .then(held.floor.get().map_or("", |floor| floor.as_str()))
        .then("'");
    let message = held
        .by
        .as_ref()
        .map_or(message.clone(), |owner| {
            message
                .clone()
                .then(" set by layer ")
                .then(MessagePart::printable(owner))
        })
        .then(" and cannot be set by layer ")
        .then(MessagePart::printable(layer));
    Diagnostic::error(Code::PermissionFloor, Some(at.clone()), message)
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
