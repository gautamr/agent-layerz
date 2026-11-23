//! Why a file could not be read, and the diagnostics that say so (spec §5.1, §5.3).

use std::io;

use alz_core::json::{JsonError, JsonErrorKind};
use alz_core::yaml::{YamlError, YamlErrorKind};
use alz_core::{Code, Diagnostic, Message, Stripped, Where};
use thiserror::Error;

/// Why a file could not be read through a [`ReadOnlyDir`](crate::ReadOnlyDir). The error holds no
/// path and no input: what it says is fixed text.
#[derive(Debug, Error)]
pub enum ReadError {
    /// There is no such file or folder.
    #[error("no such file")]
    Missing,
    /// The name is a link, a folder, a pipe, a device or anything else but a regular file.
    #[error("not a regular file")]
    NotRegular,
    /// The file is longer than the cap it was read with.
    #[error("larger than the cap")]
    TooLarge,
    /// Any other failure of the operating system, such as a permission error.
    #[error("cannot be read")]
    Io(#[source] io::Error),
}

impl ReadError {
    /// The error as a diagnostic about the `layer.yaml` of the layer that was given as `source`
    /// (§5.1): a missing file, or a file that is not a regular file, is the error `layer`, and
    /// both names the layer as it was given, stripped. A file past the cap is the error `yaml`
    /// at `<source>:layer.yaml` (§7.13).
    #[must_use]
    pub fn into_layer_yaml_diagnostic(self, source: &Stripped) -> Diagnostic {
        match self {
            Self::Missing => layer_error(
                Message::new("missing layer.yaml in layer '")
                    .then(source.clone())
                    .then("'"),
            ),
            Self::NotRegular => layer_error(
                Message::new("layer.yaml in layer '")
                    .then(source.clone())
                    .then("' must be a regular file, not a link or a folder"),
            ),
            Self::TooLarge => YamlError::new(YamlErrorKind::TooLarge, None).into_diagnostic(
                Where::layer_file(source.clone(), Stripped::from_untrusted("layer.yaml")),
            ),
            Self::Io(_) => layer_error(
                Message::new("layer.yaml in layer '")
                    .then(source.clone())
                    .then("' cannot be read"),
            ),
        }
    }

    /// The error as the diagnostic `lock` about `layer.lock` of the repository (§5.3). A caller
    /// that treats a missing lock as no lock never converts [`ReadError::Missing`].
    #[must_use]
    pub fn into_lock_diagnostic(self) -> Diagnostic {
        let file = Where::repository_file(Stripped::from_untrusted("layer.lock"));
        match self {
            Self::TooLarge => {
                JsonError::new(JsonErrorKind::TooLarge, None).into_diagnostic(Code::Lock, file)
            }
            Self::Missing => lock_error(file, "layer.lock does not exist"),
            Self::NotRegular => lock_error(
                file,
                "layer.lock must be a regular file, not a link or a folder",
            ),
            Self::Io(_) => lock_error(file, "layer.lock cannot be read"),
        }
    }
}

fn layer_error(message: Message) -> Diagnostic {
    Diagnostic::error(Code::Layer, None, message)
}

fn lock_error(file: Where, message: &'static str) -> Diagnostic {
    Diagnostic::error(Code::Lock, Some(file), Message::new(message))
}

#[cfg(test)]
mod tests {
    use alz_core::Stripped;

    use super::ReadError;

    fn source(text: &str) -> Stripped {
        Stripped::from_untrusted(text)
    }

    #[test]
    fn a_missing_layer_yaml_names_the_layer_as_given_and_stripped() {
        let line = ReadError::Missing
            .into_layer_yaml_diagnostic(&source("../a\u{1b}[31mb\nerror [x] y"))
            .to_string();
        assert_eq!(
            line,
            "error [layer] missing layer.yaml in layer '../a[31mberror [x] y'"
        );
    }

    #[test]
    fn a_layer_yaml_that_is_not_a_regular_file_is_the_error_layer() {
        let line = ReadError::NotRegular
            .into_layer_yaml_diagnostic(&source("x"))
            .to_string();
        assert_eq!(
            line,
            "error [layer] layer.yaml in layer 'x' must be a regular file, not a link or a folder"
        );
    }

    #[test]
    fn a_layer_yaml_past_the_cap_is_the_error_yaml_at_the_file() {
        let line = ReadError::TooLarge
            .into_layer_yaml_diagnostic(&source("x"))
            .to_string();
        assert_eq!(
            line,
            "error [yaml] x:layer.yaml: the file is larger than 1 MiB"
        );
    }

    #[test]
    fn a_lock_past_the_cap_is_the_error_lock_at_the_file() {
        let line = ReadError::TooLarge.into_lock_diagnostic().to_string();
        assert_eq!(
            line,
            "error [lock] layer.lock: the file is larger than 1 MiB"
        );
    }
}
