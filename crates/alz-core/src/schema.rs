//! Schema errors as values (spec §10.1 "Schema errors"). A schema error names a field by its key
//! path and says what is wrong with it. It never holds the field's value: the only input text it
//! can hold is a quoted key, in an unknown field. A file is checked as a whole, so
//! [`SchemaErrors`] holds every problem of the file, in the order they appear in it.

use thiserror::Error;

use crate::account::{AccountNameError, AccountShapeError};
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Position, Where};
use crate::identity::{EmailError, IdentityNameError};
use crate::item::{AgentRefusal, GlobError, InstructionModeError, RuleModeError};
use crate::layer::{KindError, LayerNameError, VisibilityError};
use crate::stack::{BudgetError, GitRefError, SourceError, StackError, WorkspaceDirError};
use crate::target::TargetError;
use crate::untrusted_text::Stripped;
use crate::yaml::Node;
use crate::yaml::fields::{FieldError, KeyPath, Section};

/// A value that is text but breaks the rule of its field. Each variant is the error of one
/// validated type, and none of them holds the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A layer name.
    LayerName(LayerNameError),
    /// A layer kind.
    Kind(KindError),
    /// A visibility.
    Visibility(VisibilityError),
    /// The name of a commit identity.
    IdentityName(IdentityNameError),
    /// The email of a commit identity.
    Email(EmailError),
    /// An account name.
    AccountName(AccountNameError),
    /// The shape of an `account` value.
    AccountShape(AccountShapeError),
    /// A git ref (`stack[n].ref`).
    GitRef(GitRefError),
    /// A stack entry's `source`.
    Source(SourceError),
    /// A target name.
    Target(TargetError),
    /// A workspace directory.
    WorkspaceDir(WorkspaceDirError),
    /// A budget.
    Budget(BudgetError),
    /// The stack as a whole.
    Stack(StackError),
    /// A glob of a rule's `paths`.
    Glob(GlobError),
    /// The `mode` of an instruction file.
    InstructionMode(InstructionModeError),
    /// The `mode` of a rule.
    RuleMode(RuleModeError),
    /// A field of an agent or an overlay.
    Agent(AgentRefusal),
}

impl Refusal {
    /// The rule that was broken, as the end of a sentence about the field: `must be …`.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::LayerName(error) => error.rule(),
            Self::Kind(error) => error.rule(),
            Self::Visibility(error) => error.rule(),
            Self::IdentityName(error) => error.rule(),
            Self::Email(error) => error.rule(),
            Self::AccountName(error) => error.rule(),
            Self::AccountShape(error) => error.rule(),
            Self::GitRef(error) => error.rule(),
            Self::Source(error) => error.rule(),
            Self::Target(error) => error.rule(),
            Self::WorkspaceDir(error) => error.rule(),
            Self::Budget(error) => error.rule(),
            Self::Stack(error) => error.rule(),
            Self::Glob(error) => error.rule(),
            Self::InstructionMode(error) => error.rule(),
            Self::RuleMode(error) => error.rule(),
            Self::Agent(error) => error.rule(),
        }
    }
}

macro_rules! refusal_from {
    ($($error:ident => $variant:ident),+ $(,)?) => {
        $(impl From<$error> for Refusal {
            fn from(error: $error) -> Self {
                Self::$variant(error)
            }
        })+
    };
}

refusal_from! {
    LayerNameError => LayerName,
    KindError => Kind,
    VisibilityError => Visibility,
    IdentityNameError => IdentityName,
    EmailError => Email,
    AccountNameError => AccountName,
    AccountShapeError => AccountShape,
    GitRefError => GitRef,
    SourceError => Source,
    TargetError => Target,
    WorkspaceDirError => WorkspaceDir,
    BudgetError => Budget,
    StackError => Stack,
    GlobError => Glob,
    InstructionModeError => InstructionMode,
    RuleModeError => RuleMode,
    AgentRefusal => Agent,
}

/// One problem of a file against its schema, with the position it is reported at.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SchemaError {
    /// The document is not a mapping.
    #[error("the document is not a mapping")]
    NotAMapping(Position),
    /// A field of the wrong type, or an unknown field.
    #[error("a field does not fit its schema")]
    Field(FieldError),
    /// A required field that is absent, reported at the mapping that should hold it.
    #[error("a required field is missing")]
    Missing {
        /// The key path of the field.
        path: KeyPath,
        /// Where the mapping that lacks it starts.
        at: Position,
    },
    /// A value of the right type that breaks the rule of its field.
    #[error("a field breaks its rule")]
    Refused {
        /// The key path of the field.
        path: KeyPath,
        /// The rule that was broken.
        refusal: Refusal,
        /// Where the value is.
        at: Position,
    },
    /// A `layer.yaml` with `stack` is a repository's stack manifest, not a layer (§5.1).
    #[error("the file is a stack manifest, not a layer")]
    StackManifest(Position),
}

impl SchemaError {
    /// A value at `at` that breaks the rule of the field at `path`.
    #[must_use]
    pub const fn refused(path: KeyPath, refusal: Refusal, at: Position) -> Self {
        Self::Refused { path, refusal, at }
    }

    /// A required field that is absent from the mapping at `at`.
    #[must_use]
    pub const fn missing(path: KeyPath, at: Position) -> Self {
        Self::Missing { path, at }
    }

    /// Where the problem is in the file.
    #[must_use]
    pub const fn position(&self) -> Position {
        match self {
            Self::Field(error) => error.position(),
            Self::NotAMapping(at)
            | Self::StackManifest(at)
            | Self::Missing { at, .. }
            | Self::Refused { at, .. } => *at,
        }
    }

    /// The message: the key path and what is wrong. It names no value.
    #[must_use]
    pub fn message(&self) -> Message {
        match self {
            Self::NotAMapping(_) => Message::new("the document must be a mapping"),
            Self::Field(error) => error.message(),
            Self::Missing { path, .. } => path.message().then(" is required"),
            Self::Refused {
                refusal: Refusal::Source(SourceError::Credentials),
                ..
            } => Message::new(SourceError::Credentials.rule()),
            Self::Refused { path, refusal, .. } => path.message().then(" ").then(refusal.rule()),
            Self::StackManifest(_) => Message::new(
                "this file has a 'stack' field, so it is a repository's stack manifest and not a layer",
            ),
        }
    }

    /// The error of the `layer.yaml` of `layer`: code `layer`, at `<layer>:layer.yaml`.
    #[must_use]
    pub fn diagnostic(&self, layer: Stripped) -> Diagnostic {
        Diagnostic::error(
            Code::Layer,
            Some(
                Where::layer_file(layer, Stripped::from_untrusted("layer.yaml"))
                    .at(self.position()),
            ),
            self.message(),
        )
    }
}

impl SchemaError {
    /// The error of the stack manifest `layer.yaml` of the repository: code `manifest`, at
    /// `layer.yaml` and the position. A `git:` source that holds credentials is the error
    /// `source` instead, with no location and a message that never prints the source (§5.2).
    #[must_use]
    pub fn manifest_diagnostic(&self) -> Diagnostic {
        match self {
            Self::Refused {
                refusal: Refusal::Source(SourceError::Credentials),
                ..
            } => Diagnostic::error(Code::Source, None, self.message()),
            Self::NotAMapping(_)
            | Self::Field(_)
            | Self::Missing { .. }
            | Self::Refused { .. }
            | Self::StackManifest(_) => Diagnostic::error(
                Code::Manifest,
                Some(
                    Where::repository_file(Stripped::from_untrusted("layer.yaml"))
                        .at(self.position()),
                ),
                self.message(),
            ),
        }
    }
}

impl From<FieldError> for SchemaError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

/// Every problem of one file: at least one, in document order.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("the file breaks its schema")]
pub struct SchemaErrors {
    first: SchemaError,
    rest: Vec<SchemaError>,
}

impl SchemaErrors {
    /// The problems of `errors`, in document order, or `None` when there is none.
    #[must_use]
    pub fn from_vec(errors: Vec<SchemaError>) -> Option<Self> {
        let mut all = errors.into_iter();
        all.next()
            .map(|first| all.fold(Self::from(first), Self::insert))
    }

    /// The problems of both, in document order. The problems of `self` come first among those at
    /// one position.
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        later.into_iter().fold(self, Self::insert)
    }

    /// Adds a problem after the ones at its position or before it. The errors stay sorted.
    fn insert(self, error: SchemaError) -> Self {
        if error.position() < self.first.position() {
            Self {
                first: error,
                rest: std::iter::once(self.first).chain(self.rest).collect(),
            }
        } else {
            let mut rest = self.rest;
            let place = rest.partition_point(|known| known.position() <= error.position());
            rest.insert(place, error);
            Self {
                first: self.first,
                rest,
            }
        }
    }

    /// The problems in document order.
    pub fn iter(&self) -> impl Iterator<Item = &SchemaError> + '_ {
        std::iter::once(&self.first).chain(&self.rest)
    }

    /// How many problems.
    #[must_use]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// Always false: the type holds at least one problem. It is here for the `len` convention.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// One `layer` error for each problem, at `<layer>:layer.yaml`.
    #[must_use]
    pub fn diagnostics(&self, layer: &Stripped) -> Diagnostics {
        self.iter()
            .map(|error| error.diagnostic(layer.clone()))
            .collect()
    }
}

impl SchemaErrors {
    /// One `manifest` error for each problem, at `layer.yaml` of the repository.
    #[must_use]
    pub fn manifest_diagnostics(&self) -> Diagnostics {
        self.iter().map(SchemaError::manifest_diagnostic).collect()
    }
}

impl IntoIterator for SchemaErrors {
    type Item = SchemaError;
    type IntoIter = std::iter::Chain<std::iter::Once<SchemaError>, std::vec::IntoIter<SchemaError>>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.first).chain(self.rest)
    }
}

impl From<SchemaError> for SchemaErrors {
    fn from(error: SchemaError) -> Self {
        Self {
            first: error,
            rest: Vec::new(),
        }
    }
}

impl From<FieldError> for SchemaErrors {
    fn from(error: FieldError) -> Self {
        SchemaError::from(error).into()
    }
}

/// A value, or every problem found while reading it.
pub type Checked<T> = Result<T, SchemaErrors>;

/// Both values, or the problems of both: unlike `?`, one failure does not hide the other.
///
/// # Errors
///
/// Every problem of `first` and `second`, in document order.
pub fn zip<A, B>(first: Checked<A>, second: Checked<B>) -> Checked<(A, B)> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok((first, second)),
        (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors),
        (Err(first), Err(second)) => Err(first.merge(second)),
    }
}

/// [`zip`] for three values.
///
/// # Errors
///
/// Every problem of the three, in document order.
pub fn zip3<A, B, C>(
    first: Checked<A>,
    second: Checked<B>,
    third: Checked<C>,
) -> Checked<(A, B, C)> {
    zip(zip(first, second), third).map(|((first, second), third)| (first, second, third))
}

/// The error for every key of the section that is not in `known`, as quoted keys.
///
/// # Errors
///
/// An [`SchemaError::Field`] for each unknown key.
pub fn unknown_fields(section: &Section<'_>, known: &[&str]) -> Checked<()> {
    SchemaErrors::from_vec(
        section
            .unknown_fields(known)
            .into_iter()
            .map(SchemaError::from)
            .collect(),
    )
    .map_or(Ok(()), Err)
}

/// Every value, or the problems of all that failed.
///
/// # Errors
///
/// Every problem of every item, in document order.
pub fn all<T>(items: impl IntoIterator<Item = Checked<T>>) -> Checked<Vec<T>> {
    let (values, problems) = items.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut values, mut problems), item| {
            match item {
                Ok(value) => values.push(value),
                Err(errors) => problems.extend(errors),
            }
            (values, problems)
        },
    );
    SchemaErrors::from_vec(problems).map_or(Ok(values), Err)
}

/// `node`, at `path`, as a value of the validated type `T`.
///
/// # Errors
///
/// The node's problem when it is not text, or the refusal of the type's `TryFrom<&str>`, at the
/// node. No error holds the text.
pub fn refine_node<T>(path: KeyPath, node: &Node) -> Checked<T>
where
    T: for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    let text = Section::text_of(node, path.clone())?;
    T::try_from(text).map_err(|error| SchemaError::refused(path, error.into(), node.at()).into())
}

/// The field as a list of validated values, `None` when it is absent. Every item is checked.
///
/// # Errors
///
/// The field's problem when it is not a list, and the problem of each item that is not text or
/// is refused, at `<field>[<n>]`.
pub fn refine_list<T>(section: &Section<'_>, field: &'static str) -> Checked<Option<Vec<T>>>
where
    T: for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    section
        .items(field)?
        .map(|items| {
            all(items
                .into_iter()
                .map(|(path, node)| refine_node(path, node)))
        })
        .transpose()
}

/// The field as a list of values made from text, empty when the field is absent. Every item that
/// is not text is reported, at `<field>[<n>]`.
///
/// # Errors
///
/// The field's problem when it is not a list, and the problem of each item that is not text.
pub fn text_list<T: for<'a> From<&'a str>>(
    section: &Section<'_>,
    field: &'static str,
) -> Checked<Vec<T>> {
    section.items(field)?.map_or(Ok(Vec::new()), |items| {
        all(items.into_iter().map(|(path, node)| {
            Section::text_of(node, path)
                .map(T::from)
                .map_err(SchemaErrors::from)
        }))
    })
}

/// The field as a value of the validated type `T`, `None` when it is absent.
///
/// # Errors
///
/// The field's problem when it is not text, or the refusal of the type's `TryFrom<&str>`, at the field's value. No
/// error holds the text.
pub fn refine<T>(section: &Section<'_>, field: &'static str) -> Checked<Option<T>>
where
    T: for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    let text = section.text(field)?;
    text.map(|text| {
        T::try_from(text).map_err(|error| {
            SchemaErrors::from(SchemaError::refused(
                section.field_path(field),
                error.into(),
                section.node(field).map_or(section.at(), Node::at),
            ))
        })
    })
    .transpose()
}

/// [`refine`] for a field the schema requires.
///
/// # Errors
///
/// [`SchemaError::Missing`], at the section, when the field is absent, and what [`refine`] says
/// otherwise.
pub fn require<T>(section: &Section<'_>, field: &'static str) -> Checked<T>
where
    T: for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    refine(section, field).and_then(|value| {
        value.ok_or_else(|| SchemaError::missing(section.field_path(field), section.at()).into())
    })
}

#[cfg(test)]
mod tests {
    use super::{Checked, SchemaError, SchemaErrors, zip, zip3};
    use crate::diagnostic::Position;

    fn at(line: u32) -> Position {
        Position::new(line, 1).expect("a position counted from 1")
    }

    fn not_a_mapping(line: u32) -> SchemaError {
        SchemaError::NotAMapping(at(line))
    }

    fn failed(lines: &[u32]) -> Checked<()> {
        Err(
            SchemaErrors::from_vec(lines.iter().copied().map(not_a_mapping).collect())
                .expect("at least one line"),
        )
    }

    fn lines(errors: &SchemaErrors) -> Vec<Position> {
        errors.iter().map(SchemaError::position).collect()
    }

    #[test]
    fn errors_are_kept_in_document_order_whatever_the_order_found() {
        let errors = SchemaErrors::from_vec(vec![
            not_a_mapping(5),
            not_a_mapping(2),
            not_a_mapping(9),
            not_a_mapping(2),
        ])
        .expect("there are errors");
        assert_eq!(lines(&errors), [at(2), at(2), at(5), at(9)]);
        assert_eq!(errors.len(), 4);
        assert!(SchemaErrors::from_vec(Vec::new()).is_none());
    }

    #[test]
    fn zip_keeps_the_problems_of_both_sides() {
        let both = zip(failed(&[7]), failed(&[3])).expect_err("both failed");
        assert_eq!(lines(&both), [at(3), at(7)]);
        assert!(zip(Ok(1), failed(&[3])).is_err());
        assert!(zip(failed(&[3]), Ok(1)).is_err());
        assert_eq!(zip(Ok(1), Ok(2)).expect("both fine"), (1, 2));
        assert_eq!(zip3(Ok(1), Ok(2), Ok(3)).expect("all fine"), (1, 2, 3));
        let all = zip3(failed(&[8]), Ok(()), failed(&[1])).expect_err("two failed");
        assert_eq!(lines(&all), [at(1), at(8)]);
    }

    #[test]
    fn merging_is_associative_and_keeps_every_problem() {
        let one = SchemaErrors::from(not_a_mapping(4));
        let two = SchemaErrors::from(not_a_mapping(1));
        let three = SchemaErrors::from(not_a_mapping(4));
        let left = one.clone().merge(two.clone()).merge(three.clone());
        let right = one.merge(two.merge(three));
        assert_eq!(left, right);
        assert_eq!(left.len(), 3);
    }
}
