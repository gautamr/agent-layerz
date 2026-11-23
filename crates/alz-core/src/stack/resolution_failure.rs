//! The diagnostics of resolving a source (spec §5.2, §5.3 and §6, decision-26, TASK-68), as pure
//! values. Each takes stripped text or a validated value, never a raw string, so no control or
//! format character and no unstripped source reaches a line. None has a location: the source or
//! the layer is named in the message, as the spec words each one.

use crate::diagnostic::{Code, Diagnostic, Message};
use crate::git::FetchFailureLine;
use crate::layer::LayerName;
use crate::stack::GitRef;
use crate::stack::source::CREDENTIALS_MESSAGE;
use crate::untrusted_text::Stripped;

/// A `github:` or `git:` source whose clone, `git rev-parse` or checkout failed (§6 "A failed
/// fetch"): `cannot fetch '<source>': <line>`, and `cannot fetch '<source>'` when git wrote no
/// line, never with a trailing `: `.
#[must_use]
pub fn cannot_fetch(source: Stripped, line: Option<FetchFailureLine>) -> Diagnostic {
    let message = Message::new("cannot fetch '").then(source).then("'");
    Diagnostic::error(
        Code::Source,
        None,
        line.map_or(message.clone(), |line| {
            message.then(": ").then(line.into_stripped())
        }),
    )
}

/// A clone that git made but alz could not read for the hash comparison or move into the cache
/// (§6): `cannot store '<source>' in the layer cache at <dir>`. It is not a fetch failure.
#[must_use]
pub fn cannot_store(source: Stripped, cache_directory: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Cache,
        None,
        Message::new("cannot store '")
            .then(source)
            .then("' in the layer cache at ")
            .then(cache_directory),
    )
}

/// A layer read from the cache whose content hash differs from `layer.lock` (§5.3 "Cache
/// integrity"). It names the layer by its lock entry and its source, gives the directory and says
/// to delete it. It never holds file content.
#[must_use]
pub fn cache_differs_from_lock(
    name: &LayerName,
    source: Stripped,
    cache_directory: Stripped,
) -> Diagnostic {
    Diagnostic::error(
        Code::Cache,
        None,
        Message::new("cached layer '")
            .then(crate::MessagePart::printable(name))
            .then("' (")
            .then(source)
            .then(") no longer matches layer.lock: its files in ")
            .then(cache_directory)
            .then(" changed after they were cached; delete that directory and rebuild"),
    )
}

/// A `path:` layer whose directory is missing, not a directory, or not allowed (§6).
#[must_use]
pub fn layer_directory_not_found(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Source,
        None,
        Message::new("layer directory not found for '")
            .then(source)
            .then("'"),
    )
}

/// A pinned `path:` layer whose ref does not give a layer tree (§6).
#[must_use]
pub fn cannot_resolve_ref(git_ref: &GitRef, source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Source,
        None,
        Message::new("cannot resolve ref ")
            .then(crate::MessagePart::printable(git_ref))
            .then(" for '")
            .then(source)
            .then("': not a git repo, unknown ref, the layer did not exist at that ref, or its path is not a plain directory there"),
    )
}

/// A name in a layer that is not valid UTF-8 (§5.3 "File names"). The name itself is never
/// printed.
#[must_use]
pub fn file_name_not_utf8(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Layer,
        None,
        Message::new("a file or folder name in layer '")
            .then(source)
            .then("' is not valid UTF-8; rename it"),
    )
}

/// A `git:` source that holds credentials (§5.2 "Credentials in a source"). The source is never
/// printed.
#[must_use]
pub fn source_holds_credentials() -> Diagnostic {
    Diagnostic::error(Code::Source, None, Message::new(CREDENTIALS_MESSAGE))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::{
        cache_differs_from_lock, cannot_fetch, cannot_resolve_ref, cannot_store,
        file_name_not_utf8, layer_directory_not_found, source_holds_credentials,
    };
    use crate::diagnostic::Diagnostic;
    use crate::git::FetchFailureLine;
    use crate::layer::LayerName;
    use crate::stack::GitRef;
    use crate::untrusted_text::Stripped;

    fn stripped(text: &str) -> Stripped {
        Stripped::from_untrusted(text)
    }

    fn name() -> LayerName {
        LayerName::try_from("team").expect("a valid layer name")
    }

    #[test]
    fn cannot_fetch_names_the_source_and_git_s_line() {
        let line = FetchFailureLine::from_stderr(b"\nfatal: no access\nmore\n");
        assert_eq!(
            cannot_fetch(stripped("github:o/r@v1"), line).to_string(),
            "error [source] cannot fetch 'github:o/r@v1': fatal: no access"
        );
    }

    #[test]
    fn cannot_fetch_without_a_line_ends_after_the_quoted_source() {
        let line = FetchFailureLine::from_stderr(b" \n\x1b\n");
        assert_eq!(
            cannot_fetch(stripped("git:ssh://h/x.git@v1"), line).to_string(),
            "error [source] cannot fetch 'git:ssh://h/x.git@v1'"
        );
    }

    #[test]
    fn the_cache_store_failure_is_its_own_error_and_not_a_fetch_failure() {
        let diagnostic = cannot_store(stripped("github:o/r@v1"), stripped("/c/agent-layerz"));
        assert_eq!(
            diagnostic.to_string(),
            "error [cache] cannot store 'github:o/r@v1' in the layer cache at /c/agent-layerz"
        );
        assert!(!diagnostic.to_string().contains("cannot fetch"));
        assert_ne!(
            diagnostic.to_string(),
            cannot_fetch(stripped("github:o/r@v1"), None).to_string()
        );
    }

    #[test]
    fn a_cached_layer_that_differs_names_the_lock_entry_the_source_and_the_directory() {
        assert_eq!(
            cache_differs_from_lock(&name(), stripped("github:o/r@v1"), stripped("/c/x@1"))
                .to_string(),
            "error [cache] cached layer 'team' (github:o/r@v1) no longer matches layer.lock: \
             its files in /c/x@1 changed after they were cached; delete that directory and rebuild"
        );
    }

    #[test]
    fn the_other_source_messages_follow_the_spec() {
        let git_ref = GitRef::try_from("v1").expect("a valid ref");
        assert_eq!(
            layer_directory_not_found(stripped("path:../hub")).to_string(),
            "error [source] layer directory not found for 'path:../hub'"
        );
        assert_eq!(
            cannot_resolve_ref(&git_ref, stripped("path:../hub@v1")).to_string(),
            "error [source] cannot resolve ref v1 for 'path:../hub@v1': not a git repo, unknown \
             ref, the layer did not exist at that ref, or its path is not a plain directory there"
        );
        assert_eq!(
            file_name_not_utf8(stripped("path:../hub")).to_string(),
            "error [layer] a file or folder name in layer 'path:../hub' is not valid UTF-8; \
             rename it"
        );
        assert_eq!(
            source_holds_credentials().to_string(),
            "error [source] a git: source in layer.yaml holds credentials in its URL; remove \
             them and use a git credential helper"
        );
    }

    fn any_text() -> impl Strategy<Value = String> {
        proptest::prop_oneof![
            any::<String>(),
            "[a:'\\\\ \r\n\u{1b}\u{200b}\u{202e}\u{2028}\u{e9}]{0,12}"
        ]
    }

    fn any_stderr() -> impl Strategy<Value = Vec<u8>> {
        proptest::prop_oneof![
            proptest::collection::vec(any::<u8>(), 0..48),
            any_text().prop_map(String::into_bytes)
        ]
    }

    /// Every diagnostic of this module, built from untrusted text.
    fn every_diagnostic(source: &str, directory: &str, stderr: &[u8]) -> Vec<Diagnostic> {
        let git_ref = GitRef::try_from("v1").expect("a valid ref");
        vec![
            cannot_fetch(stripped(source), FetchFailureLine::from_stderr(stderr)),
            cannot_store(stripped(source), stripped(directory)),
            cache_differs_from_lock(&name(), stripped(source), stripped(directory)),
            layer_directory_not_found(stripped(source)),
            cannot_resolve_ref(&git_ref, stripped(source)),
            file_name_not_utf8(stripped(source)),
            source_holds_credentials(),
        ]
    }

    proptest! {
        #[test]
        fn no_rendered_source_cache_or_layer_diagnostic_holds_a_control_or_format_character(
            source in any_text(),
            directory in any_text(),
            stderr in any_stderr(),
        ) {
            for diagnostic in every_diagnostic(&source, &directory, &stderr) {
                let line = diagnostic.to_string();
                let restripped = stripped(&line);
                prop_assert_eq!(restripped.as_str(), line.as_str());
                let has_line_break = line.contains(['\n', '\r', '\u{2028}', '\u{2029}']);
                prop_assert!(!has_line_break);
            }
        }

        #[test]
        fn cannot_fetch_never_ends_with_a_colon_and_a_space(
            source in any_text(),
            stderr in any_stderr(),
        ) {
            let line = cannot_fetch(stripped(&source), FetchFailureLine::from_stderr(&stderr))
                .to_string();
            prop_assert!(!line.ends_with(": "));
            let without_line = cannot_fetch(stripped(&source), None).to_string();
            prop_assert_eq!(without_line, format!("error [source] cannot fetch '{}'", stripped(&source)));
        }

        #[test]
        fn a_raw_source_never_appears_in_a_message(
            source in "[a-z]{1,5}[\u{1b}\u{200b}\u{202e}\r\n][a-z]{1,5}",
            directory in any_text(),
            stderr in any_stderr(),
        ) {
            for diagnostic in every_diagnostic(&source, &directory, &stderr) {
                prop_assert!(!diagnostic.to_string().contains(&source));
            }
        }
    }
}
