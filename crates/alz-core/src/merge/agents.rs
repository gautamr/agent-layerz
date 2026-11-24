//! The merge of agents (spec §7.2).
//!
//! An agent's identity is its name. A layer's base files define agents and its overlays refine
//! them, bases first within a layer, so an overlay can refine an agent its own layer defines:
//!
//! - A second base definition of a name is an error; an overlay or `mode: replace` is the way to
//!   change an agent.
//! - Scalars: the most specific layer that sets one wins ([`AgentScalars`]).
//! - Lists (`skills`, `targets`, `tools`): each overlay's edit applies in stack order, and the
//!   result holds each item once, in the order first seen ([`ListEdit`]).
//! - Body: `extend` appends one section per layer, headed `## <Layer kind> context: <layer
//!   name>`; `replace` makes the overlay's text the whole body and warns (`agent-replace`).
//! - An overlay with `mode: replace` and no base defines a new agent, which needs a `description`;
//!   any other overlay without a base is `unresolved`.
//! - Native fields: deep-merged per target ([`NativeFields`]).
//!
//! - Tools: after an overlay's edit every entry must be covered by an entry of the agent it refines
//!   ([`covers`]), unless the overlay sets `allow_widen` and its layer is not public; an entry
//!   that is not is a `tool-widen` error naming the agent, the layer and the entry, and the overlay
//!   is not applied. `allow_widen` in a public layer is a `tool-widen` error too. An overlay
//!   cannot remove a tool from an agent that has no tool list.
//! - Strictness: an overlay may only make an agent stricter. Its `permission_mode` is no looser than
//!   the mode of the agent it refines nor than the permissions floor of the more general layers,
//!   and `claude.disallowedTools` keeps every entry a more general layer set. An overlay that does
//!   otherwise is an `agent-overlay` error and is not applied.

use std::collections::BTreeMap;
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{
    AgentBase, AgentDescription, AgentItems, AgentName, AgentOverlay, AgentScalars, NativeFields,
    NativeValue, OverlayMode, PermissionMode, SkillName, ToolGrant, place,
};
use crate::layer::{LayerHeading, LayerMeta, Visibility};
use crate::merge::action::Action;
use crate::merge::algebra::{ListEdit, OrderedSet, Tree, widened};
use crate::merge::permissions::DeclaredFloor;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::path::SafeRelPath;
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

/// The text one overlay added to an agent's body, under the heading of its layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSection {
    heading: LayerHeading,
    text: String,
}

impl ContextSection {
    /// The layer that wrote the section.
    #[must_use]
    pub const fn heading(&self) -> &LayerHeading {
        &self.heading
    }

    /// The text, trimmed and not empty.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for ContextSection {
    /// The section as the body holds it: `## <Layer kind> context: <layer name>`, an empty line
    /// and the text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "## {} context: {}\n\n{}",
            Stripped::from_untrusted(&self.heading.kind().title()),
            self.heading.name(),
            self.text
        )
    }
}

/// The body of an agent: the text of the file that last defined or replaced it, and the sections
/// of the overlays that extended it since.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentBody {
    text: String,
    sections: Vec<ContextSection>,
}

impl AgentBody {
    /// The text of the defining file.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The sections of the overlays that extended the body, in stack order.
    #[must_use]
    pub fn sections(&self) -> &[ContextSection] {
        &self.sections
    }

    /// The whole body: the text, then each section, separated by one empty line. A part that is
    /// empty is left out.
    #[must_use]
    pub fn compose(&self) -> String {
        std::iter::once(self.text.clone())
            .chain(self.sections.iter().map(ToString::to_string))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The body after an overlay of the layer `meta` describes: the text of an `extend` overlay
    /// becomes a section when it has any, and the text of a `replace` overlay becomes the body.
    fn then(self, mode: OverlayMode, text: &str, meta: &LayerMeta) -> Self {
        match mode {
            OverlayMode::Replace => Self {
                text: text.to_owned(),
                sections: Vec::new(),
            },
            OverlayMode::Extend if text.is_empty() => self,
            OverlayMode::Extend => Self {
                text: self.text,
                sections: self
                    .sections
                    .into_iter()
                    .chain([ContextSection {
                        heading: LayerHeading::of(meta),
                        text: text.to_owned(),
                    }])
                    .collect(),
            },
        }
    }
}

/// An agent after the merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedAgent {
    file: SafeRelPath,
    name: AgentName,
    description: AgentDescription,
    scalars: AgentScalars,
    tools: Option<OrderedSet<ToolGrant>>,
    skills: OrderedSet<SkillName>,
    targets: Option<OrderedSet<TargetName>>,
    native: NativeFields,
    body: AgentBody,
}

/// Why an overlay cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OverlayRefused {
    /// It removes a tool from an agent that has no tool list, so inherits every tool.
    RemovesFromAllTools,
    /// It leaves entries that no entry of the agent it refines covers, and may not widen.
    Widens(Vec<ToolGrant>),
    /// It sets a `permission_mode` looser than a bound.
    LoosensPermissionMode {
        requested: PermissionMode,
        bound: Bound,
    },
    /// It takes entries out of `claude.disallowedTools`.
    ShrinksDisallowedTools,
}

/// What a `permission_mode` may not be looser than.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Bound {
    /// The mode of the agent the overlay refines.
    Agent(PermissionMode),
    /// The permissions floor of the more general layers.
    Floor(DeclaredFloor),
}

impl Bound {
    const fn mode(&self) -> PermissionMode {
        match self {
            Self::Agent(mode) => *mode,
            Self::Floor(floor) => floor.mode(),
        }
    }
}

/// What an overlay loosens, given the agent it refines (when there is one) and the floor above.
fn loosened(
    refined: Option<&MergedAgent>,
    overlay: &AgentOverlay,
    floor: Option<&DeclaredFloor>,
) -> Vec<OverlayRefused> {
    let modes = overlay
        .scalars()
        .permission_mode()
        .copied()
        .into_iter()
        .flat_map(|requested| {
            refined
                .and_then(|agent| agent.scalars.permission_mode().copied())
                .map(Bound::Agent)
                .into_iter()
                .chain(floor.cloned().map(Bound::Floor))
                .filter(move |bound| requested < bound.mode())
                .map(move |bound| OverlayRefused::LoosensPermissionMode { requested, bound })
        });
    let shrinks = refined
        .filter(|agent| shrinks_disallowed_tools(agent, overlay))
        .map(|_| OverlayRefused::ShrinksDisallowedTools);
    modes.chain(shrinks).collect()
}

/// The entries of `claude.disallowedTools` of `agent` that its merge with `overlay` no longer has.
fn shrinks_disallowed_tools(agent: &MergedAgent, overlay: &AgentOverlay) -> bool {
    let entries = |fields: &NativeFields| -> Vec<Tree> {
        fields
            .of(TargetName::Claude)
            .and_then(|claude| claude.get(DISALLOWED_TOOLS))
            .map_or_else(Vec::new, |value| match value {
                NativeValue::Null => Vec::new(),
                NativeValue::List(items) => items.iter().map(NativeValue::to_tree).collect(),
                NativeValue::Bool(_)
                | NativeValue::Int(_)
                | NativeValue::Float(_)
                | NativeValue::Text(_)
                | NativeValue::Map(_) => vec![value.to_tree()],
            })
    };
    let after = entries(&agent.native.clone().merge(overlay.native().clone()));
    entries(&agent.native)
        .iter()
        .any(|kept| !after.contains(kept))
}

/// The native field of Claude that lists the tools an agent may not use.
const DISALLOWED_TOOLS: &str = "disallowedTools";

impl MergedAgent {
    /// The agent a base file defines.
    fn defined_by(base: &AgentBase) -> Self {
        Self {
            file: base.path().clone(),
            name: base.name().clone(),
            description: base.description().clone(),
            scalars: base.scalars().clone(),
            tools: base.tools().cloned(),
            skills: base.skills().clone(),
            targets: base.targets().cloned(),
            native: base.native().clone(),
            body: AgentBody {
                text: base.body().to_owned(),
                sections: Vec::new(),
            },
        }
    }

    /// The agent a `replace` overlay defines when no more general layer has one: the overlay's
    /// edits apply to an agent with nothing. `None` when the overlay has no `description`.
    fn defined_by_overlay(
        overlay: &AgentOverlay,
        floor: Option<&DeclaredFloor>,
    ) -> Option<Result<Self, Vec<OverlayRefused>>> {
        overlay.description().map(|description| {
            refusals(
                loosened(None, overlay, floor),
                edited_tools(None, overlay.tools(), overlay.allow_widen()),
            )
            .map(|tools| Self {
                file: overlay.path().clone(),
                name: overlay.extends().clone(),
                description: description.clone(),
                scalars: overlay.scalars().clone(),
                tools,
                skills: overlay.skills().apply(&OrderedSet::default()),
                targets: overlay
                    .targets()
                    .map(|edit| edit.apply(&OrderedSet::default())),
                native: overlay.native().clone(),
                body: AgentBody {
                    text: overlay.body().to_owned(),
                    sections: Vec::new(),
                },
            })
        })
    }

    /// The agent after an overlay of the layer `meta` describes.
    fn refined_by(
        self,
        overlay: &AgentOverlay,
        meta: &LayerMeta,
        floor: Option<&DeclaredFloor>,
    ) -> Result<Self, Vec<OverlayRefused>> {
        refusals(
            loosened(Some(&self), overlay, floor),
            edited_tools(self.tools.as_ref(), overlay.tools(), overlay.allow_widen()),
        )
        .map(|tools| Self {
            file: self.file,
            name: self.name,
            description: overlay.description().cloned().unwrap_or(self.description),
            scalars: self.scalars.merge(overlay.scalars().clone()),
            tools,
            skills: overlay.skills().apply(&self.skills),
            targets: match overlay.targets() {
                Some(edit) => Some(edit.apply(&self.targets.unwrap_or_default())),
                None => self.targets,
            },
            native: self.native.merge(overlay.native().clone()),
            body: self.body.then(overlay.mode(), overlay.body(), meta),
        })
    }

    /// The file that defined the agent: its base, or the overlay with `mode: replace` that made it.
    #[must_use]
    pub const fn file(&self) -> &SafeRelPath {
        &self.file
    }

    /// The agent's name.
    #[must_use]
    pub const fn name(&self) -> &AgentName {
        &self.name
    }

    /// When to use the agent.
    #[must_use]
    pub const fn description(&self) -> &AgentDescription {
        &self.description
    }

    /// The scalar fields.
    #[must_use]
    pub const fn scalars(&self) -> &AgentScalars {
        &self.scalars
    }

    /// The tools, `None` when the agent inherits every tool.
    #[must_use]
    pub const fn tools(&self) -> Option<&OrderedSet<ToolGrant>> {
        self.tools.as_ref()
    }

    /// The skills to preload, each once.
    #[must_use]
    pub const fn skills(&self) -> &OrderedSet<SkillName> {
        &self.skills
    }

    /// The targets the agent is restricted to, `None` for every target.
    #[must_use]
    pub const fn targets(&self) -> Option<&OrderedSet<TargetName>> {
        self.targets.as_ref()
    }

    /// The native fields of each target.
    #[must_use]
    pub const fn native(&self) -> &NativeFields {
        &self.native
    }

    /// The body.
    #[must_use]
    pub const fn body(&self) -> &AgentBody {
        &self.body
    }
}

/// The tool list after an edit. An agent with no list inherits every tool: an edit that replaces
/// it or only adds gives it a list, and one that removes cannot say what is left. Every entry of
/// the result must be covered by an entry of `current`, unless `allow_widen`.
fn edited_tools(
    current: Option<&OrderedSet<ToolGrant>>,
    edit: Option<&ListEdit<ToolGrant>>,
    allow_widen: bool,
) -> Result<Option<OrderedSet<ToolGrant>>, OverlayRefused> {
    let edited = match (current, edit) {
        (current, None) => Ok(current.cloned()),
        (Some(tools), Some(edit)) => Ok(Some(edit.apply(tools))),
        (None, Some(ListEdit::Replace(tools))) => Ok(Some(tools.clone())),
        (None, Some(ListEdit::Adjust { remove, add })) => match (remove.is_empty(), add.is_empty())
        {
            (true, true) => Ok(None),
            (true, false) => Ok(Some(add.clone())),
            (false, _) => Err(OverlayRefused::RemovesFromAllTools),
        },
    }?;
    let wide: Vec<ToolGrant> = edited.as_ref().map_or_else(Vec::new, |result| {
        widened(current, result).into_iter().cloned().collect()
    });
    if wide.is_empty() || allow_widen {
        Ok(edited)
    } else {
        Err(OverlayRefused::Widens(wide))
    }
}

/// The tools of an overlay when nothing is refused, else every refusal, in the order of the
/// problems.
fn refusals(
    loosened: Vec<OverlayRefused>,
    tools: Result<Option<OrderedSet<ToolGrant>>, OverlayRefused>,
) -> Result<Option<OrderedSet<ToolGrant>>, Vec<OverlayRefused>> {
    match (tools, loosened.is_empty()) {
        (Ok(tools), true) => Ok(tools),
        (Ok(_), false) => Err(loosened),
        (Err(refusal), _) => Err(std::iter::once(refusal).chain(loosened).collect()),
    }
}

/// The state of the fold for agents.
#[derive(Debug, Default)]
pub(super) struct AgentState(BTreeMap<AgentName, Traced<MergedAgent>>);

impl AgentState {
    /// The state after one more, more specific, layer: its bases, then its overlays.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        self.absorb_under(context, None)
    }

    /// The state after one more, more specific, layer, whose overlays may not be looser than
    /// `floor`, the permissions floor of the layers before it.
    pub(super) fn absorb_under(
        self,
        context: &LayerContext<'_>,
        floor: Option<&DeclaredFloor>,
    ) -> (Self, Diagnostics) {
        let (items, read) = AgentItems::read(context.layer());
        let (agents, found) = items
            .bases()
            .iter()
            .fold((self.0, read), |(agents, found), base| {
                let (agents, problem) = define(agents, context.layer(), base);
                (agents, found.merge(problem.into_iter().collect()))
            });
        let (agents, found) =
            items
                .overlays()
                .iter()
                .fold((agents, found), |(agents, found), overlay| {
                    let (agents, problems) = refine(agents, context.layer(), overlay, floor);
                    (agents, found.merge(problems))
                });
        (Self(agents), found)
    }

    /// The agents.
    pub(super) fn finish(self) -> BTreeMap<AgentName, Traced<MergedAgent>> {
        self.0
    }
}

/// The agents after a base file of the layer `meta` describes, and the problem when an agent of
/// that name is already defined.
fn define(
    mut agents: BTreeMap<AgentName, Traced<MergedAgent>>,
    layer: &ResolvedLayer,
    base: &AgentBase,
) -> (BTreeMap<AgentName, Traced<MergedAgent>>, Option<Diagnostic>) {
    let meta = layer.meta();
    let problem = agents.get(base.name()).map(|existing| {
        let first = existing.provenance().contributions().first();
        let already = Message::new("agent ")
            .then(MessagePart::printable(base.name()))
            .then(" is already defined by ");
        Diagnostic::error(
            Code::AgentDuplicate,
            Some(place(meta.name(), base.path())),
            first
                .map_or_else(
                    || already.clone().then("a more general layer"),
                    |part| already.clone().then(MessagePart::printable(part.layer())),
                )
                .then("; use an overlay or mode: replace"),
        )
    });
    if problem.is_none() {
        agents.insert(
            base.name().clone(),
            Traced::new(
                MergedAgent::defined_by(base),
                Origin::new(layer, base.path().clone()),
                Action::Define,
            ),
        );
    }
    (agents, problem)
}

/// The errors for the refusals of `overlay`, a file at `at`.
fn problems(overlay: &AgentOverlay, at: &Where, refusals: Vec<OverlayRefused>) -> Diagnostics {
    let at = || Some(at.clone());
    let inherits_all = || {
        Diagnostic::error(
            Code::AgentOverlay,
            at(),
            Message::new("agent ")
                .then(MessagePart::printable(overlay.extends()))
                .then(" inherits all tools; give it an explicit tools list before removing one"),
        )
    };
    refusals
        .into_iter()
        .flat_map(|refusal| match refusal {
            OverlayRefused::RemovesFromAllTools => vec![inherits_all()],
            OverlayRefused::Widens(entries) => entries
                .iter()
                .map(|entry| {
                    Diagnostic::error(
                        Code::ToolWiden,
                        at(),
                        Message::new("overlay on agent ")
                            .then(MessagePart::printable(overlay.extends()))
                            .then(" adds tool ")
                            .then(Stripped::from_untrusted(&entry.to_string()))
                            .then(
                                ", which the agent it refines does not grant; an overlay may \
                                     only remove or narrow tools (allow_widen: true is accepted \
                                     in a layer that is not public)",
                            ),
                    )
                })
                .collect(),
            OverlayRefused::LoosensPermissionMode { requested, bound } => {
                vec![loosens_mode(overlay, requested, &bound, at())]
            }
            OverlayRefused::ShrinksDisallowedTools => vec![Diagnostic::error(
                Code::AgentOverlay,
                at(),
                Message::new("overlay on agent ")
                    .then(MessagePart::printable(overlay.extends()))
                    .then(
                        " removes entries of claude.disallowedTools that a more general \
                             layer set; the list only grows down the stack",
                    ),
            )],
        })
        .collect()
}

/// The agents after an overlay of the layer `meta` describes, and what it found.
fn refine(
    mut agents: BTreeMap<AgentName, Traced<MergedAgent>>,
    layer: &ResolvedLayer,
    overlay: &AgentOverlay,
    floor: Option<&DeclaredFloor>,
) -> (BTreeMap<AgentName, Traced<MergedAgent>>, Diagnostics) {
    let meta = layer.meta();
    let at = || Some(place(meta.name(), overlay.path()));
    let origin = Origin::new(layer, overlay.path().clone());
    if overlay.allow_widen() && meta.visibility() == Visibility::Public {
        return (
            agents,
            Diagnostics::new().with(Diagnostic::error(
                Code::ToolWiden,
                at(),
                Message::new("allow_widen is not allowed in a public layer"),
            )),
        );
    }
    let refused = |refusals: Vec<OverlayRefused>| {
        problems(overlay, &place(meta.name(), overlay.path()), refusals)
    };
    let found = match (agents.remove(overlay.extends()), overlay.mode()) {
        (Some(previous), mode) => match previous.value().clone().refined_by(overlay, meta, floor) {
            Ok(agent) => {
                let warning = matches!(mode, OverlayMode::Replace).then(|| {
                    Diagnostic::warning(
                        Code::AgentReplace,
                        at(),
                        Message::new("replaces the whole body of agent ")
                            .then(MessagePart::printable(overlay.extends())),
                    )
                });
                (
                    Some(previous.revised(agent, origin, Action::overlay(overlay))),
                    warning.into_iter().collect(),
                )
            }
            Err(refusal) => (Some(previous), refused(refusal)),
        },
        (None, OverlayMode::Extend) => (
            None,
            Diagnostics::new().with(Diagnostic::error(
                Code::Unresolved,
                at(),
                Message::new("extends ")
                    .then(MessagePart::printable(overlay.extends()))
                    .then(", but no more general layer defines it"),
            )),
        ),
        (None, OverlayMode::Replace) => match MergedAgent::defined_by_overlay(overlay, floor) {
            Some(Ok(agent)) => (
                Some(Traced::new(agent, origin, Action::Define)),
                Diagnostics::new(),
            ),
            Some(Err(refusal)) => (None, refused(refusal)),
            None => (
                None,
                Diagnostics::new().with(Diagnostic::error(
                    Code::Agent,
                    at(),
                    Message::new("description is required to define a new agent"),
                )),
            ),
        },
    };
    let (agent, problems) = found;
    if let Some(agent) = agent {
        agents.insert(overlay.extends().clone(), agent);
    }
    (agents, problems)
}

/// The error for an overlay whose `permission_mode` is looser than `bound`.
fn loosens_mode(
    overlay: &AgentOverlay,
    requested: PermissionMode,
    bound: &Bound,
    at: Option<Where>,
) -> Diagnostic {
    let sets = Message::new("overlay on agent ")
        .then(MessagePart::printable(overlay.extends()))
        .then(" sets permission_mode '")
        .then(requested.as_str())
        .then("', which is looser than ");
    let message = match bound {
        Bound::Agent(mode) => sets
            .then("'")
            .then(mode.as_str())
            .then("' of the agent it refines"),
        Bound::Floor(floor) => sets
            .then("the permissions floor '")
            .then(floor.mode().as_str())
            .then("' set by layer ")
            .then(MessagePart::printable(floor.by())),
    };
    Diagnostic::error(
        Code::AgentOverlay,
        at,
        message.then("; an overlay may only make an agent stricter"),
    )
}

#[cfg(test)]
mod narrowing_tests;
#[cfg(test)]
mod overlay_strictness;
#[cfg(test)]
mod tests;
