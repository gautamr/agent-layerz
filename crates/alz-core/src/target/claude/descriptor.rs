//! The Claude adapter's descriptor (spec §8.0 **Descriptor.**, §8.1): its static data.
//!
//! The native vocabulary stays empty until the core takes the key rules from the descriptor
//! (TASK-110).

use alz_adapter::{Descriptor, MODEL_VERSION, Territory, ToolVersion, ToolVersionRange};

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
        crate::target::claude().clone(),
        MODEL_VERSION,
        ToolVersionRange::new(ToolVersion::new(2, 1, 311), ToolVersion::new(2, 2, 0)),
        FACTS_FILE,
    );
    descriptor.territory = Territory::new(owned(&["CLAUDE.md", ".mcp.json"]), owned(&[".claude"]));
    descriptor.managed_folders = owned(&MANAGED_FOLDERS);
    descriptor.protected_patterns = owned(&PROTECTED_PATTERNS);
    descriptor.adopted_files = owned(&ADOPTED_FILES);
    descriptor
}
