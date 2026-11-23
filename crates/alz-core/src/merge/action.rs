//! What a layer did to an item, as a typed value: the verb every kind shares, and the detail that
//! `alz explain` prints for the kinds that have more to say (spec §10.3). Rendering the words is
//! the reporter's; nothing here builds a sentence.

use crate::item::{AgentOverlay, Glob, OverlayMode, RuleExtension, WrittenEdit};
use crate::target::TargetName;

/// What a layer did to an item.
///
/// A kind of §7 that needs a finer verb adds a variant here; the exhaustive matches then show every
/// place that reports the action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// The layer defined the item.
    Define,
    /// The layer added to an item of a more general layer.
    Extend,
    /// The layer replaced the item, or a part of it, of a more general layer.
    Replace,
    /// The layer removed something from an item of a more general layer.
    Remove,
    /// An agent overlay (§7.2): its mode and the changes it made.
    Overlay {
        /// `extend` or `replace`.
        mode: OverlayMode,
        /// What the overlay changed, in the order `alz explain` lists it.
        changes: Vec<AgentChange>,
    },
    /// A rule file of `mode: extend` (§7.10).
    ExtendRule {
        /// The globs the file lists, as written, which are added to the rule's `paths`.
        paths: Vec<Glob>,
    },
}

impl Action {
    /// The action of an overlay on an agent of a more general layer.
    #[must_use]
    pub fn overlay(overlay: &AgentOverlay) -> Self {
        Self::Overlay {
            mode: overlay.mode(),
            changes: AgentChange::of(overlay),
        }
    }

    /// The action of a rule file that extends a rule.
    #[must_use]
    pub fn extend_rule(extension: &RuleExtension) -> Self {
        Self::ExtendRule {
            paths: extension.paths().iter().cloned().collect(),
        }
    }
}

/// A neutral scalar field of an agent that an overlay sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AgentScalar {
    /// `description`.
    Description,
    /// `role`.
    Role,
    /// `model`.
    Model,
    /// `isolation`.
    Isolation,
    /// `permission_mode`.
    PermissionMode,
    /// `memory`.
    Memory,
}

/// One change an overlay makes to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentChange {
    /// A neutral scalar field it sets.
    Scalar(AgentScalar),
    /// A target's native fields it sets.
    Native(TargetName),
    /// An edit of `skills`.
    Skills(WrittenEdit),
    /// An edit of `targets`.
    Targets(WrittenEdit),
    /// An edit of `tools`.
    Tools(WrittenEdit),
    /// `mode: replace`: its text is the whole body.
    BodyReplaced,
    /// `mode: extend` with text: the text is appended as a section.
    BodySection,
}

impl AgentChange {
    /// The changes `overlay` makes, in the order `alz explain` lists them: scalars, native fields,
    /// skills, targets, tools, then the body.
    #[must_use]
    pub fn of(overlay: &AgentOverlay) -> Vec<Self> {
        let scalars = overlay.scalars();
        let set = [
            (overlay.description().is_some(), AgentScalar::Description),
            (scalars.role().is_some(), AgentScalar::Role),
            (scalars.model().is_some(), AgentScalar::Model),
            (scalars.isolation().is_some(), AgentScalar::Isolation),
            (
                scalars.permission_mode().is_some(),
                AgentScalar::PermissionMode,
            ),
            (scalars.memory().is_some(), AgentScalar::Memory),
        ];
        let skills = overlay.skills_written().cloned().map(Self::Skills);
        let targets = overlay.targets_written().cloned().map(Self::Targets);
        let tools = overlay.tools_written().cloned().map(Self::Tools);
        let body = match overlay.mode() {
            OverlayMode::Replace => Some(Self::BodyReplaced),
            OverlayMode::Extend => (!overlay.body().is_empty()).then_some(Self::BodySection),
        };
        set.into_iter()
            .filter(|(is_set, _)| *is_set)
            .map(|(_, scalar)| Self::Scalar(scalar))
            .chain(
                overlay
                    .native()
                    .iter()
                    .map(|(target, _)| Self::Native(target)),
            )
            .chain(skills)
            .chain(targets)
            .chain(tools)
            .chain(body)
            .collect()
    }
}
