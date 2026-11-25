//! Why the walk to a directory ended without a handle, and the diagnostic that says so.

use std::io;

use alz_core::account::Relation;
use alz_core::path::Stop;
use alz_core::{Code, Diagnostic, Message, Stripped};
use thiserror::Error;

/// Why a path could not be opened as a directory by the component walk. The error holds no path.
#[derive(Debug, Error)]
pub enum WalkError {
    /// The path touches an account folder or the home folder (§9.1); nothing there was looked at.
    #[error("{0}")]
    Refused(Relation),
    /// A folder on the way, or the directory itself, does not exist.
    #[error("a folder on the way does not exist")]
    Missing,
    /// A name on the way, or the directory itself, is not a folder.
    #[error("a name on the way is not a folder")]
    NotADirectory,
    /// More than 40 links were followed.
    #[error("too many links")]
    TooManyLinks,
    /// Any other failure of the operating system, such as a permission error.
    #[error("cannot be opened")]
    Io(#[source] io::Error),
}

impl From<Stop> for WalkError {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Refused(relation) | Stop::Inside { relation, .. } => Self::Refused(relation),
            Stop::TooManyLinks => Self::TooManyLinks,
            Stop::NotFound => Self::Missing,
            Stop::NotADirectory => Self::NotADirectory,
        }
    }
}

impl WalkError {
    /// The error as a diagnostic about the directory of the layer that was given as `source`. A
    /// directory that touches an account folder or the home folder is the error `source` (§6
    /// "Layers in or around an account folder"); any other failure is the error `layer`. Neither
    /// holds a path, only the layer as it was given, stripped.
    #[must_use]
    pub fn into_layer_diagnostic(self, source: &Stripped) -> Diagnostic {
        match self {
            Self::Refused(_) => Diagnostic::error(
                Code::Source,
                None,
                Message::new("layer '")
                    .then(source.clone())
                    .then("' touches an account folder or the home folder; alz never reads there"),
            ),
            Self::Missing | Self::NotADirectory => Diagnostic::error(
                Code::Layer,
                None,
                Message::new("layer '")
                    .then(source.clone())
                    .then("' is not a directory"),
            ),
            Self::TooManyLinks | Self::Io(_) => Diagnostic::error(
                Code::Layer,
                None,
                Message::new("layer '")
                    .then(source.clone())
                    .then("' cannot be opened"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use alz_core::Stripped;
    use alz_core::account::Relation;

    use super::WalkError;

    #[test]
    fn a_refused_layer_is_the_error_source_and_names_the_layer_only() {
        let line = WalkError::Refused(Relation::Home)
            .into_layer_diagnostic(&Stripped::from_untrusted("../x\u{1b}[0m"))
            .to_string();
        assert_eq!(
            line,
            "error [source] layer '../x[0m' touches an account folder or the home folder; alz never reads there"
        );
    }

    /// Spec §6 "Layers in or around an account folder": a layer directory that cannot be opened
    /// (a cycle of links, or an operating system failure) is the error `layer` that names the
    /// layer as given and no operating system error.
    #[test]
    fn a_layer_directory_that_cannot_be_opened_is_the_error_layer_without_the_system_error() {
        for error in [
            WalkError::TooManyLinks,
            WalkError::Io(std::io::Error::from_raw_os_error(13)),
        ] {
            let line = error
                .into_layer_diagnostic(&Stripped::from_untrusted("x\u{1b}[0m"))
                .to_string();
            assert_eq!(line, "error [layer] layer 'x[0m' cannot be opened");
        }
    }

    #[test]
    fn a_missing_layer_directory_is_the_error_layer() {
        let line = WalkError::Missing
            .into_layer_diagnostic(&Stripped::from_untrusted("x"))
            .to_string();
        assert_eq!(line, "error [layer] layer 'x' is not a directory");
    }
}
