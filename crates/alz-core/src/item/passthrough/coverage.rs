//! Which passthrough files the checks of §7.9 cover, by their path inside the target's folder.
//!
//! Paths compare with ASCII case folding only: a file system may fold more, so a path with a
//! non-ASCII character is refused instead (see the reader). The rules are the same for every
//! target, because an adapter writes whatever path a layer ships under its own target.

/// How a passthrough file is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Coverage {
    /// A Claude settings file, an `.mcp.json` or a plugin file: parsed as JSON.
    Json,
    /// A Claude agent file: its frontmatter `mcpServers` is checked.
    ClaudeAgent,
    /// A TOML file: refused by the text rule, since alz has no TOML parser.
    Toml,
    /// Nothing to check.
    Uncovered,
}

const JSON_SUFFIXES: [&str; 3] = [
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".mcp.json",
];
const PLUGIN_FOLDER: &str = ".claude-plugin";
const AGENTS_FOLDER: &str = ".claude/agents/";

impl Coverage {
    /// The coverage of `path`, the path after `targets/<target>/`, at any depth.
    pub(super) fn of(path: &str) -> Self {
        let key = path.to_ascii_lowercase();
        if JSON_SUFFIXES.iter().any(|suffix| ends_with(&key, suffix)) || is_plugin_json(&key) {
            Self::Json
        } else if is_agent_file(&key) {
            Self::ClaudeAgent
        } else if ends_with(&key, ".toml") {
            Self::Toml
        } else {
            Self::Uncovered
        }
    }
}

/// Whether `text` ends with `suffix`, ignoring ASCII case.
fn ends_with(text: &str, suffix: &str) -> bool {
    text.len()
        .checked_sub(suffix.len())
        .and_then(|start| text.get(start..))
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

/// `.claude-plugin/<name>.json`: a JSON file directly in a folder of that name, at any depth.
fn is_plugin_json(key: &str) -> bool {
    key.rsplit_once('/').is_some_and(|(folder, file)| {
        (folder == PLUGIN_FOLDER || folder.ends_with("/.claude-plugin"))
            && file.len() > ".json".len()
            && ends_with(file, ".json")
    })
}

/// `.claude/agents/<anything>.md` at any depth: the folder starts the path or follows a `/`.
fn is_agent_file(key: &str) -> bool {
    key.match_indices(AGENTS_FOLDER).any(|(start, folder)| {
        let at_boundary =
            start == 0 || key.get(..start).is_some_and(|before| before.ends_with('/'));
        let rest = key.get(start + folder.len()..).unwrap_or_default();
        at_boundary && rest.len() > ".md".len() && ends_with(rest, ".md")
    })
}

#[cfg(test)]
mod tests {
    use super::Coverage;

    #[test]
    fn json_files_are_covered_at_any_depth_with_any_case() {
        for path in [
            ".claude/settings.json",
            ".claude/settings.local.json",
            ".mcp.json",
            "sub/dir/.mcp.json",
            "sub/.claude/settings.json",
            ".Claude/Settings.JSON",
            "plugin/.claude-plugin/plugin.json",
            ".claude-plugin/marketplace.json",
        ] {
            assert_eq!(Coverage::of(path), Coverage::Json, "{path}");
        }
    }

    #[test]
    fn agent_files_are_covered_at_any_depth() {
        for path in [
            ".claude/agents/helper.md",
            ".claude/agents/team/helper.md",
            "x/.claude/agents/helper.md",
        ] {
            assert_eq!(Coverage::of(path), Coverage::ClaudeAgent, "{path}");
        }
    }

    #[test]
    fn toml_files_are_covered_whatever_their_name() {
        for path in [".codex/config.toml", "worker.toml", "deep/dir/X.TOML"] {
            assert_eq!(Coverage::of(path), Coverage::Toml, "{path}");
        }
    }

    #[test]
    fn other_files_and_near_misses_are_not_covered() {
        for path in [
            "notes.md",
            ".claude/agents/.md",
            "y.claude/agents/helper.md",
            ".claude/agents/helper.txt",
            ".claude-plugin/.json",
            "x.claude-plugin/plugin.json",
            ".claude-plugin/sub/plugin.json",
            ".claude/output-styles/x.md",
        ] {
            assert_eq!(Coverage::of(path), Coverage::Uncovered, "{path}");
        }
    }
}
