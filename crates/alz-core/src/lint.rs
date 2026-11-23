//! What `alz lint` checks of the files R2 models (spec §10): the schema of one `layer.yaml`, and
//! the stack manifest and `layer.lock` of a repository, and the metadata of each stack entry. Every
//! function here is pure: the caller reads the files and prints the diagnostics.
//!
//! Instructions, skills, agents, settings, protected variables, the leak guard, the secret scan
//! and slots arrive with the phases that model them (R4, R5).

use crate::diagnostic::{Diagnostic, Diagnostics, Where};
use crate::layer::LayerMeta;
use crate::lock::Lock;
use crate::stack::Manifest;
use crate::untrusted_text::Stripped;
use crate::yaml::{self, Node, Value, YamlError, YamlText};

/// The result of linting one layer: what to call it in the summary line, and what was wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerLint {
    name: Stripped,
    diagnostics: Diagnostics,
}

impl LayerLint {
    /// The layer's name when its `layer.yaml` gave a valid one, else the directory as it was
    /// given (§10.1), stripped.
    #[must_use]
    pub const fn name(&self) -> &Stripped {
        &self.name
    }

    /// What was wrong with the layer, in document order.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The diagnostics, consumed.
    #[must_use]
    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}

/// Parses the bytes of a `layer.yaml`: at most 1 MiB, strict UTF-8, the YAML rules of §7.13.
///
/// # Errors
///
/// The first problem of the file, as a [`YamlError`].
pub fn parse_document(bytes: &[u8]) -> Result<Node, YamlError> {
    yaml::parse(&YamlText::from_bytes(bytes)?)
}

/// Whether the document is a repository's stack manifest and not a layer: a mapping with a
/// `stack` field (§5.1).
#[must_use]
pub fn is_stack_manifest(document: &Node) -> bool {
    match document.value() {
        Value::Map(entries) => entries.get("stack").is_some(),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_) => false,
    }
}

/// Lints the `layer.yaml` of the layer that was given as `source`: a YAML error, or every schema
/// error at once. `source` is the name of the layer's directory as the person wrote it.
#[must_use]
pub fn layer(source: &Stripped, document: Result<Node, YamlError>) -> LayerLint {
    match read_layer(source, document) {
        Ok(meta) => LayerLint {
            name: Stripped::from_untrusted(meta.name().as_str()),
            diagnostics: Diagnostics::new(),
        },
        Err(diagnostics) => LayerLint {
            name: source.clone(),
            diagnostics,
        },
    }
}

/// What the caller found where the repository's `layer.lock` should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockFile<'a> {
    /// There is no lock.
    Absent,
    /// The lock's bytes, read.
    Bytes(&'a [u8]),
    /// A lock that could not be read, as the diagnostic that says why.
    Unreadable(Diagnostic),
}

/// The repository's stack manifest and, when it has one, its `layer.lock`, both valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFiles {
    /// The stack manifest.
    pub manifest: Manifest,
    /// The lock, `None` when the repository has none.
    pub lock: Option<Lock>,
}

/// Reads a repository's stack manifest and, when there is one, its `layer.lock`, as lint does.
///
/// # Errors
///
/// Every manifest error, then the lock's error, in that order; the layers are not resolved until
/// both files are valid.
pub fn read_stack(manifest: &Node, lock: LockFile<'_>) -> Result<StackFiles, Diagnostics> {
    let manifest = Manifest::from_yaml(manifest).map_err(|errors| errors.manifest_diagnostics());
    let lock = match lock {
        LockFile::Absent => Ok(None),
        LockFile::Bytes(bytes) => Lock::parse(bytes)
            .map(Some)
            .map_err(|error| Diagnostics::new().with(error.into_diagnostic())),
        LockFile::Unreadable(diagnostic) => Err(Diagnostics::new().with(diagnostic)),
    };
    match (manifest, lock) {
        (Ok(manifest), Ok(lock)) => Ok(StackFiles { manifest, lock }),
        (Err(manifest), Err(lock)) => Err(manifest.merge(lock)),
        (Err(problems), Ok(_)) | (Ok(_), Err(problems)) => Err(problems),
    }
}

/// Reads the `layer.yaml` of one stack entry, given as `source` as it was written: its metadata,
/// or every schema error at once.
///
/// # Errors
///
/// A YAML error, or every schema error, at `<source>:layer.yaml`.
pub fn read_layer(
    source: &Stripped,
    document: Result<Node, YamlError>,
) -> Result<LayerMeta, Diagnostics> {
    let file = || Where::layer_file(source.clone(), Stripped::from_untrusted("layer.yaml"));
    document
        .map_err(|error| Diagnostics::new().with(error.into_diagnostic(file())))
        .and_then(|root| LayerMeta::from_yaml(&root).map_err(|errors| errors.diagnostics(source)))
}

#[cfg(test)]
mod tests {
    use super::{LockFile, is_stack_manifest, layer, parse_document, read_layer, read_stack};
    use crate::untrusted_text::Stripped;

    fn node(text: &str) -> crate::yaml::Node {
        parse_document(text.as_bytes()).expect("valid YAML")
    }

    fn lines(diagnostics: &crate::Diagnostics) -> Vec<String> {
        diagnostics.iter().map(ToString::to_string).collect()
    }

    const LAYER: &str = "name: personal\nkind: personal\nvisibility: private\n";

    #[test]
    fn a_valid_layer_has_no_diagnostic_and_is_called_by_its_name() {
        let report = layer(&Stripped::from_untrusted("../x"), Ok(node(LAYER)));
        assert!(report.diagnostics().iter().next().is_none());
        assert_eq!(report.name().as_str(), "personal");
    }

    #[test]
    fn a_schema_error_names_the_field_and_the_layer_as_given() {
        let source = Stripped::from_untrusted("../x");
        let report = layer(&source, Ok(node("name: personal\nkind: personal\n")));
        assert_eq!(report.name().as_str(), "../x");
        assert_eq!(
            lines(report.diagnostics()),
            ["error [layer] ../x:layer.yaml:1:1: visibility is required"]
        );
    }

    #[test]
    fn a_yaml_error_is_the_error_yaml_at_the_file() {
        let source = Stripped::from_untrusted("d");
        let report = layer(&source, parse_document(b"a: [\n"));
        let [line] = lines(report.diagnostics()).try_into().expect("one line");
        assert!(line.starts_with("error [yaml] d:layer.yaml:"), "{line}");
    }

    #[test]
    fn a_stack_manifest_is_told_from_a_layer_by_its_stack_field() {
        assert!(is_stack_manifest(&node("stack: []\n")));
        assert!(!is_stack_manifest(&node(LAYER)));
        assert!(!is_stack_manifest(&node("- a\n")));
    }

    #[test]
    fn a_layer_with_a_stack_field_is_refused_as_a_layer() {
        let report = layer(
            &Stripped::from_untrusted("."),
            Ok(node("name: a\nstack: []\n")),
        );
        assert_eq!(report.diagnostics().error_count(), 1);
    }

    #[test]
    fn a_valid_manifest_without_a_lock_is_read_with_no_lock() {
        let files =
            read_stack(&node("stack:\n  - source: path:../a\n"), LockFile::Absent).expect("valid");
        assert_eq!(files.manifest.stack().len(), 1);
        assert!(files.lock.is_none());
    }

    #[test]
    fn manifest_errors_and_lock_errors_are_all_reported() {
        let found =
            read_stack(&node("stack: 1\nbogus: 2\n"), LockFile::Bytes(b"{")).expect_err("invalid");
        let all = lines(&found);
        assert!(all.iter().any(|line| line.starts_with("error [manifest]")));
        assert!(all.iter().any(|line| line.starts_with("error [lock]")));
    }

    #[test]
    fn a_lock_error_alone_stops_a_valid_manifest() {
        let found = read_stack(
            &node("stack:\n  - source: path:../a\n"),
            LockFile::Bytes(b"{"),
        )
        .expect_err("invalid");
        assert_eq!(found.error_count(), 1);
    }

    #[test]
    fn a_layer_is_read_into_its_metadata_or_every_schema_error() {
        let source = Stripped::from_untrusted("path:../x");
        let meta = read_layer(&source, Ok(node(LAYER))).expect("valid");
        assert_eq!(meta.name().as_str(), "personal");
        let found = read_layer(&source, Ok(node("name: a\n"))).expect_err("invalid");
        assert_eq!(
            lines(&found),
            [
                "error [layer] path:../x:layer.yaml:1:1: kind is required",
                "error [layer] path:../x:layer.yaml:1:1: visibility is required"
            ]
        );
    }
}
