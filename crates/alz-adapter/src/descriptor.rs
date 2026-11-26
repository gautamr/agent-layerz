//! The descriptor of an adapter (spec §8.0 **Descriptor.**): static data about the adapter and the
//! tool it writes for. Spec §8.1 gives the Claude adapter's values.

use std::fmt;

use crate::model::ModelVersion;
use crate::native_type::TypedKeys;
use crate::target_id::TargetId;

/// The version of a tool, `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolVersion {
    /// The major version.
    pub major: u32,
    /// The minor version.
    pub minor: u32,
    /// The patch version.
    pub patch: u32,
}

impl ToolVersion {
    /// The version `major.minor.patch`.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The verified range of the tool's versions, which `alz doctor` checks: `from` up to, and not
/// including, `below`. Claude Code's is `>=2.1.311 <2.2.0` (§8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolVersionRange {
    /// The earliest verified version.
    pub from: ToolVersion,
    /// The first version that is not verified.
    pub below: ToolVersion,
}

impl ToolVersionRange {
    /// The range `>=from <below`.
    #[must_use]
    pub const fn new(from: ToolVersion, below: ToolVersion) -> Self {
        Self { from, below }
    }

    /// Whether `version` is in the range.
    #[must_use]
    pub fn contains(&self, version: ToolVersion) -> bool {
        self.from <= version && version < self.below
    }
}

impl fmt::Display for ToolVersionRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, ">={} <{}", self.from, self.below)
    }
}

/// The files and folders an adapter may write (§8.0 **Output checks.**). A path is in the
/// territory when it is one of the files or lies below one of the folders. Claude's is `CLAUDE.md`,
/// `.mcp.json` and the folder `.claude/` (§8.1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Territory {
    /// The files, as paths relative to the repository.
    pub files: Vec<String>,
    /// The folders, as paths relative to the repository.
    pub folders: Vec<String>,
}

impl Territory {
    /// A territory of `files` and `folders`.
    #[must_use]
    pub const fn new(files: Vec<String>, folders: Vec<String>) -> Self {
        Self { files, folders }
    }
}

/// A first-class key (§7.9): refused in `settings.yaml`, because another file sets it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FirstClassKey {
    /// The key path.
    pub path: String,
    /// The file that sets it instead, as `permissions.yaml`.
    pub set_through: String,
}

impl FirstClassKey {
    /// A first-class key.
    #[must_use]
    pub fn new(path: impl Into<String>, set_through: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            set_through: set_through.into(),
        }
    }
}

/// A reserved key (§7.9): refused in `settings.yaml`, with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReservedKey {
    /// The key path.
    pub path: String,
    /// Why it is not set there.
    pub reason: String,
}

impl ReservedKey {
    /// A reserved key.
    #[must_use]
    pub fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

/// How a floor key may change (§7.9 **Floor keys**): only to a stricter value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorRule {
    /// A boolean whose stricter value is `true`.
    TrueIsStricter,
    /// A boolean whose stricter value is `false`.
    FalseIsStricter,
    /// Ordered text: the ranks from the loosest to the strictest, each rank the values that are
    /// equally strict. A value keeps its rank or moves to a stricter one, and a value that is in
    /// no rank may not change.
    Ranked(Vec<Vec<String>>),
    /// A list or mapping that gains no entry (list item or mapping key, at any depth) that a more
    /// general layer did not set.
    NoAdditions,
    /// A list that may only grow.
    Union,
}

/// A floor key (§7.9): once a layer sets it, a more specific layer may only make it stricter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FloorKey {
    /// The key path.
    pub path: String,
    /// How it may change.
    pub rule: FloorRule,
}

impl FloorKey {
    /// A floor key.
    #[must_use]
    pub fn new(path: impl Into<String>, rule: FloorRule) -> Self {
        Self {
            path: path.into(),
            rule,
        }
    }
}

/// The native vocabulary of a target (§8.0 **Descriptor.**). The core merges a target's native
/// settings with these key rules, so the security floor (§7.6, §7.9, §9.3) stays the core's.
///
/// The rows of one key path are looked up in the order first-class, floor, reserved, and the first
/// row whose path matches is the one that applies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NativeVocabulary {
    /// The settings keys with their types (§7.9 **Typed keys**). `None` when the target's settings
    /// keys are not typed: its section is checked by the key rules below and nothing else.
    pub settings_keys: Option<TypedKeys>,
    /// The first-class keys, in the order §7.9 gives them.
    pub first_class_keys: Vec<FirstClassKey>,
    /// The reserved keys, in the order §7.9 gives them.
    pub reserved_keys: Vec<ReservedKey>,
    /// The floor keys, in the order §7.9 gives them.
    pub floor_keys: Vec<FloorKey>,
    /// The native fields of an agent with their types (§7.2). `None` when they are not typed.
    pub agent_fields: Option<TypedKeys>,
    /// The keys of the target's agent map that have a neutral field, and so are refused inside it
    /// (§7.2).
    pub agent_neutral_keys: Vec<String>,
    /// The keys of the target's agent map that duplicate a first-class item, each with the file
    /// that sets it instead (§7.2): servers and hooks.
    pub agent_first_class_keys: Vec<FirstClassKey>,
    /// The target's own hook events, as `Notification`, for `<target>:<Event>` (§7.5).
    pub hook_events: Vec<String>,
}

impl NativeVocabulary {
    /// The vocabulary with the key rules of §7.9: first-class, reserved and floor keys, each list
    /// in the order the spec gives it.
    #[must_use]
    pub fn with_key_rules(
        self,
        first_class_keys: Vec<FirstClassKey>,
        reserved_keys: Vec<ReservedKey>,
        floor_keys: Vec<FloorKey>,
    ) -> Self {
        Self {
            first_class_keys,
            reserved_keys,
            floor_keys,
            ..self
        }
    }

    /// The vocabulary with the typed settings keys of §7.9.
    #[must_use]
    pub fn with_settings_keys(self, settings_keys: TypedKeys) -> Self {
        Self {
            settings_keys: Some(settings_keys),
            ..self
        }
    }

    /// The vocabulary with the native agent fields of §7.2: the neutral keys, the first-class keys
    /// and, when the target types them, the typed fields.
    #[must_use]
    pub fn with_agent_native(
        self,
        agent_neutral_keys: Vec<String>,
        agent_first_class_keys: Vec<FirstClassKey>,
        agent_fields: Option<TypedKeys>,
    ) -> Self {
        Self {
            agent_fields,
            agent_neutral_keys,
            agent_first_class_keys,
            ..self
        }
    }
}

/// What an adapter states, as static data (§8.0 **Descriptor.**).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Descriptor {
    /// The adapter's target id (§5.2).
    pub target: TargetId,
    /// The version of the model the adapter is built for.
    pub model_version: ModelVersion,
    /// The verified range of the tool's versions.
    pub tool_versions: ToolVersionRange,
    /// The files and folders the adapter may write.
    pub territory: Territory,
    /// The managed folders (§4.3).
    pub managed_folders: Vec<String>,
    /// The protected patterns the adapter adds (§4.5).
    pub protected_patterns: Vec<String>,
    /// The files the adapter always adopts (§4.3).
    pub adopted_files: Vec<String>,
    /// The native vocabulary.
    pub vocabulary: NativeVocabulary,
    /// The facts file that records the tool's facts the adapter relies on (§8 **What an adapter
    /// may rely on**), as a path relative to the repository.
    pub facts_file: String,
}

impl Descriptor {
    /// A descriptor with an empty territory and vocabulary and no managed folders, protected
    /// patterns or adopted files, which the caller then fills.
    #[must_use]
    pub fn new(
        target: TargetId,
        model_version: ModelVersion,
        tool_versions: ToolVersionRange,
        facts_file: impl Into<String>,
    ) -> Self {
        Self {
            target,
            model_version,
            tool_versions,
            territory: Territory::default(),
            managed_folders: Vec::new(),
            protected_patterns: Vec::new(),
            adopted_files: Vec::new(),
            vocabulary: NativeVocabulary::default(),
            facts_file: facts_file.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolVersion, ToolVersionRange};

    #[test]
    fn a_range_includes_its_start_and_excludes_its_end() {
        let range = ToolVersionRange::new(ToolVersion::new(2, 1, 311), ToolVersion::new(2, 2, 0));
        assert!(range.contains(ToolVersion::new(2, 1, 311)));
        assert!(range.contains(ToolVersion::new(2, 1, 999)));
        assert!(!range.contains(ToolVersion::new(2, 1, 310)));
        assert!(!range.contains(ToolVersion::new(2, 2, 0)));
        assert_eq!(range.to_string(), ">=2.1.311 <2.2.0");
    }
}
