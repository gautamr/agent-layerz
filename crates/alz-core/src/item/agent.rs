//! The agent files of a layer (spec §7.2): `agents/<name>.md`, a full definition in the neutral
//! schema, and `agents/<name>.overlay.md`, an overlay on the agent of that name from a more
//! general layer.
//!
//! A file is read into one value, or into diagnostics, and a file with a problem gives no item.
//! A tool entry is read into a [`ToolGrant`]; which entries an overlay may add is the narrowing
//! rule of the merge (spec §7.2).

mod native;
mod ordered_tree;
mod tool;
mod vocabulary;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::{ItemPath, LayerName};
use crate::markdown::MarkdownDoc;
use crate::merge::{Last, ListEdit, OrderedSet};
use crate::path::SafeRelPath;
use crate::schema::{Checked, refine, refine_list, require, unknown_fields, zip, zip3};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;
use crate::yaml::fields::Section;

use super::list_edit::refine_edit;
use super::{place, read_markdown, schema_diagnostics, zip_diagnostics};
pub use native::NativeFields;
pub(in crate::item) use native::tree_of;
pub use ordered_tree::{NativeMap, NativeValue};
pub use tool::{McpToolName, NeutralTool, ToolGrant, ToolName, ToolScope};
pub use vocabulary::{
    AgentDescription, AgentName, AgentRefusal, Isolation, Memory, ModelId, OverlayMode,
    PermissionMode, Role, SkillName,
};

/// The fields of a base.
const BASE_FIELDS: [&str; 12] = [
    "name",
    "description",
    "role",
    "model",
    "tools",
    "skills",
    "isolation",
    "permission_mode",
    "memory",
    "targets",
    "claude",
    "codex",
];

/// The fields of an overlay: those of a base but `name`, and what makes it an overlay.
const OVERLAY_FIELDS: [&str; 14] = [
    "extends",
    "mode",
    "allow_widen",
    "description",
    "role",
    "model",
    "tools",
    "skills",
    "isolation",
    "permission_mode",
    "memory",
    "targets",
    "claude",
    "codex",
];

/// The scalar fields of an agent that a layer may leave unset, where the most specific layer that
/// sets one wins (§7.2). `description` is not among them: a base must have one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentScalars {
    role: Last<Role>,
    model: Last<ModelId>,
    isolation: Last<Isolation>,
    permission_mode: Last<PermissionMode>,
    memory: Last<Memory>,
}

impl AgentScalars {
    /// The scalars after a more specific layer: each one it sets replaces the earlier value.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        Self {
            role: self.role.merge(later.role),
            model: self.model.merge(later.model),
            isolation: self.isolation.merge(later.isolation),
            permission_mode: self.permission_mode.merge(later.permission_mode),
            memory: self.memory.merge(later.memory),
        }
    }

    /// The `role`, used for model routing (§7.8).
    #[must_use]
    pub const fn role(&self) -> Option<&Role> {
        self.role.get()
    }

    /// The `model`: `inherit` or a logical model id.
    #[must_use]
    pub const fn model(&self) -> Option<&ModelId> {
        self.model.get()
    }

    /// The `isolation`.
    #[must_use]
    pub const fn isolation(&self) -> Option<&Isolation> {
        self.isolation.get()
    }

    /// The `permission_mode`.
    #[must_use]
    pub const fn permission_mode(&self) -> Option<&PermissionMode> {
        self.permission_mode.get()
    }

    /// The `memory`.
    #[must_use]
    pub const fn memory(&self) -> Option<&Memory> {
        self.memory.get()
    }
}

/// One `agents/<name>.md` of a layer: a whole agent, the base that overlays refine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBase {
    path: SafeRelPath,
    name: AgentName,
    description: AgentDescription,
    scalars: AgentScalars,
    tools: Option<OrderedSet<ToolGrant>>,
    skills: OrderedSet<SkillName>,
    targets: Option<OrderedSet<TargetName>>,
    native: NativeFields,
    body: String,
}

impl AgentBase {
    /// The path in the layer, `agents/<name>.md`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The agent's name, equal to the file stem.
    #[must_use]
    pub const fn name(&self) -> &AgentName {
        &self.name
    }

    /// When to use the agent.
    #[must_use]
    pub const fn description(&self) -> &AgentDescription {
        &self.description
    }

    /// The scalar fields the file sets.
    #[must_use]
    pub const fn scalars(&self) -> &AgentScalars {
        &self.scalars
    }

    /// The tools, `None` when the agent inherits every tool.
    #[must_use]
    pub const fn tools(&self) -> Option<&OrderedSet<ToolGrant>> {
        self.tools.as_ref()
    }

    /// The skills to preload.
    #[must_use]
    pub const fn skills(&self) -> &OrderedSet<SkillName> {
        &self.skills
    }

    /// The targets the agent is restricted to, `None` for every target.
    #[must_use]
    pub const fn targets(&self) -> Option<&OrderedSet<TargetName>> {
        self.targets.as_ref()
    }

    /// The native fields.
    #[must_use]
    pub const fn native(&self) -> &NativeFields {
        &self.native
    }

    /// The system prompt, trimmed.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// One `agents/<name>.overlay.md` of a layer: what it changes in the agent of its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentOverlay {
    path: SafeRelPath,
    extends: AgentName,
    mode: OverlayMode,
    allow_widen: bool,
    description: Option<AgentDescription>,
    scalars: AgentScalars,
    tools: Option<ListEdit<ToolGrant>>,
    skills: ListEdit<SkillName>,
    targets: Option<ListEdit<TargetName>>,
    native: NativeFields,
    body: String,
}

impl AgentOverlay {
    /// The path in the layer, `agents/<name>.overlay.md`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The agent the overlay is on, equal to the file stem.
    #[must_use]
    pub const fn extends(&self) -> &AgentName {
        &self.extends
    }

    /// Whether the overlay's text is appended or is the whole body.
    #[must_use]
    pub const fn mode(&self) -> OverlayMode {
        self.mode
    }

    /// `allow_widen: true`, which lets the overlay add a tool in a layer that is not public
    /// (the merge decides).
    #[must_use]
    pub const fn allow_widen(&self) -> bool {
        self.allow_widen
    }

    /// The description it sets, `None` to keep the inherited one.
    #[must_use]
    pub const fn description(&self) -> Option<&AgentDescription> {
        self.description.as_ref()
    }

    /// The scalar fields it sets.
    #[must_use]
    pub const fn scalars(&self) -> &AgentScalars {
        &self.scalars
    }

    /// The edit of the tools, `None` to keep them.
    #[must_use]
    pub const fn tools(&self) -> Option<&ListEdit<ToolGrant>> {
        self.tools.as_ref()
    }

    /// The edit of the skills; the identity when the overlay does not mention them.
    #[must_use]
    pub const fn skills(&self) -> &ListEdit<SkillName> {
        &self.skills
    }

    /// The edit of the targets, `None` to keep them.
    #[must_use]
    pub const fn targets(&self) -> Option<&ListEdit<TargetName>> {
        self.targets.as_ref()
    }

    /// The native fields, to deep-merge.
    #[must_use]
    pub const fn native(&self) -> &NativeFields {
        &self.native
    }

    /// The text, trimmed; empty when the overlay has none.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// What one layer holds of §7.2: its base agents and its overlays, each in file name order (§6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentItems {
    bases: Vec<AgentBase>,
    overlays: Vec<AgentOverlay>,
}

impl AgentItems {
    /// The base agents in file name order.
    #[must_use]
    pub fn bases(&self) -> &[AgentBase] {
        &self.bases
    }

    /// The overlays in file name order.
    #[must_use]
    pub fn overlays(&self) -> &[AgentOverlay] {
        &self.overlays
    }

    /// Reads the agent files of `layer`. Every problem is reported; a file with a problem gives
    /// no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        let name = layer.meta().name();
        let (items, diagnostics) = layer.contents().iter().fold(
            (Self::default(), Diagnostics::new()),
            |(items, found), (path, bytes)| match ItemPath::of(path) {
                ItemPath::Agent { name: stem } => match read_base(name, path, stem, bytes) {
                    Ok(base) => (
                        Self {
                            bases: items.bases.into_iter().chain([base]).collect(),
                            overlays: items.overlays,
                        },
                        found,
                    ),
                    Err(problems) => (items, found.merge(problems)),
                },
                ItemPath::AgentOverlay { name: stem } => {
                    match read_overlay(name, path, stem, bytes) {
                        Ok(overlay) => (
                            Self {
                                bases: items.bases,
                                overlays: items.overlays.into_iter().chain([overlay]).collect(),
                            },
                            found,
                        ),
                        Err(problems) => (items, found.merge(problems)),
                    }
                }
                ItemPath::LayerYaml
                | ItemPath::Instruction { .. }
                | ItemPath::SkillFile { .. }
                | ItemPath::Command { .. }
                | ItemPath::Rule { .. }
                | ItemPath::Hooks
                | ItemPath::HookScript { .. }
                | ItemPath::Permissions
                | ItemPath::Mcp
                | ItemPath::Models
                | ItemPath::Settings
                | ItemPath::Passthrough { .. }
                | ItemPath::ClaudeExtra
                | ItemPath::Other => (items, found),
            },
        );
        (items, diagnostics)
    }
}

/// The scalar fields of a file, `description` apart.
fn scalars(section: &Section<'_>) -> Checked<AgentScalars> {
    zip(
        zip3(
            refine::<Role>(section, "role"),
            refine::<ModelId>(section, "model"),
            refine::<Isolation>(section, "isolation"),
        ),
        zip(
            refine::<PermissionMode>(section, "permission_mode"),
            refine::<Memory>(section, "memory"),
        ),
    )
    .map(
        |((role, model, isolation), (permission_mode, memory))| AgentScalars {
            role: role.into(),
            model: model.into(),
            isolation: isolation.into(),
            permission_mode: permission_mode.into(),
            memory: memory.into(),
        },
    )
}

fn set_of<T: Eq>(items: Option<Vec<T>>) -> Option<OrderedSet<T>> {
    items.map(|items| items.into_iter().collect())
}

/// The Markdown file at `path` and its place.
fn open(
    layer: &LayerName,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Result<(Where, MarkdownDoc), Diagnostics> {
    read_markdown(layer, path, bytes)
        .map(|doc| (place(layer, path), doc))
        .map_err(|problem| Diagnostics::new().with(problem))
}

fn read_base(
    layer: &LayerName,
    path: &SafeRelPath,
    stem: &str,
    bytes: &[u8],
) -> Result<AgentBase, Diagnostics> {
    let (at, doc) = open(layer, path, bytes)?;
    let section = doc.front().section();
    let schema = zip(
        zip3(
            unknown_fields(&section, &BASE_FIELDS),
            require::<AgentName>(&section, "name"),
            require::<AgentDescription>(&section, "description"),
        ),
        zip(
            zip3(
                scalars(&section),
                refine_list::<ToolGrant>(&section, "tools"),
                refine_list::<SkillName>(&section, "skills"),
            ),
            refine_list::<TargetName>(&section, "targets"),
        ),
    )
    .map_err(|errors| schema_diagnostics(Code::Agent, &at, &errors));
    let native = NativeFields::read(&section, &at);
    let ((((), name, description), ((scalars, tools, skills), targets)), native) =
        zip_diagnostics(schema, native)?;
    if name.as_str() == stem {
        Ok(AgentBase {
            path: path.clone(),
            name,
            description,
            scalars,
            tools: set_of(tools),
            skills: skills.unwrap_or_default().into_iter().collect(),
            targets: set_of(targets),
            native,
            body: doc.body().as_str().trim().to_owned(),
        })
    } else {
        Err(Diagnostics::new().with(Diagnostic::error(
            Code::Agent,
            Some(at),
            Message::new("name must match the file name"),
        )))
    }
}

fn read_overlay(
    layer: &LayerName,
    path: &SafeRelPath,
    stem: &str,
    bytes: &[u8],
) -> Result<AgentOverlay, Diagnostics> {
    let (at, doc) = open(layer, path, bytes)?;
    let section = doc.front().section();
    let header = zip3(
        require::<AgentName>(&section, "extends"),
        refine::<OverlayMode>(&section, "mode"),
        section
            .boolean("allow_widen")
            .map_err(crate::schema::SchemaErrors::from),
    )
    .map_err(|errors| schema_diagnostics(Code::AgentOverlay, &at, &errors))
    .and_then(|(extends, mode, allow_widen)| {
        if extends.as_str() == stem {
            Ok((extends, mode, allow_widen))
        } else {
            Err(Diagnostics::new().with(Diagnostic::error(
                Code::AgentOverlay,
                Some(at.clone()),
                Message::new("extends must be ")
                    .then(Stripped::from_untrusted(stem))
                    .then(" to match the file name"),
            )))
        }
    });
    let fields = zip(
        zip3(
            unknown_fields(&section, &OVERLAY_FIELDS),
            refine::<AgentDescription>(&section, "description"),
            scalars(&section),
        ),
        zip3(
            refine_edit::<ToolGrant>(&section, "tools", ListEdit::Replace),
            refine_edit::<SkillName>(&section, "skills", ListEdit::Replace),
            refine_edit::<TargetName>(&section, "targets", ListEdit::Replace),
        ),
    )
    .map_err(|errors| schema_diagnostics(Code::Agent, &at, &errors));
    let native = NativeFields::read(&section, &at);
    let (
        ((extends, mode, allow_widen), (((), description, scalars), (tools, skills, targets))),
        native,
    ) = zip_diagnostics(zip_diagnostics(header, fields), native)?;
    Ok(AgentOverlay {
        path: path.clone(),
        extends,
        mode: mode.unwrap_or(OverlayMode::Extend),
        allow_widen: allow_widen.unwrap_or(false),
        description,
        scalars,
        tools,
        skills: skills.unwrap_or_default(),
        targets,
        native,
        body: doc.body().as_str().trim().to_owned(),
    })
}

#[cfg(test)]
mod tests;
