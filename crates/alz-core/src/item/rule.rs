//! The path-scoped rules of a layer (spec §7.10): `rules/<name>.md`, Markdown with frontmatter
//! `paths: [<globs>]` and an optional `mode: extend`.

use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::layer::{ItemPath, LayerName};
use crate::merge::OrderedSet;
use crate::path::SafeRelPath;
use crate::schema::{Checked, refine, refine_list, unknown_fields, zip3};
use crate::stack::ResolvedLayer;
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::Section;

use super::kebab_case::is_kebab_case;
use super::{place, read_markdown, schema_diagnostics};

/// The fields a rule file knows.
const FIELDS: [&str; 2] = ["paths", "mode"];

/// The name of a rule: its file name without `.md`, in kebab-case, `^[a-z0-9]+(?:-[a-z0-9]+)*$`.
/// A rule's identity across layers (§7.10). Its alphabet is printable ASCII, so it goes into a
/// message as it is, and its `Ord` is the code point order of §6.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleName(String);

/// Text that is not a rule name. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleNameError;

impl RuleName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for RuleName {
    type Error = RuleNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let kebab = is_kebab_case(text);
        if kebab {
            Ok(Self(text.to_owned()))
        } else {
            Err(RuleNameError)
        }
    }
}

impl fmt::Display for RuleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl crate::diagnostic::sealed::Sealed for RuleName {}

impl crate::diagnostic::Printable for RuleName {
    fn printable(&self) -> &str {
        &self.0
    }
}

/// One glob of a rule's `paths`, relative to the repository root: text with at least one
/// character that is not white space. It is kept as written; what it matches is the target's
/// business.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Glob(String);

/// A glob that is empty or white space. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobError;

impl GlobError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be a non-empty glob"
    }
}

impl Glob {
    /// The glob as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Glob {
    type Error = GlobError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            Err(GlobError)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// What `mode:` says a rule file does. Without the field a rule replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleMode {
    /// `replace`: the whole file replaces the rule of a more general layer.
    Replace,
    /// `extend`: the body is appended under a layer heading and the paths are combined.
    Extend,
}

/// A `mode` that is neither `replace` nor `extend`. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleModeError;

impl RuleModeError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be replace or extend"
    }
}

impl TryFrom<&str> for RuleMode {
    type Error = RuleModeError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "replace" => Ok(Self::Replace),
            "extend" => Ok(Self::Extend),
            _ => Err(RuleModeError),
        }
    }
}

/// A whole rule: at least one glob, and a body. Only a rule that loads for some files exists: a
/// rule without `paths` would load in every session, outside the instructions budget, which is
/// what `instructions/` is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleBase {
    paths: OrderedSet<Glob>,
    body: String,
}

impl RuleBase {
    /// A rule with these globs and this text, `None` when there is no glob.
    #[must_use]
    pub fn new(paths: OrderedSet<Glob>, body: String) -> Option<Self> {
        (!paths.is_empty()).then_some(Self { paths, body })
    }

    /// The globs, each once, in the order written.
    #[must_use]
    pub const fn paths(&self) -> &OrderedSet<Glob> {
        &self.paths
    }

    /// The body, trimmed.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// What `mode: extend` adds to a rule of a more general layer: more globs, which may be none, and
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleExtension {
    paths: OrderedSet<Glob>,
    body: String,
}

impl RuleExtension {
    /// An extension that adds these globs, which may be none, and this text.
    #[must_use]
    pub const fn new(paths: OrderedSet<Glob>, body: String) -> Self {
        Self { paths, body }
    }

    /// The globs to add, each once, in the order written.
    #[must_use]
    pub const fn paths(&self) -> &OrderedSet<Glob> {
        &self.paths
    }

    /// The text to append, trimmed; empty when the file only adds globs.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// What one rule file does: replaces a rule whole, or extends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleEdit {
    /// Defines the rule, replacing the rule of a more general layer if there is one.
    Replace(RuleBase),
    /// Adds to the rule of a more general layer.
    Extend(RuleExtension),
}

/// One `rules/<name>.md` of a layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFile {
    path: SafeRelPath,
    name: RuleName,
    edit: RuleEdit,
}

impl RuleFile {
    /// The path in the layer, `rules/<name>.md`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The rule's name.
    #[must_use]
    pub const fn name(&self) -> &RuleName {
        &self.name
    }

    /// What the file does.
    #[must_use]
    pub const fn edit(&self) -> &RuleEdit {
        &self.edit
    }
}

/// What one layer holds of §7.10: its rule files in name order (§6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleItems {
    files: Vec<RuleFile>,
}

impl RuleItems {
    /// The rule files in name order.
    #[must_use]
    pub fn files(&self) -> &[RuleFile] {
        &self.files
    }

    /// Reads the rule files of `layer`. Every problem is reported; a file with a problem gives no
    /// item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        let name = layer.meta().name();
        let (files, diagnostics) = layer.contents().iter().fold(
            (Vec::new(), Diagnostics::new()),
            |(files, found), (path, bytes)| match ItemPath::of(path) {
                ItemPath::Rule { name: stem } => match read_file(name, path, stem, bytes) {
                    Ok(file) => (files.into_iter().chain([file]).collect(), found),
                    Err(problems) => (files, found.merge(problems)),
                },
                ItemPath::LayerYaml
                | ItemPath::Instruction { .. }
                | ItemPath::Agent { .. }
                | ItemPath::AgentOverlay { .. }
                | ItemPath::SkillFile { .. }
                | ItemPath::Command { .. }
                | ItemPath::Hooks
                | ItemPath::HookScript { .. }
                | ItemPath::Permissions
                | ItemPath::Mcp
                | ItemPath::Models
                | ItemPath::Settings
                | ItemPath::Passthrough { .. }
                | ItemPath::ClaudeExtra
                | ItemPath::Other => (files, found),
            },
        );
        (Self { files }, diagnostics)
    }
}

/// The frontmatter of a rule: its `mode` (a replacing rule when absent) and its globs.
fn fields_of(section: &Section<'_>) -> Checked<(RuleMode, OrderedSet<Glob>)> {
    zip3(
        unknown_fields(section, &FIELDS),
        refine::<RuleMode>(section, "mode"),
        refine_list::<Glob>(section, "paths"),
    )
    .map(|((), mode, paths)| {
        (
            mode.unwrap_or(RuleMode::Replace),
            paths.unwrap_or_default().into_iter().collect(),
        )
    })
}

fn read_file(
    layer: &LayerName,
    path: &SafeRelPath,
    stem: &str,
    bytes: &[u8],
) -> Result<RuleFile, Diagnostics> {
    let at = place(layer, path);
    let name = RuleName::try_from(stem).map_err(|_not_kebab| {
        Diagnostics::new().with(Diagnostic::error(
            Code::Rule,
            Some(at.clone()),
            Message::new("rule name ")
                .then(QuotedKey::from_untrusted(stem))
                .then(" must be kebab-case: lower-case letters and digits in groups joined by '-'"),
        ))
    })?;
    let doc =
        read_markdown(layer, path, bytes).map_err(|problem| Diagnostics::new().with(problem))?;
    let (mode, paths) = fields_of(&doc.front().section())
        .map_err(|errors| schema_diagnostics(Code::Rule, &at, &errors))?;
    let body = doc.body().as_str().trim().to_owned();
    let edit = match mode {
        RuleMode::Extend => RuleEdit::Extend(RuleExtension::new(paths, body)),
        RuleMode::Replace => RuleBase::new(paths, body)
            .map(RuleEdit::Replace)
            .ok_or_else(|| {
                Diagnostics::new().with(Diagnostic::error(
                    Code::Rule,
                    Some(at),
                    Message::new(
                        "'paths' is required; always-loaded text belongs in instructions/",
                    ),
                ))
            })?,
    };
    Ok(RuleFile {
        path: path.clone(),
        name,
        edit,
    })
}

#[cfg(test)]
mod tests;
