//! The repository's stack manifest, `layer.yaml` with a `stack` (spec §5.2), as one value.

use crate::item::McpServerId;
use crate::merge::OrderedSet;
use crate::schema::{
    Checked, SchemaError, SchemaErrors, refine_list, text_list, unknown_fields, zip, zip3,
};
use crate::stack::budgets::Budgets;
use crate::stack::entry::DeclaredStack;
use crate::stack::workspace::Workspace;
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};
use alz_adapter::TargetId;

/// The fields a stack manifest knows.
const FIELDS: [&str; 7] = [
    "stack",
    "targets",
    "mcp",
    "workspace",
    "protected_paths",
    "budgets",
    "options",
];

macro_rules! free_text {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The text as written in the file.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(text: &str) -> Self {
                Self(text.to_owned())
            }
        }
    };
}

free_text! {
    /// An entry of `protected_paths`, a glob the build never touches (§4.5), kept as text.
    ProtectedPattern
}

/// The `options` of the manifest. Both are `true` when the manifest says nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    commit_generated: bool,
    strict_slots: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            commit_generated: true,
            strict_slots: true,
        }
    }
}

impl Options {
    /// Reads the `options` mapping at `node`.
    ///
    /// # Errors
    ///
    /// An unknown field, or a value that is not `true` or `false`.
    pub fn from_node(node: &Node, path: KeyPath) -> Checked<Self> {
        let section = Section::new(node, path)?;
        zip3(
            unknown_fields(&section, &["commit_generated", "strict_slots"]),
            section
                .boolean("commit_generated")
                .map_err(SchemaErrors::from),
            section.boolean("strict_slots").map_err(SchemaErrors::from),
        )
        .map(|((), commit_generated, strict_slots)| {
            let default = Self::default();
            Self {
                commit_generated: commit_generated.unwrap_or(default.commit_generated),
                strict_slots: strict_slots.unwrap_or(default.strict_slots),
            }
        })
    }

    /// Whether the generated files are committed (§8.3).
    #[must_use]
    pub const fn commit_generated(&self) -> bool {
        self.commit_generated
    }

    /// Whether a missing slot is an error and not a warning.
    #[must_use]
    pub const fn strict_slots(&self) -> bool {
        self.strict_slots
    }
}

/// The `mcp.enable` allowlist at `node`.
fn mcp_enable(node: &Node, path: KeyPath) -> Checked<Vec<McpServerId>> {
    let section = Section::new(node, path)?;
    zip(
        unknown_fields(&section, &["enable"]),
        refine_list::<McpServerId>(&section, "enable"),
    )
    .map(|((), ids)| ids.unwrap_or_default())
}

/// The field read by `read`, or `default` when the field is absent.
fn section_or<T>(
    section: &Section<'_>,
    field: &'static str,
    default: T,
    read: impl FnOnce(&Node, KeyPath) -> Checked<T>,
) -> Checked<T> {
    section
        .node(field)
        .map_or(Ok(default), |node| read(node, section.field_path(field)))
}

fn targets(section: &Section<'_>) -> Checked<OrderedSet<TargetId>> {
    refine_list::<TargetId>(section, "targets").map(|listed| {
        listed.map_or_else(
            || OrderedSet::from_iter([crate::target::claude().clone()]),
            |names| names.into_iter().collect(),
        )
    })
}

fn protected_paths(section: &Section<'_>) -> Checked<Vec<ProtectedPattern>> {
    text_list::<ProtectedPattern>(section, "protected_paths")
}

/// The stack manifest of a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    stack: DeclaredStack,
    targets: OrderedSet<TargetId>,
    workspace: Workspace,
    protected_paths: Vec<ProtectedPattern>,
    mcp_enable: Vec<McpServerId>,
    budgets: Budgets,
    options: Options,
}

impl Manifest {
    /// Reads a parsed `layer.yaml` of a repository.
    ///
    /// # Errors
    ///
    /// Every problem of the file at once, in document order, each naming its field by key path:
    /// an unknown field (as a quoted key), a missing, empty or malformed `stack`, a `ref` that is
    /// not a git ref (and not text, so `ref: 1.10` is refused), a source that is refused (a
    /// `git:` URL with credentials among them), a target that is not `claude` or `codex`, a
    /// workspace path that is empty or not relative, a budget that is not a whole number, an
    /// option that is not `true` or `false`. No error holds a value.
    pub fn from_yaml(root: &Node) -> Checked<Self> {
        let section = Section::new(root, KeyPath::root())
            .map_err(|error| SchemaErrors::from(SchemaError::NotAMapping(error.position())))?;
        zip3(
            zip(
                unknown_fields(&section, &FIELDS),
                DeclaredStack::from_section(&section),
            ),
            zip3(
                targets(&section),
                section_or(&section, "mcp", Vec::new(), mcp_enable),
                protected_paths(&section),
            ),
            zip3(
                section_or(
                    &section,
                    "workspace",
                    Workspace::default(),
                    Workspace::from_node,
                ),
                section_or(&section, "budgets", Budgets::default(), Budgets::from_node),
                section_or(&section, "options", Options::default(), Options::from_node),
            ),
        )
        .map(
            |(
                ((), stack),
                (targets, mcp_enable, protected_paths),
                (workspace, budgets, options),
            )| Self {
                stack,
                targets,
                workspace,
                protected_paths,
                mcp_enable,
                budgets,
                options,
            },
        )
    }

    /// The stack as declared.
    #[must_use]
    pub const fn stack(&self) -> &DeclaredStack {
        &self.stack
    }

    /// The targets to write for, each once, in the order the manifest declares them: `claude`
    /// when the manifest names none.
    #[must_use]
    pub const fn targets(&self) -> &OrderedSet<TargetId> {
        &self.targets
    }

    /// The hub and the additional directories.
    #[must_use]
    pub const fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// The extra paths the build never touches (§4.5), in the order written.
    #[must_use]
    pub fn protected_paths(&self) -> &[ProtectedPattern] {
        &self.protected_paths
    }

    /// The MCP servers that are on, in the order written.
    #[must_use]
    pub fn mcp_enable(&self) -> &[McpServerId] {
        &self.mcp_enable
    }

    /// The budgets.
    #[must_use]
    pub const fn budgets(&self) -> &Budgets {
        &self.budgets
    }

    /// The options.
    #[must_use]
    pub const fn options(&self) -> &Options {
        &self.options
    }
}
