//! The diagnostics of resolving a source (spec §5.2, §5.3 and §6, decision-26, TASK-68), as pure
//! values. Each takes stripped text or a validated value, never a raw string, so no control or
//! format character and no unstripped source reaches a line. None has a location: the source or
//! the layer is named in the message, as the spec words each one.

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::git::{FetchFailureLine, ListingRefusal};
use crate::layer::LayerName;
use crate::lock::ObjectId;
use crate::path::SafeRelPath;
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

/// A cached `github:` or `git:` layer that differs from a fresh clone of the same commit, when the
/// lock has no entry to compare it with (§6): the same "delete that directory" error.
#[must_use]
pub fn cache_differs_from_clone(source: Stripped, cache_directory: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Cache,
        None,
        Message::new("cached layer '")
            .then(source)
            .then("' differs from a fresh clone of the same commit: its files in ")
            .then(cache_directory)
            .then(" changed after they were cached; delete that directory and rebuild"),
    )
}

/// A local `git:` path with a `..` segment (§7.12): text and disk can disagree about where it
/// leads.
#[must_use]
pub fn local_git_path_has_dotdot(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Source,
        None,
        Message::new("'")
            .then(source)
            .then("': a local git: path must not contain '..' segments; use a path without them"),
    )
}

/// A `file://` URL in a `git:` source that names no path.
#[must_use]
pub fn invalid_file_url(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Source,
        None,
        Message::new("'")
            .then(source)
            .then("': not a valid file:// URL"),
    )
}

/// A `path:` layer or local `git:` source whose real directory touches an account folder or the
/// home folder (§6 "Layers in or around an account folder"). alz never reads there.
#[must_use]
pub fn layer_touches_account_folder(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Source,
        None,
        Message::new("layer '")
            .then(source)
            .then("' touches an account folder or the home folder; alz never reads there"),
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

/// A pinned `path:` layer whose tree could not be read from the repository (§6): git failed,
/// outran its deadline, or the repository lacks a blob. `cannot read '<source>' at <ref>: <detail>`,
/// and without the detail when there is none.
#[must_use]
pub fn cannot_read_pinned(
    source: Stripped,
    git_ref: &GitRef,
    detail: Option<Stripped>,
) -> Diagnostic {
    let message = Message::new("cannot read '")
        .then(source)
        .then("' at ")
        .then(crate::MessagePart::printable(git_ref));
    Diagnostic::error(
        Code::Source,
        None,
        detail.map_or(message.clone(), |detail| message.then(": ").then(detail)),
    )
}

/// A `tree2@` entry that no longer matches the git tree it is named after (§6): the "delete that
/// directory" error. It never names a file or holds content.
#[must_use]
pub fn cache_differs_from_tree(
    source: Stripped,
    tree: &ObjectId,
    cache_directory: Stripped,
) -> Diagnostic {
    Diagnostic::error(
        Code::Cache,
        None,
        Message::new("cached layer '")
            .then(source)
            .then("' no longer matches git tree ")
            .then(Stripped::from_untrusted(&tree.to_string()))
            .then(": its files in ")
            .then(cache_directory)
            .then(" changed after they were cached; delete that directory and rebuild"),
    )
}

/// A pinned `path:` layer whose git tree lists a path that is refused (§5.3 "File names", §6).
/// Nothing was written. No path is printed.
#[must_use]
pub fn pinned_tree_refused(refusal: ListingRefusal, source: Stripped) -> Diagnostic {
    match refusal {
        ListingRefusal::NotUtf8 => file_name_not_utf8(source),
        ListingRefusal::NotSafe => Diagnostic::error(
            Code::Layer,
            None,
            Message::new("the git tree of layer '")
                .then(source)
                .then("' lists a path with a backslash, a '.', '..' or '.git' segment; rename it"),
        ),
        ListingRefusal::Collision => Diagnostic::error(
            Code::Layer,
            None,
            Message::new("the git tree of layer '")
                .then(source)
                .then("' lists two paths that one file system would merge (equal after Unicode normalisation and lowercasing); rename one"),
        ),
    }
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

/// A layer that the walk could not read, or whose names are not allowed, that is not a build of
/// the cache (§5.3 "File names"): the working tree of an unpinned `path:` layer.
#[must_use]
pub fn layer_unreadable(source: Stripped) -> Diagnostic {
    Diagnostic::error(
        Code::Layer,
        None,
        Message::new("layer '")
            .then(source)
            .then("' holds a file that cannot be read or a name that is not allowed"),
    )
}

/// A `.env` file in a layer (§5.3 "`.env` files"): the error `env-file` at `<layer>:<path>`. The
/// file was listed and never opened.
#[must_use]
pub fn env_file_in_layer(layer: &LayerName, file: &SafeRelPath) -> Diagnostic {
    Diagnostic::error(
        Code::EnvFile,
        Some(Where::layer_file(
            Stripped::from_untrusted(layer.as_str()),
            Stripped::from_untrusted(file.as_str()),
        )),
        Message::new("layers must not contain .env files; remove it"),
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
        cache_differs_from_clone, cache_differs_from_lock, cache_differs_from_tree, cannot_fetch,
        cannot_read_pinned, cannot_resolve_ref, cannot_store, file_name_not_utf8, invalid_file_url,
        layer_directory_not_found, layer_touches_account_folder, local_git_path_has_dotdot,
        pinned_tree_refused, source_holds_credentials,
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
    fn a_pinned_tree_failure_names_the_source_the_ref_and_the_detail() {
        let git_ref = GitRef::try_from("v1").expect("a ref");
        assert_eq!(
            cannot_read_pinned(
                stripped("path:../hub"),
                &git_ref,
                Some(stripped("git did not finish within 120 seconds"))
            )
            .to_string(),
            "error [source] cannot read 'path:../hub' at v1: git did not finish within 120 seconds"
        );
        assert_eq!(
            cannot_read_pinned(stripped("path:../hub"), &git_ref, None).to_string(),
            "error [source] cannot read 'path:../hub' at v1"
        );
    }

    #[test]
    fn a_tree_entry_that_changed_says_to_delete_its_directory() {
        let tree = crate::lock::ObjectId::try_from("b8b1dadfa8feded6f6d93583dd449585eaafb9f8")
            .expect("an id");
        assert_eq!(
            cache_differs_from_tree(stripped("path:../hub"), &tree, stripped("/c/tree2@x"))
                .to_string(),
            "error [cache] cached layer 'path:../hub' no longer matches git tree b8b1dadfa8feded6f6d93583dd449585eaafb9f8: its files in /c/tree2@x changed after they were cached; delete that directory and rebuild"
        );
    }

    #[test]
    fn a_refused_tree_names_the_layer_and_never_a_path() {
        use crate::git::ListingRefusal;
        for refusal in [
            ListingRefusal::NotUtf8,
            ListingRefusal::NotSafe,
            ListingRefusal::Collision,
        ] {
            let text = pinned_tree_refused(refusal, stripped("path:../hub")).to_string();
            assert!(text.starts_with("error [layer] "), "{text}");
            assert!(text.contains("'path:../hub'"), "{text}");
        }
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
            cache_differs_from_clone(stripped(source), stripped(directory)),
            local_git_path_has_dotdot(stripped(source)),
            invalid_file_url(stripped(source)),
            layer_touches_account_folder(stripped(source)),
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
