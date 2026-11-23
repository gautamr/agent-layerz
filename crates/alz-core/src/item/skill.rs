//! The skills and commands of a layer (spec §7.3, §7.4): `skills/<name>/…`, a directory of files
//! with a `SKILL.md`, and `commands/<name>.md`, which compiles into a skill that only an explicit
//! call reaches.
//!
//! A skill is its whole directory: every file below it, with the bytes the layer walk hashed, so
//! the merge replaces the directory as one value (§7.3). A command and a skill share one
//! namespace, the skill names. A directory or file with a problem gives no item, and the reader
//! reports every problem of the layer in one pass.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::layer::{ItemPath, LayerMeta, LayerName};
use crate::path::SafeRelPath;
use crate::schema::{Checked, SchemaErrors, require, unknown_fields, zip, zip3};
use crate::stack::ResolvedLayer;
use crate::untrusted_text::{QuotedKey, Stripped};
use crate::yaml::fields::Section;

use super::{SkillName, place, read_markdown, schema_diagnostics, zip_diagnostics};

/// The fields a command file knows (§7.4).
const COMMAND_FIELDS: [&str; 2] = ["description", "argument_hint"];

/// The `description` of a skill or a command: text with a character that is not white space.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillDescription(String);

/// A blank `description`. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillDescriptionError;

impl SkillDescriptionError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must not be empty"
    }
}

impl SkillDescription {
    /// The text as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for SkillDescription {
    type Error = SkillDescriptionError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            Err(SkillDescriptionError)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// A command's `argument_hint`, as written: the text a client shows after the command's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentHint(String);

impl ArgumentHint {
    /// The text as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One file of a skill's directory, with its bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct SkillFile {
    path: SafeRelPath,
    bytes: Vec<u8>,
}

impl SkillFile {
    /// The path in the layer, `skills/<name>/<file>`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The segments of the path inside the skill's directory, `SKILL.md` or `references/x.md`.
    pub fn within(&self) -> impl Iterator<Item = &str> + '_ {
        self.path.segments().skip(2)
    }

    /// The bytes of the file.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for SkillFile {
    /// The path and the size, never the bytes: a skill's files may hold anything.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SkillFile")
            .field("path", &self.path)
            .field("size", &self.bytes.len())
            .finish()
    }
}

/// A skill: its directory with every file, in name order. Its identity across layers is the
/// directory's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDirectory {
    name: SkillName,
    folder: SafeRelPath,
    description: SkillDescription,
    files: Vec<SkillFile>,
}

impl SkillDirectory {
    /// The skill's folder in the layer, `skills/<name>`: what a header and `alz explain` name as
    /// its source.
    #[must_use]
    pub const fn folder(&self) -> &SafeRelPath {
        &self.folder
    }

    /// The skill's name, equal to its directory's.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        &self.name
    }

    /// The `description` of the `SKILL.md`.
    #[must_use]
    pub const fn description(&self) -> &SkillDescription {
        &self.description
    }

    /// Every file of the directory, `SKILL.md` included, in name order.
    #[must_use]
    pub fn files(&self) -> &[SkillFile] {
        &self.files
    }
}

/// A command, `commands/<name>.md`: a skill that only an explicit call reaches (§7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    path: SafeRelPath,
    name: SkillName,
    description: SkillDescription,
    argument_hint: Option<ArgumentHint>,
    body: String,
}

impl Command {
    /// The path in the layer, `commands/<name>.md`.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The command's name, the file name without `.md`.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        &self.name
    }

    /// The `description`.
    #[must_use]
    pub const fn description(&self) -> &SkillDescription {
        &self.description
    }

    /// The `argument_hint`, when the file has one.
    #[must_use]
    pub const fn argument_hint(&self) -> Option<&ArgumentHint> {
        self.argument_hint.as_ref()
    }

    /// The body, trimmed.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// What a name stands for after the merge or in one layer: a skill's whole directory, or a
/// command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skill {
    /// `skills/<name>/…`.
    Directory(SkillDirectory),
    /// `commands/<name>.md`.
    Command(Command),
}

impl Skill {
    /// The layer path the skill is named by as a source: a skill's folder, or the command's file.
    #[must_use]
    pub const fn source(&self) -> &SafeRelPath {
        match self {
            Self::Directory(directory) => directory.folder(),
            Self::Command(command) => command.path(),
        }
    }

    /// The name, which is the identity across layers.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        match self {
            Self::Directory(directory) => directory.name(),
            Self::Command(command) => command.name(),
        }
    }
}

/// What one layer holds of §7.3 and §7.4: its skills and commands in name order (skills first),
/// and the skill names its `requires_slots` lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillItems {
    skills: Vec<Skill>,
    requires: Vec<SkillName>,
}

impl SkillItems {
    /// The skills and commands. A name is there once: a command with the name of a skill of its
    /// own layer is an error and gives no item.
    #[must_use]
    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    /// The skill names the layer's `requires_slots` lists, in the order written.
    #[must_use]
    pub fn requires(&self) -> &[SkillName] {
        &self.requires
    }

    /// The skills and the required slots, for a caller that takes them apart.
    #[must_use]
    pub fn into_parts(self) -> (Vec<Skill>, Vec<SkillName>) {
        (self.skills, self.requires)
    }

    /// Reads the skills, the commands and the `requires_slots` of `layer`. Every problem is
    /// reported; a skill or command with a problem gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        let name = layer.meta().name();
        let folders = layer.contents().iter().fold(
            BTreeMap::<&str, Vec<(&SafeRelPath, &str, &[u8])>>::new(),
            |mut folders, (path, bytes)| {
                if let ItemPath::SkillFile { skill, file } = ItemPath::of(path) {
                    folders.entry(skill).or_default().push((path, file, bytes));
                }
                folders
            },
        );
        let (directories, found) = collect(
            folders
                .iter()
                .map(|(folder, files)| read_directory(name, folder, files)),
        );
        let (commands, found_commands) = collect(layer.contents().iter().filter_map(
            |(path, bytes)| match ItemPath::of(path) {
                ItemPath::Command { name: stem } => Some(read_command(name, path, stem, bytes)),
                ItemPath::LayerYaml
                | ItemPath::Instruction { .. }
                | ItemPath::Agent { .. }
                | ItemPath::AgentOverlay { .. }
                | ItemPath::SkillFile { .. }
                | ItemPath::Rule { .. }
                | ItemPath::Hooks
                | ItemPath::HookScript { .. }
                | ItemPath::Permissions
                | ItemPath::Mcp
                | ItemPath::Models
                | ItemPath::Settings
                | ItemPath::Passthrough { .. }
                | ItemPath::ClaudeExtra
                | ItemPath::Other => None,
            },
        ));
        let taken: BTreeSet<&SkillName> = directories
            .iter()
            .filter_map(|skill| match skill {
                Skill::Directory(directory) => Some(directory.name()),
                Skill::Command(_) => None,
            })
            .collect();
        let (clashing, commands): (Vec<Command>, Vec<Command>) = commands
            .into_iter()
            .partition(|command| taken.contains(command.name()));
        let clashes: Diagnostics = clashing
            .iter()
            .map(|command| clash(name, command))
            .collect();
        let (requires, found_requires) = read_requires(layer.meta());
        (
            Self {
                skills: directories
                    .into_iter()
                    .chain(commands.into_iter().map(Skill::Command))
                    .collect(),
                requires,
            },
            found
                .merge(found_commands)
                .merge(clashes)
                .merge(found_requires),
        )
    }

    /// The `slot` errors for the layer's `provides_slots`: an entry without a skill of this layer
    /// to fill it. This is the per-layer check of `alz lint` (§10); a build does not make it.
    #[must_use]
    pub fn check_provided(&self, meta: &LayerMeta) -> Diagnostics {
        let names: BTreeSet<&str> = self
            .skills
            .iter()
            .map(|skill| skill.name().as_str())
            .collect();
        meta.provides_slots()
            .iter()
            .filter(|slot| !names.contains(slot.as_str()))
            .map(|slot| {
                Diagnostic::error(
                    Code::Slot,
                    Some(layer_yaml(meta.name())),
                    Message::new("provides_slots lists ")
                        .then(Stripped::from_untrusted(slot.as_str()))
                        .then(", but the layer has no such skill"),
                )
            })
            .collect()
    }
}

/// The values and the diagnostics of a list of reads, each read giving one or the other.
fn collect<T>(reads: impl Iterator<Item = Result<T, Diagnostics>>) -> (Vec<T>, Diagnostics) {
    reads.fold(
        (Vec::new(), Diagnostics::new()),
        |(mut items, found), read| match read {
            Ok(item) => {
                items.push(item);
                (items, found)
            }
            Err(problems) => (items, found.merge(problems)),
        },
    )
}

fn layer_yaml(layer: &LayerName) -> Where {
    Where::layer_file(
        Stripped::from_untrusted(layer.as_str()),
        Stripped::from_untrusted("layer.yaml"),
    )
}

/// The `requires_slots` of a layer as skill names, and an error for each entry that cannot be one:
/// a stub is a directory with that name, so the name must be a skill name.
fn read_requires(meta: &LayerMeta) -> (Vec<SkillName>, Diagnostics) {
    collect(meta.requires_slots().iter().map(|slot| {
        SkillName::try_from(slot.as_str()).map_err(|_not_kebab| {
            Diagnostics::new().with(Diagnostic::error(
                Code::Slot,
                Some(layer_yaml(meta.name())),
                Message::new("requires_slots entry ")
                    .then(Stripped::from_untrusted(slot.as_str()))
                    .then(" is not a kebab-case skill name"),
            ))
        })
    }))
}

fn clash(layer: &LayerName, command: &Command) -> Diagnostic {
    Diagnostic::error(
        Code::SkillCommandClash,
        Some(place(layer, command.path())),
        Message::new(MessagePart::printable(command.name()))
            .then(" is both a skill and a command in this layer"),
    )
}

/// The frontmatter of a `SKILL.md`: `name` and `description`, both required. Other fields are the
/// Agent Skills standard's and pass through in the file.
fn skill_fields(section: &Section<'_>) -> Checked<(SkillName, SkillDescription)> {
    zip(
        require::<SkillName>(section, "name"),
        require::<SkillDescription>(section, "description"),
    )
}

fn read_directory(
    layer: &LayerName,
    folder: &str,
    files: &[(&SafeRelPath, &str, &[u8])],
) -> Result<Skill, Diagnostics> {
    let directory_at = Where::layer_file(
        Stripped::from_untrusted(layer.as_str()),
        Stripped::from_untrusted(&format!("skills/{folder}")),
    );
    let (md_path, md_bytes) = files
        .iter()
        .find(|(_, file, _)| *file == "SKILL.md")
        .map(|(path, _, bytes)| (*path, *bytes))
        .ok_or_else(|| {
            Diagnostics::new().with(Diagnostic::error(
                Code::Skill,
                Some(directory_at),
                Message::new("skill directory ")
                    .then(QuotedKey::from_untrusted(folder))
                    .then(" has no SKILL.md"),
            ))
        })?;
    let at = place(layer, md_path);
    let doc = read_markdown(layer, md_path, md_bytes)
        .map_err(|problem| Diagnostics::new().with(problem))?;
    let directory = SkillName::try_from(folder).map_err(|_not_kebab| {
        Diagnostics::new().with(Diagnostic::error(
            Code::Skill,
            Some(at.clone()),
            Message::new("skill name ")
                .then(QuotedKey::from_untrusted(folder))
                .then(" must be kebab-case: lower-case letters and digits in groups joined by '-'"),
        ))
    });
    let fields = skill_fields(&doc.front().section())
        .map_err(|errors| schema_diagnostics(Code::Skill, &at, &errors));
    let (directory, (name, description)) = zip_diagnostics(directory, fields)?;
    if name == directory {
        Ok(Skill::Directory(SkillDirectory {
            name,
            // The folder is a prefix of `md_path`, so it is a safe path; `md_path` stands in for it
            // only if that were ever wrong.
            folder: SafeRelPath::try_from(format!("skills/{folder}").as_str())
                .unwrap_or_else(|_unsafe| md_path.clone()),
            description,
            files: files
                .iter()
                .map(|(path, _, bytes)| SkillFile {
                    path: (*path).clone(),
                    bytes: bytes.to_vec(),
                })
                .collect(),
        }))
    } else {
        Err(Diagnostics::new().with(Diagnostic::error(
            Code::Skill,
            Some(at),
            Message::new("name must match the directory name, ")
                .then(MessagePart::printable(&directory)),
        )))
    }
}

/// The frontmatter of a command: the fields it knows, a required `description` and an optional
/// `argument_hint`.
fn command_fields(section: &Section<'_>) -> Checked<(SkillDescription, Option<ArgumentHint>)> {
    zip3(
        unknown_fields(section, &COMMAND_FIELDS),
        require::<SkillDescription>(section, "description"),
        section
            .text("argument_hint")
            .map(|hint| hint.map(|text| ArgumentHint(text.to_owned())))
            .map_err(SchemaErrors::from),
    )
    .map(|((), description, hint)| (description, hint))
}

fn read_command(
    layer: &LayerName,
    path: &SafeRelPath,
    stem: &str,
    bytes: &[u8],
) -> Result<Command, Diagnostics> {
    let at = place(layer, path);
    let name = SkillName::try_from(stem).map_err(|_not_kebab| {
        Diagnostics::new().with(Diagnostic::error(
            Code::Command,
            Some(at.clone()),
            Message::new("command name ")
                .then(QuotedKey::from_untrusted(stem))
                .then(" must be kebab-case: lower-case letters and digits in groups joined by '-'"),
        ))
    })?;
    let doc =
        read_markdown(layer, path, bytes).map_err(|problem| Diagnostics::new().with(problem))?;
    let (description, argument_hint) = command_fields(&doc.front().section())
        .map_err(|errors| schema_diagnostics(Code::Command, &at, &errors))?;
    Ok(Command {
        path: path.clone(),
        name,
        description,
        argument_hint,
        body: doc.body().as_str().trim().to_owned(),
    })
}

#[cfg(test)]
mod tests;
