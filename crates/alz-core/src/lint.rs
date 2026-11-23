//! What `alz lint` checks of the files R2 models (spec §10): the schema of one `layer.yaml`, and
//! the stack manifest and `layer.lock` of a repository. Every function here is pure: the caller
//! reads the files and prints the diagnostics.
//!
//! Instructions, skills, agents, settings, protected variables, the leak guard, the secret scan
//! and slots arrive with the phases that model them (R4, R5).

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
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
    let file = || Where::layer_file(source.clone(), Stripped::from_untrusted("layer.yaml"));
    let (name, diagnostics) = match document {
        Err(error) => (
            source.clone(),
            Diagnostics::new().with(error.into_diagnostic(file())),
        ),
        Ok(root) => match LayerMeta::from_yaml(&root) {
            Ok(meta) => (
                Stripped::from_untrusted(meta.name().as_str()),
                Diagnostics::new(),
            ),
            Err(errors) => (source.clone(), errors.diagnostics(source)),
        },
    };
    LayerLint { name, diagnostics }
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

/// Lints a repository's stack manifest and, when there is one, its `layer.lock`: every manifest
/// error, then the lock's error. A manifest and a lock that are both valid end with the error
/// `not-ported`, because resolving the stack's layers is not ported yet (R3, TASK-55.4); R3
/// removes it.
#[must_use]
pub fn stack_manifest(manifest: &Node, lock: LockFile<'_>) -> Diagnostics {
    let manifest_errors = Manifest::from_yaml(manifest).map_or_else(
        |errors| errors.manifest_diagnostics(),
        |_| Diagnostics::new(),
    );
    let lock_errors = match lock {
        LockFile::Absent => Diagnostics::new(),
        LockFile::Bytes(bytes) => Lock::parse(bytes).map_or_else(
            |error| Diagnostics::new().with(error.into_diagnostic()),
            |_| Diagnostics::new(),
        ),
        LockFile::Unreadable(diagnostic) => Diagnostics::new().with(diagnostic),
    };
    let found = manifest_errors.merge(lock_errors);
    if found.has_errors() {
        found
    } else {
        found.with(not_ported())
    }
}

fn not_ported() -> Diagnostic {
    Diagnostic::error(
        Code::NotPorted,
        Some(Where::repository_file(Stripped::from_untrusted(
            "layer.yaml",
        ))),
        Message::new(
            "linting a repository's stack is not ported to the Rust alz yet (R3, TASK-55.4); \
             use alz lint --layer <path> for one layer",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{LockFile, is_stack_manifest, layer, parse_document, stack_manifest};
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
    fn a_valid_manifest_and_no_lock_stop_at_not_ported() {
        let found = stack_manifest(&node("stack:\n  - source: path:../a\n"), LockFile::Absent);
        assert_eq!(
            lines(&found),
            [
                "error [not-ported] layer.yaml: linting a repository's stack is not ported to the Rust alz yet (R3, TASK-55.4); use alz lint --layer <path> for one layer"
            ]
        );
    }

    #[test]
    fn manifest_errors_and_lock_errors_are_all_reported_and_not_ported_is_not() {
        let found = stack_manifest(&node("stack: 1\nbogus: 2\n"), LockFile::Bytes(b"{"));
        let all = lines(&found);
        assert!(all.iter().any(|line| line.starts_with("error [manifest]")));
        assert!(all.iter().any(|line| line.starts_with("error [lock]")));
        assert!(!all.iter().any(|line| line.contains("not-ported")));
    }
}
