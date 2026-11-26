//! The descriptor of an adapter (spec §8.0 **Descriptor.**): static data about the adapter and the
//! tool it writes for. Spec §8.1 gives the Claude adapter's values.

use std::fmt;

use crate::model::ModelVersion;
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

/// The type of a native settings key's value (§7.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// Text.
    Text,
    /// `true` or `false`.
    Boolean,
    /// A whole number.
    WholeNumber,
    /// Any number.
    Number,
    /// A list.
    List,
    /// A mapping.
    Mapping,
}

/// A native settings key with its type, as a key path (§7.9 **Key paths.**).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SettingsKey {
    /// The key path, such as `sandbox.enabled` or `**.features.hooks`.
    pub path: String,
    /// The type of the value.
    pub kind: ValueKind,
}

impl SettingsKey {
    /// A key.
    #[must_use]
    pub fn new(path: impl Into<String>, kind: ValueKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
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
    /// Ordered text: the ranks from the loosest to the strictest. A value keeps its rank or moves
    /// to a stricter one, and a value that is in no rank may not change.
    Ranked(Vec<String>),
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

/// A native field of an agent (§7.2) with its type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AgentField {
    /// The field's name, as `maxTurns`.
    pub name: String,
    /// The type of its value.
    pub kind: ValueKind,
}

impl AgentField {
    /// A native agent field.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: ValueKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

/// The native vocabulary of a target (§8.0 **Descriptor.**). The core merges a target's native
/// settings with these key rules, so the security floor (§7.6, §7.9, §9.3) stays the core's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NativeVocabulary {
    /// The settings keys with their types.
    pub settings_keys: Vec<SettingsKey>,
    /// The first-class keys, in the order §7.9 gives them.
    pub first_class_keys: Vec<FirstClassKey>,
    /// The reserved keys, in the order §7.9 gives them.
    pub reserved_keys: Vec<ReservedKey>,
    /// The floor keys, in the order §7.9 gives them.
    pub floor_keys: Vec<FloorKey>,
    /// The native fields of an agent, in the order §7.2 gives them.
    pub agent_fields: Vec<AgentField>,
    /// The target's own hook events, as `Notification`, for `<target>:<Event>` (§7.5).
    pub hook_events: Vec<String>,
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
