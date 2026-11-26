//! The Claude adapter's descriptor (spec §8.0 **Descriptor.**, §8.1): its static data.
//!
//! The native vocabulary holds the key rules of Claude's settings (TASK-110); the typed keys and
//! the native agent fields stay in the core until their own move.

use alz_adapter::{
    Descriptor, MODEL_VERSION, NativeVocabulary, TargetId, Territory, ToolVersion, ToolVersionRange,
};

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
    descriptor.vocabulary = NativeVocabulary::default().with_key_rules(
        key_rules::first_class_keys(),
        key_rules::reserved_keys(),
        key_rules::floor_keys(),
    );
    descriptor
}
