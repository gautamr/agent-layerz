//! The `layer.yaml` of a layer (spec §5.1), read from the YAML tree into one value.

use crate::account::AccountDeclaration;
use crate::identity::CommitIdentity;
use crate::layer::kind::Kind;
use crate::layer::name::LayerName;
use crate::layer::text::{Description, ForbidPatternText, LayerVersion, SlotName};
use crate::layer::visibility::Visibility;
use crate::schema::{Checked, SchemaError, SchemaErrors, require, unknown_fields, zip, zip3};
use crate::yaml::Node;
use crate::yaml::fields::{KeyPath, Section};

/// The fields a layer's `layer.yaml` knows.
const FIELDS: [&str; 10] = [
    "name",
    "kind",
    "visibility",
    "version",
    "description",
    "provides_slots",
    "requires_slots",
    "forbid_patterns",
    "commit_identity",
    "account",
];

/// What a layer says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerMeta {
    name: LayerName,
    kind: Kind,
    visibility: Visibility,
    version: Option<LayerVersion>,
    description: Option<Description>,
    provides_slots: Vec<SlotName>,
    requires_slots: Vec<SlotName>,
    forbid_patterns: Vec<ForbidPatternText>,
    commit_identity: Option<CommitIdentity>,
    account: Option<AccountDeclaration>,
}

/// The text at `field` as a free-text type, `None` when the field is absent.
fn free_text<T: for<'a> From<&'a str>>(
    section: &Section<'_>,
    field: &'static str,
) -> Checked<Option<T>> {
    section
        .text(field)
        .map(|text| text.map(T::from))
        .map_err(SchemaErrors::from)
}

/// The text list at `field`, empty when the field is absent.
fn text_list<T: for<'a> From<&'a str>>(
    section: &Section<'_>,
    field: &'static str,
) -> Checked<Vec<T>> {
    section
        .text_list(field)
        .map(|items| items.unwrap_or_default().into_iter().map(T::from).collect())
        .map_err(SchemaErrors::from)
}

/// The field read by `read`, `None` when it is absent.
fn optional<T>(
    section: &Section<'_>,
    field: &'static str,
    read: impl FnOnce(&Node, KeyPath) -> Checked<T>,
) -> Checked<Option<T>> {
    section
        .node(field)
        .map(|node| read(node, section.field_path(field)))
        .transpose()
}

/// The three fields every layer must have.
fn header(section: &Section<'_>) -> Checked<(LayerName, Kind, Visibility)> {
    zip3(
        require::<LayerName>(section, "name"),
        require::<Kind>(section, "kind"),
        require::<Visibility>(section, "visibility"),
    )
}

/// The optional text fields and the three lists.
type Described = (
    (Option<LayerVersion>, Option<Description>),
    (Vec<SlotName>, Vec<SlotName>, Vec<ForbidPatternText>),
);

fn described(section: &Section<'_>) -> Checked<Described> {
    zip(
        zip(
            free_text(section, "version"),
            free_text(section, "description"),
        ),
        zip3(
            text_list(section, "provides_slots"),
            text_list(section, "requires_slots"),
            text_list(section, "forbid_patterns"),
        ),
    )
}

/// The two declarations that only some layers may make (the merge decides who).
fn declarations(
    section: &Section<'_>,
) -> Checked<(Option<CommitIdentity>, Option<AccountDeclaration>)> {
    zip(
        optional(section, "commit_identity", CommitIdentity::from_node),
        optional(section, "account", AccountDeclaration::from_node),
    )
}

impl LayerMeta {
    /// Reads a parsed `layer.yaml`.
    ///
    /// # Errors
    ///
    /// Every problem of the file at once, in document order: an unknown field (as a quoted key),
    /// a required field that is missing, a number where text is expected (with the hint to quote
    /// it), a value that breaks the rule of its field. A file with a `stack` field is a stack
    /// manifest and is refused for that alone. No error holds a value.
    pub fn from_yaml(root: &Node) -> Checked<Self> {
        let section = Section::new(root, KeyPath::root())
            .map_err(|error| SchemaErrors::from(SchemaError::NotAMapping(error.position())))?;
        if let Some(at) = section.key_position("stack") {
            return Err(SchemaError::StackManifest(at).into());
        }
        zip3(
            zip(unknown_fields(&section, &FIELDS), header(&section)),
            described(&section),
            declarations(&section),
        )
        .map(
            |(
                ((), (name, kind, visibility)),
                ((version, description), (provides_slots, requires_slots, forbid_patterns)),
                (commit_identity, account),
            )| Self {
                name,
                kind,
                visibility,
                version,
                description,
                provides_slots,
                requires_slots,
                forbid_patterns,
                commit_identity,
                account,
            },
        )
    }

    /// The layer's name.
    #[must_use]
    pub const fn name(&self) -> &LayerName {
        &self.name
    }

    /// The layer's kind.
    #[must_use]
    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    /// The layer's visibility.
    #[must_use]
    pub const fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// The layer's version, if it gives one.
    #[must_use]
    pub const fn version(&self) -> Option<&LayerVersion> {
        self.version.as_ref()
    }

    /// The layer's description, if it gives one.
    #[must_use]
    pub const fn description(&self) -> Option<&Description> {
        self.description.as_ref()
    }

    /// The skills the layer provides, in the order written.
    #[must_use]
    pub fn provides_slots(&self) -> &[SlotName] {
        &self.provides_slots
    }

    /// The skills the layer expects others to provide, in the order written.
    #[must_use]
    pub fn requires_slots(&self) -> &[SlotName] {
        &self.requires_slots
    }

    /// The leak-guard patterns, in the order written.
    #[must_use]
    pub fn forbid_patterns(&self) -> &[ForbidPatternText] {
        &self.forbid_patterns
    }

    /// The commit identity the layer declares, if any.
    #[must_use]
    pub const fn commit_identity(&self) -> Option<&CommitIdentity> {
        self.commit_identity.as_ref()
    }

    /// The account the layer declares, if any.
    #[must_use]
    pub const fn account(&self) -> Option<&AccountDeclaration> {
        self.account.as_ref()
    }
}

#[cfg(test)]
mod tests;
