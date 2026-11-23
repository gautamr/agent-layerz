//! §6 step 2 over resolved layers, and the report of a whole stack.

use crate::diagnostic::Diagnostics;
use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::stack::Source;
use crate::stack::resolved::{
    FailedEntry, ResolvedEntry, ResolvedLayer, ResolvedStack, StackReport,
};
use crate::untrusted_text::Stripped;

fn layer(source: &str, name: &str, visibility: &str) -> ResolvedLayer {
    let yaml = format!("name: {name}\nkind: personal\nvisibility: {visibility}\n");
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    ResolvedLayer::new(
        Source::try_from(source).expect("a source"),
        LayerMeta::from_yaml(&node).expect("a layer"),
        LayerContents::default(),
        None,
    )
}

fn lines(diagnostics: &Diagnostics) -> Vec<String> {
    diagnostics.iter().map(ToString::to_string).collect()
}

fn resolved(source: &str, name: &str, visibility: &str) -> ResolvedEntry {
    ResolvedEntry {
        layer: layer(source, name, visibility),
        diagnostics: Diagnostics::new(),
    }
}

#[test]
fn visibility_may_only_grow_along_the_stack() {
    let layers = vec![
        layer("path:../a", "a", "public"),
        layer("path:../b", "b", "private"),
        layer("path:../c", "c", "confidential"),
    ];
    assert!(ResolvedStack::validate(layers).is_ok());
}

#[test]
fn a_public_layer_after_a_private_one_is_the_error_visibility_at_its_source() {
    let found = ResolvedStack::validate(vec![
        layer("path:../a", "a", "private"),
        layer("path:../b", "b", "public"),
    ])
    .expect_err("refused");
    assert_eq!(
        lines(&found),
        [
            "error [visibility] path:../b: public layer 'b' cannot extend private layer 'a'; order the stack from most public to most private"
        ]
    );
}

#[test]
fn every_earlier_layer_that_is_more_private_is_named() {
    let found = ResolvedStack::validate(vec![
        layer("path:../a", "a", "confidential"),
        layer("path:../b", "b", "private"),
        layer("path:../c", "c", "public"),
    ])
    .expect_err("refused");
    assert_eq!(found.error_count(), 3);
}

#[test]
fn a_name_used_twice_is_the_error_duplicate_layer_naming_both_sources() {
    let found = ResolvedStack::validate(vec![
        layer("path:../a", "same", "public"),
        layer("github:o/r@v1", "same", "public"),
    ])
    .expect_err("refused");
    assert_eq!(
        lines(&found),
        [
            "error [duplicate-layer] layer name 'same' is used by both 'path:../a' and 'github:o/r@v1'"
        ]
    );
}

#[test]
fn duplicates_come_before_visibility_problems() {
    let found = ResolvedStack::validate(vec![
        layer("path:../a", "a", "private"),
        layer("path:../b", "a", "public"),
    ])
    .expect_err("refused");
    let all = lines(&found);
    assert!(all[0].starts_with("error [duplicate-layer]"), "{all:?}");
    assert!(all[1].starts_with("error [visibility]"), "{all:?}");
}

#[test]
fn a_report_names_each_entry_and_holds_the_stack_when_nothing_failed() {
    let report = StackReport::of(vec![
        Ok(resolved("path:../a", "a", "public")),
        Ok(resolved("path:../b", "b", "private")),
    ]);
    let names: Vec<&str> = report.names().iter().map(Stripped::as_str).collect();
    assert_eq!(names, ["a", "b"]);
    assert!(!report.diagnostics().has_errors());
    assert!(report.stack().is_some());
}

#[test]
fn a_failed_entry_is_named_by_its_source_and_the_other_problems_are_still_found() {
    let failed = FailedEntry {
        source: Stripped::from_untrusted("path:../gone"),
        diagnostics: Diagnostics::new().with(
            crate::stack::resolution_failure::layer_directory_not_found(Stripped::from_untrusted(
                "path:../gone",
            )),
        ),
    };
    let report = StackReport::of(vec![
        Ok(resolved("path:../a", "a", "private")),
        Err(failed),
        Ok(resolved("path:../c", "c", "public")),
    ]);
    let names: Vec<&str> = report.names().iter().map(Stripped::as_str).collect();
    assert_eq!(names, ["a", "path:../gone", "c"]);
    let all = lines(report.diagnostics());
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all[0].starts_with("error [source]"), "{all:?}");
    assert!(all[1].starts_with("error [visibility]"), "{all:?}");
    assert!(report.stack().is_none());
}

#[test]
fn an_error_that_did_not_stop_an_entry_still_keeps_the_stack_from_being_valid() {
    let with_env_file = Ok(ResolvedEntry {
        layer: layer("path:../a", "a", "public"),
        diagnostics: Diagnostics::new().with(crate::stack::resolution_failure::env_file_in_layer(
            layer("path:../a", "a", "public").meta().name(),
            &crate::path::SafeRelPath::try_from(".env").expect("a path"),
        )),
    });
    let report = StackReport::of(vec![with_env_file]);
    assert_eq!(
        lines(report.diagnostics()),
        ["error [env-file] a:.env: layers must not contain .env files; remove it"]
    );
    assert!(report.stack().is_none());
}

#[test]
fn the_files_of_a_resolved_layer_are_derived_from_its_contents() {
    let yaml = "name: a\nkind: personal\nvisibility: private\n";
    let contents: LayerContents = [
        (
            crate::path::SafeRelPath::try_from("layer.yaml").expect("a path"),
            yaml.as_bytes().to_vec(),
        ),
        (
            crate::path::SafeRelPath::try_from("instructions/00.md").expect("a path"),
            b"text".to_vec(),
        ),
    ]
    .into_iter()
    .collect();
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    let layer = ResolvedLayer::new(
        Source::try_from("path:../a").expect("a source"),
        LayerMeta::from_yaml(&node).expect("a layer"),
        contents.clone(),
        None,
    );
    assert_eq!(layer.contents(), &contents);
    assert_eq!(layer.files(), &contents.files());
    assert_eq!(layer.files().len(), 2);
}
