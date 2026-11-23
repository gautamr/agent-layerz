//! What a path of a layer is for (spec §4.1): the kind of item each file belongs to.
//!
//! [`ItemPath::of`] classifies by shape alone. It does not judge a name: whether `agents/x y.md`
//! names a valid agent is for the reader of that kind. A path that is none of the layouts of §4.1
//! is [`ItemPath::Other`], so every path has exactly one variant.

use crate::path::SafeRelPath;
use crate::target::TargetName;

/// The file `targets/claude/CLAUDE.extra.md` (§7.1).
const CLAUDE_EXTRA: &str = "targets/claude/CLAUDE.extra.md";

/// The kind of item a path of a layer is, with the parts of the path that name it. The text
/// borrows from the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPath<'a> {
    /// `layer.yaml`: the layer's own description (§5.1).
    LayerYaml,
    /// `instructions/<name>.md`, directly inside the folder.
    Instruction {
        /// The file name without `.md`.
        name: &'a str,
    },
    /// `agents/<name>.md`: a full agent definition, the base.
    Agent {
        /// The file name without `.md`.
        name: &'a str,
    },
    /// `agents/<name>.overlay.md`: an overlay on an agent of a more general layer.
    AgentOverlay {
        /// The file name without `.overlay.md`.
        name: &'a str,
    },
    /// `skills/<name>/<file>`: a file of a skill, `SKILL.md` or anything below its folder.
    SkillFile {
        /// The skill's folder name.
        skill: &'a str,
        /// The path inside the skill's folder, `/`-separated.
        file: &'a str,
    },
    /// `commands/<name>.md`, directly inside the folder.
    Command {
        /// The file name without `.md`.
        name: &'a str,
    },
    /// `rules/<name>.md`, directly inside the folder.
    Rule {
        /// The file name without `.md`.
        name: &'a str,
    },
    /// `hooks/hooks.yaml`.
    Hooks,
    /// `hooks/scripts/<path>`: a hook script, which hooks name relatively.
    HookScript {
        /// The path inside `hooks/scripts`, `/`-separated.
        script: &'a str,
    },
    /// `permissions.yaml`.
    Permissions,
    /// `mcp.yaml`.
    Mcp,
    /// `models.yaml`.
    Models,
    /// `settings.yaml`.
    Settings,
    /// `targets/<target>/<path>`, except `CLAUDE.extra.md`: a raw file for a known target.
    Passthrough {
        /// The target the file is for.
        target: TargetName,
        /// The path inside the target's folder, `/`-separated.
        path: &'a str,
    },
    /// `targets/claude/CLAUDE.extra.md`: Claude-specific text for `CLAUDE.md`.
    ClaudeExtra,
    /// Any other path: a README, a folder of notes, a file for a target alz does not know.
    Other,
}

impl<'a> ItemPath<'a> {
    /// The kind of item `path` is.
    #[must_use]
    pub fn of(path: &'a SafeRelPath) -> Self {
        let text = path.as_str();
        match text {
            "layer.yaml" => Self::LayerYaml,
            "hooks/hooks.yaml" => Self::Hooks,
            "permissions.yaml" => Self::Permissions,
            "mcp.yaml" => Self::Mcp,
            "models.yaml" => Self::Models,
            "settings.yaml" => Self::Settings,
            CLAUDE_EXTRA => Self::ClaudeExtra,
            _ => text
                .split_once('/')
                .map_or(Self::Other, |(folder, rest)| in_folder(folder, rest)),
        }
    }
}

/// A path below a top-level folder.
fn in_folder<'a>(folder: &str, rest: &'a str) -> ItemPath<'a> {
    match folder {
        "instructions" => markdown_file(rest, ".md")
            .map_or(ItemPath::Other, |name| ItemPath::Instruction { name }),
        "rules" => {
            markdown_file(rest, ".md").map_or(ItemPath::Other, |name| ItemPath::Rule { name })
        }
        "commands" => {
            markdown_file(rest, ".md").map_or(ItemPath::Other, |name| ItemPath::Command { name })
        }
        "agents" => agent(rest),
        "skills" => rest
            .split_once('/')
            .filter(|(skill, file)| !skill.is_empty() && !file.is_empty())
            .map_or(ItemPath::Other, |(skill, file)| ItemPath::SkillFile {
                skill,
                file,
            }),
        "hooks" => rest
            .strip_prefix("scripts/")
            .filter(|script| !script.is_empty())
            .map_or(ItemPath::Other, |script| ItemPath::HookScript { script }),
        "targets" => rest
            .split_once('/')
            .and_then(|(target, path)| {
                TargetName::try_from(target)
                    .ok()
                    .map(|target| ItemPath::Passthrough { target, path })
            })
            .unwrap_or(ItemPath::Other),
        _ => ItemPath::Other,
    }
}

/// The name of a file directly inside its folder that ends in `suffix`, when the name is not
/// empty.
fn markdown_file<'a>(rest: &'a str, suffix: &str) -> Option<&'a str> {
    (!rest.contains('/'))
        .then(|| rest.strip_suffix(suffix))
        .flatten()
        .filter(|name| !name.is_empty())
}

/// A file of `agents/`: an overlay when it ends in `.overlay.md`, a base when it ends in `.md`.
fn agent(rest: &str) -> ItemPath<'_> {
    if rest.ends_with(".overlay.md") {
        markdown_file(rest, ".overlay.md")
            .map_or(ItemPath::Other, |name| ItemPath::AgentOverlay { name })
    } else {
        markdown_file(rest, ".md").map_or(ItemPath::Other, |name| ItemPath::Agent { name })
    }
}

#[cfg(test)]
mod tests;
