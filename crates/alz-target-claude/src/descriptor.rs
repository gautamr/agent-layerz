//! The Claude adapter's descriptor (spec §8.0 **Descriptor.**, §8.1): its static data.
//!
//! The native vocabulary holds the key rules of Claude's settings (TASK-110), the typed settings
//! keys and the native agent fields (TASK-128).

use alz_adapter::{
    Descriptor, FirstClassKey, MODEL_VERSION, NativeVocabulary, TargetId, Territory, ToolVersion,
    ToolVersionRange, TypedKeys,
};

use super::typed_keys::{AGENT_FIELDS, SETTINGS_KEYS};
use super::{TARGET, key_rules};

/// The facts file that records what the adapter relies on.
const FACTS_FILE: &str = "docs/target-facts.md";

/// The folders the adapter owns, where a file the build does not make is a collision (§4.3).
const MANAGED_FOLDERS: [&str; 4] = [
    ".claude/agents",
    ".claude/skills",
    ".claude/rules",
    ".claude/hooks",
];

/// The folders where Claude keeps per-agent memory (target-facts §9): repository-owned lessons
/// that the build never generates (§4.5).
const PROTECTED_PATTERNS: [&str; 2] = [".claude/agent-memory/**", ".claude/agent-memory-local/**"];

/// The files that adoption always takes, when they exist (§4.3).
const ADOPTED_FILES: [&str; 2] = [".claude/settings.json", ".mcp.json"];

/// Where the typed rows are recorded (target-facts section 14).
const RECORDED_IN: &str = "docs/target-facts.md section 14";

/// The keys of a subagent's `claude:` map that have a neutral field, and so are refused inside it
/// (§7.2).
const AGENT_NEUTRAL_KEYS: [&str; 8] = [
    "name",
    "description",
    "tools",
    "model",
    "permissionMode",
    "skills",
    "isolation",
    "memory",
];

/// The keys of a subagent's `claude:` map that bring servers and hooks past `mcp.yaml` and
/// `hooks.yaml`, each with the file that sets it (§7.2).
fn agent_first_class_keys() -> Vec<FirstClassKey> {
    vec![
        FirstClassKey::new("mcpServers", "mcp.yaml"),
        FirstClassKey::new("hooks", "hooks/hooks.yaml"),
    ]
}

fn owned(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

/// The descriptor of the Claude adapter.
pub(super) fn descriptor() -> Descriptor {
    let mut descriptor = Descriptor::new(
        TargetId::try_from(TARGET).expect("the literal `claude` is a well-formed target id"),
        MODEL_VERSION,
        ToolVersionRange::new(ToolVersion::new(2, 1, 311), ToolVersion::new(2, 2, 0)),
        FACTS_FILE,
    );
    descriptor.territory = Territory::new(owned(&["CLAUDE.md", ".mcp.json"]), owned(&[".claude"]));
    descriptor.managed_folders = owned(&MANAGED_FOLDERS);
    descriptor.protected_patterns = owned(&PROTECTED_PATTERNS);
    descriptor.adopted_files = owned(&ADOPTED_FILES);
    descriptor.vocabulary = NativeVocabulary::default()
        .with_key_rules(
            key_rules::first_class_keys(),
            key_rules::reserved_keys(),
            key_rules::floor_keys(),
        )
        .with_settings_keys(TypedKeys::new(SETTINGS_KEYS, "Claude Code", RECORDED_IN))
        .with_agent_native(
            owned(&AGENT_NEUTRAL_KEYS),
            agent_first_class_keys(),
            Some(TypedKeys::new(AGENT_FIELDS, "Claude Code", RECORDED_IN)),
        );
    descriptor
}
