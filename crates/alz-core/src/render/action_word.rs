//! The words `alz explain` prints for what a layer did to an item (spec §10.3), made from the typed
//! [`Action`] of a contribution. The kind's own word for a definition, such as `rule`, `skill` or
//! `base`, is given by the caller, which knows the kind.

use std::fmt;

use crate::item::{EditSign, WrittenEdit};
use crate::merge::{Action, AgentChange, AgentScalar};

/// The words of one contribution: `definition` for an action that defines or replaces the item,
/// or the words of the action itself.
#[derive(Debug, Clone, Copy)]
pub struct ActionWord<'a> {
    action: &'a Action,
    definition: &'a str,
}

impl<'a> ActionWord<'a> {
    /// The words of `action`, where `definition` is the kind's word for a definition.
    #[must_use]
    pub const fn new(action: &'a Action, definition: &'a str) -> Self {
        Self { action, definition }
    }
}

impl fmt::Display for ActionWord<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.action {
            Action::Define | Action::Replace => formatter.write_str(self.definition),
            Action::Extend => formatter.write_str("extend"),
            Action::Remove => formatter.write_str("remove"),
            Action::Overlay { mode, changes } => {
                let words = changes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(
                    formatter,
                    "{}: {}",
                    mode_word(*mode),
                    if words.is_empty() {
                        "no changes"
                    } else {
                        &words
                    }
                )
            }
            Action::ExtendRule { paths } => write!(
                formatter,
                "extend: paths +[{}]",
                paths
                    .iter()
                    .map(crate::item::Glob::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

const fn mode_word(mode: crate::item::OverlayMode) -> &'static str {
    match mode {
        crate::item::OverlayMode::Extend => "extend",
        crate::item::OverlayMode::Replace => "replace",
    }
}

impl fmt::Display for AgentChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(scalar) => formatter.write_str(scalar_word(*scalar)),
            Self::Native(target) => write!(formatter, "{target} fields"),
            Self::Skills(change) => write!(formatter, "skills {change}"),
            Self::Targets(change) => write!(formatter, "targets {change}"),
            Self::Tools(change) => write!(formatter, "tools {change}"),
            Self::BodyReplaced => formatter.write_str("body replaced"),
            Self::BodySection => formatter.write_str("body section"),
        }
    }
}

const fn scalar_word(scalar: AgentScalar) -> &'static str {
    match scalar {
        AgentScalar::Description => "description",
        AgentScalar::Role => "role",
        AgentScalar::Model => "model",
        AgentScalar::Isolation => "isolation",
        AgentScalar::PermissionMode => "permission_mode",
        AgentScalar::Memory => "memory",
    }
}

/// The segments as the layer wrote them, joined by a space: `+[a] -[b]`, `[a, b]`. An edit written
/// as a string without a segment reads as nothing, as the reference prints it.
impl fmt::Display for WrittenEdit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words = self
            .segments()
            .iter()
            .map(|segment| {
                let sign = match segment.sign() {
                    EditSign::Add => "+",
                    EditSign::Remove => "-",
                    EditSign::Replace => "",
                };
                format!("{sign}[{}]", segment.items().join(", "))
            })
            .collect::<Vec<_>>()
            .join(" ");
        formatter.write_str(&words)
    }
}
