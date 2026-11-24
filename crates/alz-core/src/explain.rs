//! Which layers made an item, for `alz explain <type> <name>` (spec §10.3): the contributions in
//! stack order, each with its source and the kind's word for what the layer did.
//!
//! The explanation is read from the merged model alone and holds text that is safe to print:
//! the name the person typed is stripped, and so are the source and the action of each line,
//! because a layer's revision, file name and globs are untrusted text. The merged output file is
//! the caller's: [`Explanation::outputs`] names the paths the item is written to.

use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Message};
use crate::item::Skill;
use crate::merge::{Action, Merged, Origin, Traced};
use crate::render::action_word::ActionWord;
use crate::render::header::SourceRef;
use crate::render::instructions::agents_md_path;
use crate::render::tree::OutputPath;
use crate::untrusted_text::Stripped;

/// The source of a slot stub, which has no layer file.
const SLOT_STUB_SOURCE: &str = "slot stub";
/// What a slot stub is called as an action.
const STUB_WORD: &str = "stub";
/// What an instruction block is called as an action.
const BLOCK_WORD: &str = "block";
/// The suffix of a definition that a more specific layer replaced.
const OVERRIDDEN: &str = " (overridden)";
/// The file that holds hooks.
const SETTINGS_PATH: &str = ".claude/settings.json";
/// The file that holds servers.
const MCP_PATH: &str = ".mcp.json";
/// The folder of the instruction files in a layer.
const INSTRUCTIONS_DIR: &str = "instructions/";

/// The kind of item `alz explain` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemType {
    /// An agent (§7.2).
    Agent,
    /// A skill (§7.3), or the stub of a slot.
    Skill,
    /// A command (§7.4).
    Command,
    /// A path-scoped rule (§7.10).
    Rule,
    /// A hook (§7.5).
    Hook,
    /// An MCP server (§7.7).
    Mcp,
    /// An instruction file (§7.1), by file name.
    Instruction,
}

impl ItemType {
    /// The word the command line and the output use.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Skill => "skill",
            Self::Command => "command",
            Self::Rule => "rule",
            Self::Hook => "hook",
            Self::Mcp => "mcp",
            Self::Instruction => "instruction",
        }
    }
}

impl fmt::Display for ItemType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.word())
    }
}

/// The merged result has no item of that type and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSuchItem {
    item_type: ItemType,
    name: Stripped,
}

impl NoSuchItem {
    /// The error `explain` of this refusal (§10.3).
    #[must_use]
    pub fn into_diagnostic(self) -> Diagnostic {
        Diagnostic::error(
            Code::Explain,
            None,
            Message::new("no ")
                .then(self.item_type.word())
                .then(" named '")
                .then(self.name)
                .then("' in the merged result"),
        )
    }
}

impl fmt::Display for NoSuchItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "no {} named '{}' in the merged result",
            self.item_type, self.name
        )
    }
}

impl std::error::Error for NoSuchItem {}

/// One contribution: where it came from and what the layer did, both stripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedSource {
    source: Stripped,
    action: Stripped,
}

impl ExplainedSource {
    /// `<layer>@<rev>:<file>`, or `slot stub`.
    #[must_use]
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// The kind's word for what the layer did, such as `rule` or `extend: paths +[e2e/**]`.
    #[must_use]
    pub fn action(&self) -> &str {
        self.action.as_str()
    }

    fn of(origin: &Origin, action: &str) -> Self {
        Self {
            source: source_text(origin),
            action: Stripped::from_untrusted(action),
        }
    }

    fn slot_stub() -> Self {
        Self {
            source: Stripped::from_untrusted(SLOT_STUB_SOURCE),
            action: Stripped::from_untrusted(STUB_WORD),
        }
    }
}

/// The source as a header names it (§4.3). A revision or file that cannot stand in a header is
/// still shown, stripped, since explain describes the contributions that were overridden too.
fn source_text(origin: &Origin) -> Stripped {
    let text = SourceRef::of_origin(origin).map_or_else(
        |_| {
            let revision = origin
                .revision()
                .map_or_else(String::new, |revision| format!("@{}", revision.as_str()));
            format!("{}{revision}:{}", origin.layer(), origin.file().as_str())
        },
        |source| source.to_string(),
    );
    Stripped::from_untrusted(&text)
}

/// What `alz explain` says about one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    item_type: ItemType,
    name: Stripped,
    not_enabled: bool,
    sources: Vec<ExplainedSource>,
    outputs: Vec<OutputPath>,
}

impl Explanation {
    /// The kind of item.
    #[must_use]
    pub const fn item_type(&self) -> ItemType {
        self.item_type
    }

    /// The contributions, most general first.
    #[must_use]
    pub fn sources(&self) -> &[ExplainedSource] {
        &self.sources
    }

    /// Whether the item is a server that is defined and not enabled.
    #[must_use]
    pub const fn is_defined_but_not_enabled(&self) -> bool {
        self.not_enabled
    }

    /// The generated files that hold the item, which the caller prints when the build makes them.
    #[must_use]
    pub fn outputs(&self) -> &[OutputPath] {
        &self.outputs
    }
}

impl fmt::Display for Explanation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.not_enabled {
            writeln!(
                formatter,
                "mcp {} is defined but not enabled (add it to mcp.enable in layer.yaml)",
                self.name
            )?;
        }
        writeln!(formatter, "{} {}", self.item_type, self.name)?;
        writeln!(formatter, "sources, general → specific:")?;
        self.sources
            .iter()
            .enumerate()
            .try_for_each(|(index, source)| {
                writeln!(
                    formatter,
                    "  {}. {} — {}",
                    index + 1,
                    source.source,
                    source.action
                )
            })
    }
}

/// What a lookup found: the contributions and the files, before the name is attached.
struct Found {
    sources: Vec<ExplainedSource>,
    outputs: Vec<OutputPath>,
    not_enabled: bool,
}

impl Found {
    fn new(sources: Vec<ExplainedSource>, outputs: &[String]) -> Self {
        Self {
            sources,
            outputs: outputs
                .iter()
                .filter_map(|path| OutputPath::try_from(path.as_str()).ok())
                .collect(),
            not_enabled: false,
        }
    }
}

/// Explains the item of `item_type` called `name` in the merged model.
///
/// # Errors
///
/// [`NoSuchItem`] when the merged result has none.
pub fn explain(
    merged: &Merged,
    item_type: ItemType,
    name: &str,
) -> Result<Explanation, NoSuchItem> {
    find(merged, item_type, name)
        .map(|found| Explanation {
            item_type,
            name: Stripped::from_untrusted(name),
            not_enabled: found.not_enabled,
            sources: found.sources,
            outputs: found.outputs,
        })
        .ok_or_else(|| NoSuchItem {
            item_type,
            name: Stripped::from_untrusted(name),
        })
}

fn find(merged: &Merged, item_type: ItemType, name: &str) -> Option<Found> {
    match item_type {
        ItemType::Agent => merged
            .agents()
            .iter()
            .find(|(agent, _)| agent.as_str() == name)
            .map(|(agent, traced)| {
                Found::new(
                    sources_of(traced, "base", Overridable::Never),
                    &[format!(".claude/agents/{agent}.md")],
                )
            }),
        ItemType::Skill | ItemType::Command => skill_or_command(merged, item_type, name),
        ItemType::Rule => merged
            .rules()
            .iter()
            .find(|(rule, _)| rule.to_string() == name)
            .map(|(rule, traced)| {
                Found::new(
                    sources_of(traced, "rule", Overridable::ByReplace),
                    &[format!(".claude/rules/{rule}.md")],
                )
            }),
        ItemType::Hook => merged
            .hooks()
            .iter()
            .find(|traced| traced.value().id().as_str() == name)
            .map(|traced| {
                let word = format!("hook {}", traced.value().id().as_str());
                Found::new(
                    sources_of(traced, &word, Overridable::Never),
                    &[SETTINGS_PATH.to_owned()],
                )
            }),
        ItemType::Mcp => merged
            .mcp()
            .defined()
            .iter()
            .find(|(id, _)| id.as_str() == name)
            .map(|(id, traced)| {
                let word = format!("server {id}");
                Found {
                    not_enabled: !merged.mcp().enabled().contains(id),
                    ..Found::new(
                        sources_of(traced, &word, Overridable::ByReplace),
                        &[MCP_PATH.to_owned()],
                    )
                }
            }),
        ItemType::Instruction => instruction(merged, name),
    }
}

fn skill_or_command(merged: &Merged, item_type: ItemType, name: &str) -> Option<Found> {
    let defined = merged
        .skills()
        .iter()
        .find(|(skill, traced)| {
            skill.as_str() == name
                && matches!(
                    (item_type, traced.value()),
                    (ItemType::Skill, Skill::Directory(_)) | (ItemType::Command, Skill::Command(_))
                )
        })
        .map(|(skill, traced)| {
            let word = match traced.value() {
                Skill::Directory(_) => "skill",
                Skill::Command(_) => "command",
            };
            Found::new(
                sources_of(traced, word, Overridable::ByReplace),
                &[format!(".claude/skills/{skill}/SKILL.md")],
            )
        });
    defined.or_else(|| match item_type {
        ItemType::Skill => merged
            .slot_stubs()
            .keys()
            .find(|slot| slot.as_str() == name)
            .map(|slot| {
                Found::new(
                    vec![ExplainedSource::slot_stub()],
                    &[format!(".claude/skills/{slot}/SKILL.md")],
                )
            }),
        ItemType::Command
        | ItemType::Agent
        | ItemType::Rule
        | ItemType::Hook
        | ItemType::Mcp
        | ItemType::Instruction => None,
    })
}

/// The blocks whose file is called `name`, each as one contribution.
fn instruction(merged: &Merged, name: &str) -> Option<Found> {
    let sources: Vec<ExplainedSource> = merged
        .instructions()
        .iter()
        .filter(|block| {
            let file = block.value().file().as_str();
            file.strip_prefix(INSTRUCTIONS_DIR).unwrap_or(file) == name
        })
        .map(|block| ExplainedSource::of(block.value().origin(), BLOCK_WORD))
        .collect();
    (!sources.is_empty()).then(|| Found {
        sources,
        outputs: vec![agents_md_path()],
        not_enabled: false,
    })
}

/// Whether a contribution can be marked as overridden by a later one.
#[derive(Debug, Clone, Copy)]
enum Overridable {
    /// A later contribution that replaces the item marks every one before it.
    ByReplace,
    /// Never: an overlay refines an agent, and a hook stays what it was.
    Never,
}

/// The contributions of an item with the kind's word for a definition.
fn sources_of<T>(
    traced: &Traced<T>,
    definition: &str,
    overridable: Overridable,
) -> Vec<ExplainedSource> {
    let contributions = traced.provenance().contributions();
    contributions
        .iter()
        .enumerate()
        .map(|(index, contribution)| {
            let overridden = match overridable {
                Overridable::ByReplace => contributions
                    .iter()
                    .skip(index + 1)
                    .any(|later| matches!(later.action(), Action::Replace)),
                Overridable::Never => false,
            };
            let words = ActionWord::new(contribution.action(), definition).to_string();
            let action = if overridden {
                format!("{words}{OVERRIDDEN}")
            } else {
                words
            };
            ExplainedSource::of(contribution.origin(), &action)
        })
        .collect()
}

#[cfg(test)]
mod tests;
