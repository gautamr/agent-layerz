//! The permissions of a layer (spec §7.6): `permissions.yaml`, with the lists `allow`, `ask` and
//! `deny`, a `default_mode` and the `floor` flag.
//!
//! A rule is a tool entry of the neutral vocabulary, `read:.env*` or `bash:git push *`, read once
//! into a [`PermissionRule`]. A list is an edit of the inherited list ([`ListEdit`]): a YAML list
//! adds its rules, and a string of `+[..]`, `-[..]` and `[..]` segments edits them as written. This
//! differs from an overlay's `tools`, where a YAML list replaces: a permission list that quietly
//! dropped the rules of a more general layer would be a way to loosen it. `floor` is a flag of
//! `default_mode` and means nothing without one, so the pair is one value, [`DefaultMode`].

use std::collections::BTreeMap;
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::layer::ItemPath;
use crate::merge::ListEdit;
use crate::path::SafeRelPath;
use crate::schema::{Checked, SchemaError, refine, unknown_fields, zip, zip3};
use crate::stack::ResolvedLayer;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{self, Value, YamlText};

use super::list_edit::refine_edit;
use super::{AgentRefusal, PermissionMode, ToolGrant, place, schema_diagnostics};

/// The fields of `permissions.yaml`.
const FIELDS: [&str; 8] = [
    "allow",
    "ask",
    "deny",
    "default_mode",
    "floor",
    "disable_bypass_mode",
    "block_reads_outside",
    "block_writes_outside",
];

/// A first-class flag of `permissions.yaml` that only ever moves to `true` down the stack (§7.6):
/// once a layer sets it, no more specific layer may set it to `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloorFlag {
    /// `disable_bypass_mode`: the target's bypass-permissions mode is switched off.
    DisableBypassMode,
    /// `block_reads_outside`: reads outside the working directories are blocked.
    BlockReadsOutside,
    /// `block_writes_outside`: writes outside the working directories are blocked.
    BlockWritesOutside,
}

impl FloorFlag {
    /// Every flag, in field order.
    pub const ALL: [Self; 3] = [
        Self::DisableBypassMode,
        Self::BlockReadsOutside,
        Self::BlockWritesOutside,
    ];

    /// The field of `permissions.yaml`.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::DisableBypassMode => "disable_bypass_mode",
            Self::BlockReadsOutside => "block_reads_outside",
            Self::BlockWritesOutside => "block_writes_outside",
        }
    }
}

/// Text that is not a permission rule: the rule that was broken. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionRefusal {
    /// A blank entry.
    Blank,
    /// A tool that is not in the neutral vocabulary.
    UnknownTool,
    /// A `:` with nothing after it.
    EmptyPattern,
}

impl PermissionRefusal {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::Blank => "must not be empty",
            Self::UnknownTool => {
                "must name a tool: read, write, edit, grep, glob, bash, web_fetch, web_search, agent, notebook_edit, todo_write, skill or an mcp__ tool, then an optional :pattern"
            }
            Self::EmptyPattern => "must have a pattern after the ':'",
        }
    }
}

/// One rule of an `allow`, `ask` or `deny` list: a tool, and the pattern it is limited to when it
/// has one. Two rules are equal when they are written alike.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PermissionRule(ToolGrant);

impl PermissionRule {
    /// The tool and its pattern.
    #[must_use]
    pub const fn grant(&self) -> &ToolGrant {
        &self.0
    }
}

impl TryFrom<&str> for PermissionRule {
    type Error = PermissionRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let grant = ToolGrant::try_from(text).map_err(|refusal| match refusal {
            AgentRefusal::Tool => PermissionRefusal::Blank,
            AgentRefusal::Name
            | AgentRefusal::Skill
            | AgentRefusal::Description
            | AgentRefusal::Role
            | AgentRefusal::Model
            | AgentRefusal::UnknownTool
            | AgentRefusal::Isolation
            | AgentRefusal::Memory
            | AgentRefusal::PermissionMode
            | AgentRefusal::OverlayMode
            | AgentRefusal::ListEdit => PermissionRefusal::UnknownTool,
        })?;
        match grant.scope() {
            Some(scope) if scope.as_str().is_empty() => Err(PermissionRefusal::EmptyPattern),
            Some(_) | None => Ok(Self(grant)),
        }
    }
}

impl fmt::Display for PermissionRule {
    /// The rule as written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Whether a `default_mode` binds the layers after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// `floor` is absent or `false`: a more specific layer may set any mode.
    Free,
    /// `floor: true`: a more specific layer may set this mode or a stricter one.
    Floor,
}

/// A `default_mode` and whether it is a floor. The pair `floor` without a mode is not a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultMode {
    mode: PermissionMode,
    binding: Binding,
}

impl DefaultMode {
    /// The mode.
    #[must_use]
    pub const fn mode(self) -> PermissionMode {
        self.mode
    }

    /// Whether it binds the more specific layers.
    #[must_use]
    pub const fn binding(self) -> Binding {
        self.binding
    }
}

/// What one `permissions.yaml` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permissions {
    allow: ListEdit<PermissionRule>,
    ask: ListEdit<PermissionRule>,
    deny: ListEdit<PermissionRule>,
    default_mode: Option<DefaultMode>,
    flags: BTreeMap<FloorFlag, bool>,
}

impl Permissions {
    /// The edit of the `allow` list.
    #[must_use]
    pub const fn allow(&self) -> &ListEdit<PermissionRule> {
        &self.allow
    }

    /// The edit of the `ask` list.
    #[must_use]
    pub const fn ask(&self) -> &ListEdit<PermissionRule> {
        &self.ask
    }

    /// The edit of the `deny` list.
    #[must_use]
    pub const fn deny(&self) -> &ListEdit<PermissionRule> {
        &self.deny
    }

    /// The `default_mode`, when the file sets one.
    #[must_use]
    pub const fn default_mode(&self) -> Option<DefaultMode> {
        self.default_mode
    }

    /// The value the file gives `flag`, when it sets one.
    #[must_use]
    pub fn flag(&self, flag: FloorFlag) -> Option<bool> {
        self.flags.get(&flag).copied()
    }

    /// The permissions of a file that says what the arguments say. For the tests of the merge.
    #[cfg(test)]
    pub(crate) const fn new(
        allow: ListEdit<PermissionRule>,
        ask: ListEdit<PermissionRule>,
        deny: ListEdit<PermissionRule>,
        default_mode: Option<DefaultMode>,
    ) -> Self {
        Self {
            allow,
            ask,
            deny,
            default_mode,
            flags: BTreeMap::new(),
        }
    }

    /// The same file with `flag` set to `value`. For the tests of the merge.
    #[cfg(test)]
    pub(crate) fn with_flag(self, flag: FloorFlag, value: bool) -> Self {
        Self {
            flags: self.flags.into_iter().chain([(flag, value)]).collect(),
            ..self
        }
    }
}

#[cfg(test)]
impl DefaultMode {
    /// A mode and how it binds. For the tests of the merge.
    pub(crate) const fn new(mode: PermissionMode, binding: Binding) -> Self {
        Self { mode, binding }
    }
}

/// What one layer holds of §7.6: its `permissions.yaml`, read, with the path it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionItems {
    file: Option<(SafeRelPath, Permissions)>,
}

impl PermissionItems {
    /// The path of the file and what it says, `None` when the layer has no `permissions.yaml`
    /// or the file has a problem.
    #[must_use]
    pub fn into_file(self) -> Option<(SafeRelPath, Permissions)> {
        self.file
    }

    /// Reads the permissions of `layer`. Every problem of the file is reported; a file with a
    /// problem gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        layer
            .contents()
            .iter()
            .find(|(path, _)| matches!(ItemPath::of(path), ItemPath::Permissions))
            .map_or_else(
                || (Self::default(), Diagnostics::new()),
                |(path, bytes)| {
                    let at = place(layer.meta().name(), path);
                    match read_file(&at, bytes) {
                        Ok(file) => (
                            Self {
                                file: file.map(|permissions| (path.clone(), permissions)),
                            },
                            Diagnostics::new(),
                        ),
                        Err(found) => (Self::default(), found),
                    }
                },
            )
    }
}

/// The permissions of the file, `None` for an empty file.
fn read_file(
    at: &crate::diagnostic::Where,
    bytes: &[u8],
) -> Result<Option<Permissions>, Diagnostics> {
    let node = YamlText::from_bytes(bytes)
        .and_then(|text| yaml::parse(&text))
        .map_err(|problem| Diagnostics::new().with(problem.into_diagnostic(at.clone())))?;
    match node.value() {
        Value::Null => Ok(None),
        Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_)
        | Value::Map(_) => {
            let section = Section::new(&node, KeyPath::root()).map_err(|problem| {
                schema_diagnostics(
                    Code::Permissions,
                    at,
                    &SchemaError::NotAMapping(problem.position()).into(),
                )
            })?;
            let fields = zip(
                zip3(
                    unknown_fields(&section, &FIELDS),
                    list(&section, "allow"),
                    list(&section, "ask"),
                ),
                zip(
                    zip3(
                        list(&section, "deny"),
                        refine::<PermissionMode>(&section, "default_mode"),
                        floor(&section),
                    ),
                    flags(&section),
                ),
            )
            .map_err(|errors| schema_diagnostics(Code::Permissions, at, &errors))?;
            let (((), allow, ask), ((deny, mode, floor), flags)) = fields;
            let default_mode = match (mode, floor) {
                (Some(mode), floor) => Ok(Some(DefaultMode {
                    mode,
                    binding: if floor == Some(true) {
                        Binding::Floor
                    } else {
                        Binding::Free
                    },
                })),
                (None, None) => Ok(None),
                (None, Some(_)) => Err(Diagnostics::new().with(Diagnostic::error(
                    Code::Permissions,
                    Some(
                        section
                            .key_position("floor")
                            .map_or_else(|| at.clone(), |position| at.clone().at(position)),
                    ),
                    Message::new("floor needs a default_mode"),
                ))),
            }?;
            Ok(Some(Permissions {
                allow: allow.unwrap_or_default(),
                ask: ask.unwrap_or_default(),
                deny: deny.unwrap_or_default(),
                default_mode,
                flags,
            }))
        }
    }
}

/// The flags the file sets, each a boolean.
fn flags(section: &Section<'_>) -> Checked<BTreeMap<FloorFlag, bool>> {
    let [first, second, third] = FloorFlag::ALL;
    zip3(
        flag(section, first),
        flag(section, second),
        flag(section, third),
    )
    .map(|(first, second, third)| [first, second, third].into_iter().flatten().collect())
}

fn flag(section: &Section<'_>, flag: FloorFlag) -> Checked<Option<(FloorFlag, bool)>> {
    section
        .boolean(flag.field())
        .map(|value| value.map(|value| (flag, value)))
        .map_err(Into::into)
}

/// A list of rules as an edit: a YAML list adds, a string edits.
fn list(section: &Section<'_>, field: &'static str) -> Checked<Option<ListEdit<PermissionRule>>> {
    refine_edit::<PermissionRule>(section, field, |rules| ListEdit::Adjust {
        remove: crate::merge::OrderedSet::default(),
        add: rules,
    })
}

fn floor(section: &Section<'_>) -> Checked<Option<bool>> {
    section.boolean("floor").map_err(Into::into)
}

#[cfg(test)]
mod tests;
