//! Why a YAML file is invalid, as a value (spec §7.13 "YAML limits" and "What else makes a YAML
//! file invalid"). An error names its kind and a position. It holds no input text, except the
//! quoted key of a duplicate, and it never holds a scalar, an anchor or alias name or a tag.

use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Code, Diagnostic, Message, Position, Where};
use crate::untrusted_text::QuotedKey;

/// The kind of problem. Each kind has a fixed message, so no kind can carry input text except
/// [`YamlErrorKind::DuplicateKey`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlErrorKind {
    /// The file is larger than 1 MiB.
    TooLarge,
    /// The file is not valid UTF-8.
    InvalidUtf8,
    /// A control character other than tab, line feed and carriage return in the file's text.
    ControlCharacter,
    /// A key or value that holds a control character other than tab and line feed once its
    /// escapes are applied.
    DecodedControlCharacter,
    /// Text that is not YAML.
    Syntax,
    /// More than 100 aliases, as they are written.
    TooManyAliases,
    /// Mappings and sequences nested more than 64 deep.
    TooDeep,
    /// More than 20000 nodes.
    TooManyNodes,
    /// More than 10000 events replayed through aliases.
    TooManyReplayedEvents,
    /// More than 4 MiB of scalar text, counting each use of an alias again.
    TooMuchScalarText,
    /// An explicit tag outside the YAML 1.2 core set.
    Tag,
    /// More than one document.
    MoreThanOneDocument,
    /// An alias inside the node its own anchor names.
    CyclicAlias,
    /// An alias whose anchor is not defined before it.
    UnresolvedAlias,
    /// Two equal keys in one mapping. The key is quoted when the parser can show it as text.
    DuplicateKey(Option<QuotedKey>),
    /// A mapping key that is not a scalar.
    NonScalarKey,
}

impl YamlErrorKind {
    /// The diagnostic code of the kind: `yaml-tag` for a tag, `yaml` for every other kind.
    #[must_use]
    pub const fn code(&self) -> Code {
        match self {
            Self::Tag => Code::YamlTag,
            Self::TooLarge
            | Self::InvalidUtf8
            | Self::ControlCharacter
            | Self::DecodedControlCharacter
            | Self::Syntax
            | Self::TooManyAliases
            | Self::TooDeep
            | Self::TooManyNodes
            | Self::TooManyReplayedEvents
            | Self::TooMuchScalarText
            | Self::MoreThanOneDocument
            | Self::CyclicAlias
            | Self::UnresolvedAlias
            | Self::DuplicateKey(_)
            | Self::NonScalarKey => Code::Yaml,
        }
    }

    /// The message of the kind, built from literals and the one quoted key.
    #[must_use]
    pub fn message(&self) -> Message {
        match self {
            Self::TooLarge => Message::new("the file is larger than 1 MiB"),
            Self::InvalidUtf8 => Message::new("the file is not valid UTF-8"),
            Self::ControlCharacter => Message::new("the file holds a control character"),
            Self::DecodedControlCharacter => Message::new(
                "a key or value holds a control character after its escapes are applied",
            ),
            Self::Syntax => Message::new("the file is not valid YAML"),
            Self::TooManyAliases => Message::new("more than 100 aliases"),
            Self::TooDeep => Message::new("mappings and sequences nested more than 64 deep"),
            Self::TooManyNodes => Message::new("more than 20000 nodes"),
            Self::TooManyReplayedEvents => {
                Message::new("more than 10000 events replayed through aliases")
            }
            Self::TooMuchScalarText => {
                Message::new("more than 4 MiB of scalar text, counting each alias use again")
            }
            Self::Tag => Message::new("a tag outside the YAML 1.2 core set"),
            Self::MoreThanOneDocument => Message::new("more than one document"),
            Self::CyclicAlias => Message::new("an alias inside its own anchor"),
            Self::UnresolvedAlias => Message::new("an alias whose anchor is not defined"),
            Self::DuplicateKey(Some(key)) => Message::new("duplicate key ").then(key.clone()),
            Self::DuplicateKey(None) => Message::new("duplicate key"),
            Self::NonScalarKey => Message::new("a mapping key that is not a scalar"),
        }
    }
}

impl fmt::Display for YamlErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message().fmt(formatter)
    }
}

/// A YAML file that is invalid: the kind of problem and, when it can be placed, where.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{kind}")]
pub struct YamlError {
    kind: YamlErrorKind,
    at: Option<Position>,
}

impl YamlError {
    /// An error of `kind` at `at`.
    #[must_use]
    pub const fn new(kind: YamlErrorKind, at: Option<Position>) -> Self {
        Self { kind, at }
    }

    /// The kind of problem.
    #[must_use]
    pub const fn kind(&self) -> &YamlErrorKind {
        &self.kind
    }

    /// Where the problem is, when it can be placed.
    #[must_use]
    pub const fn position(&self) -> Option<Position> {
        self.at
    }

    /// The error as a diagnostic in `file`, which is the place without a position: the error adds
    /// its own line and column.
    #[must_use]
    pub fn into_diagnostic(self, file: Where) -> Diagnostic {
        let at = self.at.map_or(file.clone(), |position| file.at(position));
        Diagnostic::error(self.kind.code(), Some(at), self.kind.message())
    }
}
