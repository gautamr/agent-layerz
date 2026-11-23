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
//! Which tool entries an overlay may add is the narrowing rule of R4.6; here the edits apply as
//! written, except that an overlay cannot remove a tool from an agent that has no tool list.

use std::collections::BTreeMap;
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart};
use crate::item::{
    AgentBase, AgentDescription, AgentItems, AgentName, AgentOverlay, AgentScalars, NativeFields,
    OverlayMode, SkillName, ToolEntry, place,
};
use crate::layer::{LayerHeading, LayerMeta};
use crate::merge::algebra::{ListEdit, OrderedSet};
use crate::merge::provenance::{Action, Traced};
use crate::merge::stack::LayerContext;
use crate::target::TargetName;

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
            self.heading.kind().title(),
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
    name: AgentName,
    description: AgentDescription,
    scalars: AgentScalars,
    tools: Option<OrderedSet<ToolEntry>>,
    skills: OrderedSet<SkillName>,
    targets: Option<OrderedSet<TargetName>>,
    native: NativeFields,
    body: AgentBody,
}

/// An overlay that removes a tool from an agent that has no tool list, so inherits every tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemovesFromAllTools;

impl MergedAgent {
    /// The agent a base file defines.
    fn defined_by(base: &AgentBase) -> Self {
        Self {
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
    fn defined_by_overlay(overlay: &AgentOverlay) -> Option<Result<Self, RemovesFromAllTools>> {
        overlay.description().map(|description| {
            edited_tools(None, overlay.tools()).map(|tools| Self {
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
    ) -> Result<Self, RemovesFromAllTools> {
        edited_tools(self.tools, overlay.tools()).map(|tools| Self {
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
    pub const fn tools(&self) -> Option<&OrderedSet<ToolEntry>> {
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
/// it or only adds gives it a list, and one that removes cannot say what is left.
fn edited_tools(
    current: Option<OrderedSet<ToolEntry>>,
    edit: Option<&ListEdit<ToolEntry>>,
) -> Result<Option<OrderedSet<ToolEntry>>, RemovesFromAllTools> {
    match (current, edit) {
        (current, None) => Ok(current),
        (Some(tools), Some(edit)) => Ok(Some(edit.apply(&tools))),
        (None, Some(ListEdit::Replace(tools))) => Ok(Some(tools.clone())),
        (None, Some(ListEdit::Adjust { remove, add })) => match (remove.is_empty(), add.is_empty())
        {
            (true, true) => Ok(None),
            (true, false) => Ok(Some(add.clone())),
            (false, _) => Err(RemovesFromAllTools),
        },
    }
}

/// The state of the fold for agents.
#[derive(Debug, Default)]
pub(super) struct AgentState(BTreeMap<AgentName, Traced<MergedAgent>>);

impl AgentState {
    /// The state after one more, more specific, layer: its bases, then its overlays.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let meta = context.layer().meta();
        let (items, read) = AgentItems::read(context.layer());
        let (agents, found) = items
            .bases()
            .iter()
            .fold((self.0, read), |(agents, found), base| {
                let (agents, problem) = define(agents, meta, base);
                (agents, found.merge(problem.into_iter().collect()))
            });
        let (agents, found) =
            items
                .overlays()
                .iter()
                .fold((agents, found), |(agents, found), overlay| {
                    let (agents, problems) = refine(agents, meta, overlay);
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
    meta: &LayerMeta,
    base: &AgentBase,
) -> (BTreeMap<AgentName, Traced<MergedAgent>>, Option<Diagnostic>) {
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
                meta.name().clone(),
                Action::Define,
            ),
        );
    }
    (agents, problem)
}

/// The agents after an overlay of the layer `meta` describes, and what it found.
fn refine(
    mut agents: BTreeMap<AgentName, Traced<MergedAgent>>,
    meta: &LayerMeta,
    overlay: &AgentOverlay,
) -> (BTreeMap<AgentName, Traced<MergedAgent>>, Diagnostics) {
    let at = || Some(place(meta.name(), overlay.path()));
    let layer = meta.name().clone();
    let inherits_all = || {
        Diagnostic::error(
            Code::AgentOverlay,
            at(),
            Message::new("agent ")
                .then(MessagePart::printable(overlay.extends()))
                .then(" inherits all tools; give it an explicit tools list before removing one"),
        )
    };
    let found = match (agents.remove(overlay.extends()), overlay.mode()) {
        (Some(previous), mode) => match previous.value().clone().refined_by(overlay, meta) {
            Ok(agent) => {
                let action = match mode {
                    OverlayMode::Extend => Action::Extend,
                    OverlayMode::Replace => Action::Replace,
                };
                let warning = matches!(mode, OverlayMode::Replace).then(|| {
                    Diagnostic::warning(
                        Code::AgentReplace,
                        at(),
                        Message::new("replaces the whole body of agent ")
                            .then(MessagePart::printable(overlay.extends())),
                    )
                });
                (
                    Some(previous.revised(agent, layer, action)),
                    warning.into_iter().collect(),
                )
            }
            Err(RemovesFromAllTools) => (Some(previous), Diagnostics::new().with(inherits_all())),
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
        (None, OverlayMode::Replace) => match MergedAgent::defined_by_overlay(overlay) {
            Some(Ok(agent)) => (
                Some(Traced::new(agent, layer, Action::Define)),
                Diagnostics::new(),
            ),
            Some(Err(RemovesFromAllTools)) => (None, Diagnostics::new().with(inherits_all())),
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

#[cfg(test)]
mod tests;
