//! What stops `alz init` before it writes anything.

use std::fmt;

use crate::stack::SourceError;
use crate::untrusted_text::Stripped;

/// A problem with what `alz init` was asked to do, printed as `error [init] <message>`. The
/// messages that hold text of the command line hold it stripped (spec §10.1); a `--stack` value
/// is never printed, because a source can hold a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaffoldError {
    /// A `layer.yaml` is already in the folder.
    LayerYamlExists,
    /// The name, given or taken from the folder's name, is not a layer name.
    NotALayerName(Stripped),
    /// The newest `layer-v*` tag of the hub is not a git ref, so it cannot be written as a pin.
    HubTagNotARef,
    /// A `--stack` value that is not a source, or whose pin is not a git ref.
    StackSource(StackSourceProblem),
    /// A `--stack` or `--hub` value that holds a control character, which would break the line of
    /// `layer.yaml` it is written on.
    ControlCharacter,
}

/// What is wrong with a `--stack` value, without the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackSourceProblem {
    /// The value is not a source.
    Source(SourceError),
    /// The text after the last `@` of a `path:` value is not a git ref.
    Pin,
}

impl fmt::Display for ScaffoldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayerYamlExists => formatter.write_str("layer.yaml already exists here"),
            Self::NotALayerName(name) => {
                write!(formatter, "'{name}' is not a valid layer name; pass --name")
            }
            Self::HubTagNotARef => {
                formatter.write_str("the newest layer-v* tag of the hub is not a valid git ref")
            }
            Self::StackSource(StackSourceProblem::Source(error)) => {
                write!(formatter, "a --stack value {}", error.rule())
            }
            Self::StackSource(StackSourceProblem::Pin) => {
                formatter.write_str("the ref after '@' of a --stack value must be a git ref")
            }
            Self::ControlCharacter => {
                formatter.write_str("a --stack or --hub value holds a control character")
            }
        }
    }
}

impl std::error::Error for ScaffoldError {}
