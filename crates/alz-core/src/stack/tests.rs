//! Tests of the stack manifest as validated types (spec §5.2): the `ref` rules of TASK-61, the
//! defaults, the workspace and budget rules, every error of one file at once, and the guarantee
//! that no error holds a refused value.

use proptest::prelude::{prop_assert, proptest};

use crate::schema::SchemaErrors;
use crate::stack::{GitRef, Manifest, McpName, ProtectedPattern, Source, StackEntry, WorkspaceDir};
use crate::target::TargetName;
use crate::yaml::{Node, YamlText, parse};

const STACK: &str = "stack:\n  - source: path:./.layer\n";

fn tree(document: &str) -> Node {
    let text = YamlText::from_bytes(document.as_bytes()).expect("the test document is text");
    parse(&text).expect("the test document is YAML")
}

fn read(document: &str) -> Result<Manifest, SchemaErrors> {
    Manifest::from_yaml(&tree(document))
}

fn with(extra: &str) -> Result<Manifest, SchemaErrors> {
    read(&format!("{STACK}{extra}\n"))
}

fn messages(errors: &SchemaErrors) -> Vec<String> {
    errors
        .iter()
        .map(|error| error.message().to_string())
        .collect()
}

/// Everything a person would see for these errors, as the lint command prints it.
fn printed(errors: &SchemaErrors) -> String {
    errors
        .manifest_diagnostics()
        .to_string()
        .trim_end_matches('\n')
        .to_owned()
}

fn refused(document: &str) -> SchemaErrors {
    read(document).expect_err("the document breaks the schema")
}

fn entry_with(ref_line: &str) -> String {
    format!("stack:\n  - source: path:../hub/layer\n    ref: {ref_line}\n")
}

// ---- ref (TASK-61) ----

#[test]
fn a_quoted_ref_that_looks_like_a_number_is_kept_as_written() {
    let manifest = read(&entry_with("\"1.10\"")).expect("a quoted ref");
    let entry = manifest.stack().iter().next().expect("one entry");
    assert_eq!(entry.pin().map(GitRef::as_str), Some("1.10"));
}

#[test]
fn a_ref_that_is_a_list_a_number_or_a_mapping_is_refused_naming_the_entry_and_not_the_value() {
    for line in ["[x]", "1.10", "1", "{a: b}", "true", "[secretref]"] {
        let errors = refused(&entry_with(line));
        // Only a number gets the hint to quote it (§5.2).
        let hint = if matches!(line, "1.10" | "1") {
            "; put the value in quotes to keep it as text"
        } else {
            ""
        };
        assert_eq!(
            messages(&errors),
            [format!("stack[0].ref must be text{hint}")],
            "ref: {line}"
        );
        assert!(
            printed(&errors).starts_with("error [manifest] layer.yaml:3:10: stack[0].ref"),
            "{}",
            printed(&errors)
        );
    }
}

#[test]
fn a_ref_that_breaks_the_rule_is_refused_naming_the_entry_and_not_the_value() {
    for text in ["-x", "--upload-pack=touch", "a..b", "a b", "a:b", "é", ""] {
        let errors = refused(&entry_with(&format!("\"{text}\"")));
        let shown = printed(&errors);
        assert!(shown.contains("stack[0].ref must be a git ref"), "{shown}");
        if !text.is_empty() {
            assert!(!shown.contains(text), "{shown}");
        }
    }
}

#[test]
fn a_ref_on_a_second_entry_names_that_entry() {
    let errors = refused("stack:\n  - source: path:./a\n  - source: path:./b\n    ref: \"-bad\"\n");
    assert_eq!(
        messages(&errors)[0],
        "stack[1].ref must be a git ref: letters, digits and . _ / + -, not starting with '-' and without '..'"
    );
}

#[test]
fn a_ref_pins_a_path_source_and_is_checked_but_dropped_on_other_sources() {
    let manifest =
        read("stack:\n  - source: path:./a\n    ref: v1\n  - source: github:o/r@v2\n    ref: v3\n")
            .expect("a valid stack");
    let pins: Vec<Option<&str>> = manifest
        .stack()
        .iter()
        .map(|entry| entry.pin().map(GitRef::as_str))
        .collect();
    assert_eq!(pins, [Some("v1"), None]);
    let checked = refused("stack:\n  - source: github:o/r@v2\n    ref: \"-x\"\n");
    assert!(printed(&checked).contains("stack[0].ref must be a git ref"));
}

// ---- stack ----

#[test]
fn a_stack_entry_keeps_its_source_and_prints_it_back() {
    let manifest =
        read("stack:\n  - source: github:me/personal@v1.4.0\n  - source: path:./.layer\n")
            .expect("a valid stack");
    let sources: Vec<String> = manifest
        .stack()
        .iter()
        .map(|entry| entry.source().to_string())
        .collect();
    assert_eq!(sources, ["github:me/personal@v1.4.0", "path:./.layer"]);
    assert!(matches!(
        manifest.stack().iter().last().map(StackEntry::source),
        Some(Source::Path { .. })
    ));
}

#[test]
fn a_file_without_stack_an_empty_stack_and_a_malformed_stack_are_refused_with_the_field_path() {
    assert_eq!(
        messages(&refused("targets: [claude]\n")),
        ["stack is required"]
    );
    assert_eq!(
        messages(&refused("stack: []\n")),
        ["stack must not be empty"]
    );
    assert_eq!(messages(&refused("stack: x\n")), ["stack must be a list"]);
    assert_eq!(
        messages(&refused("stack:\n  - x\n")),
        ["stack[0] must be a mapping"]
    );
    assert_eq!(
        messages(&refused("stack:\n  - ref: v1\n")),
        ["stack[0].source is required"]
    );
    assert_eq!(
        messages(&refused("stack:\n  - source: 5\n")),
        ["stack[0].source must be text; put the value in quotes to keep it as text"]
    );
    assert_eq!(
        messages(&refused("- a\n")),
        ["the document must be a mapping"]
    );
    assert_eq!(
        printed(&refused("targets: [claude]\n")),
        "error [manifest] layer.yaml:1:1: stack is required"
    );
}

#[test]
fn unknown_fields_are_refused_as_quoted_keys_at_every_level() {
    let errors = refused(
        "stack:\n  - source: path:./a\n    extra: 1\nsecret-key: 1\nbudgets:\n  other: 1\nmcp:\n  x: 1\nworkspace:\n  y: 1\noptions:\n  z: 1\n",
    );
    assert_eq!(
        messages(&errors),
        [
            "unknown field 'extra' in stack[0]",
            "unknown field 'secret-key'",
            "unknown field 'other' in budgets",
            "unknown field 'x' in mcp",
            "unknown field 'y' in workspace",
            "unknown field 'z' in options",
        ]
    );
}

#[test]
fn a_source_the_spec_refuses_is_an_error_at_its_entry_and_never_prints_the_source() {
    for (source, rule) in [
        (
            "ftp://x",
            "stack[0].source must start with path:, github: or git:",
        ),
        ("path:", "stack[0].source must name a directory after path:"),
        (
            "github:o/r",
            "stack[0].source must end with @<ref>: <url>@<tag or commit>",
        ),
        ("github:o/r@a..SECRET", "stack[0].source must be a git ref"),
        (
            "git:-x@v1",
            "stack[0].source must have a URL that does not start with '-'",
        ),
        (
            "github:o@v1",
            "stack[0].source must be github:<owner>/<repo>@<ref>",
        ),
    ] {
        let errors = refused(&format!("stack:\n  - source: \"{source}\"\n"));
        let message = messages(&errors).remove(0);
        assert!(message.starts_with(rule), "{source}: {message}");
        assert!(!message.contains("SECRET"), "{message}");
    }
}

#[test]
fn a_git_source_with_credentials_is_the_error_source_without_a_location_or_the_token() {
    for source in [
        "git:https://user:tok3n@host/x.git@v1",
        "git:https://tok3n@host/x.git@v1",
        "git:ssh://git:tok3n@host/x.git@v1",
    ] {
        let errors = refused(&format!("stack:\n  - source: {source}\n"));
        let shown = printed(&errors);
        assert_eq!(
            shown,
            "error [source] a git: source in layer.yaml holds credentials in its URL; remove them and use a git credential helper"
        );
        assert!(!shown.contains("tok3n"));
    }
    for source in ["git:ssh://git@host/x.git@v1", "git:git@host:o/x.git@v1"] {
        read(&format!("stack:\n  - source: {source}\n")).expect("a user name alone is fine");
    }
}

// ---- targets ----

#[test]
fn targets_default_to_claude_and_are_a_set_in_name_order() {
    assert_eq!(
        read(STACK)
            .expect("valid")
            .targets()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [TargetName::Claude]
    );
    assert_eq!(
        with("targets: [codex, claude, codex]")
            .expect("valid")
            .targets()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [TargetName::Claude, TargetName::Codex]
    );
}

#[test]
fn an_unknown_target_names_its_item_and_not_the_value() {
    let errors = with("targets: [claude, cursor]").expect_err("unknown target");
    assert_eq!(
        messages(&errors),
        ["targets[1] must be a target: claude or codex"]
    );
    assert!(!printed(&errors).contains("cursor"));
    assert_eq!(
        messages(&with("targets: claude").expect_err("not a list")),
        ["targets must be a list"]
    );
}

// ---- workspace ----

#[test]
fn the_hub_comes_first_among_the_additional_directories_and_duplicates_go() {
    let manifest =
        with("workspace:\n  hub: ../hub/\n  add_dirs: [../x, ..\\hub, ../x, ./y]").expect("valid");
    let dirs: Vec<&str> = manifest
        .workspace()
        .add_dirs()
        .iter()
        .map(WorkspaceDir::as_str)
        .collect();
    assert_eq!(dirs, ["../hub", "../x", "y"]);
    assert_eq!(
        manifest.workspace().hub().map(WorkspaceDir::as_str),
        Some("../hub")
    );
    let none = read(STACK).expect("valid");
    assert!(none.workspace().hub().is_none() && none.workspace().add_dirs().is_empty());
}

#[test]
fn a_workspace_path_that_is_empty_or_not_relative_is_refused_by_field_without_the_path() {
    let errors =
        with("workspace:\n  hub: /etc/secret-hub\n  add_dirs: ['', ~/x, ok]").expect_err("refused");
    assert_eq!(
        messages(&errors),
        [
            "workspace.hub must be relative to the repository: not absolute and not starting with '~'",
            "workspace.add_dirs[0] must be a non-empty path",
            "workspace.add_dirs[1] must be relative to the repository: not absolute and not starting with '~'",
        ]
    );
    assert!(!printed(&errors).contains("secret-hub"));
}

// ---- lists, budgets, options ----

#[test]
fn protected_paths_and_mcp_enable_are_lists_of_text() {
    let manifest =
        with("protected_paths: [a/**, b]\nmcp:\n  enable: [context7, playwright]").expect("valid");
    assert_eq!(
        manifest
            .protected_paths()
            .iter()
            .map(ProtectedPattern::as_str)
            .collect::<Vec<_>>(),
        ["a/**", "b"]
    );
    assert_eq!(
        manifest
            .mcp_enable()
            .iter()
            .map(McpName::as_str)
            .collect::<Vec<_>>(),
        ["context7", "playwright"]
    );
    assert_eq!(
        messages(&with("protected_paths: [1]").expect_err("number")),
        ["protected_paths[0] must be text; put the value in quotes to keep it as text"]
    );
    assert_eq!(
        messages(&with("mcp:\n  enable: x").expect_err("not a list")),
        ["mcp.enable must be a list"]
    );
}

#[test]
fn budgets_are_whole_numbers_with_the_spec_defaults() {
    let defaults = read(STACK).expect("valid");
    assert_eq!(defaults.budgets().instructions_max_lines(), None);
    assert_eq!(defaults.budgets().instructions_max_bytes(), 32768);
    assert_eq!(defaults.budgets().always_loaded_skills_max(), None);

    let set = with(
        "budgets:\n  instructions_max_lines: 400\n  instructions_max_bytes: 1e3\n  always_loaded_skills_max: 0.0",
    )
    .expect("valid");
    assert_eq!(set.budgets().instructions_max_lines(), Some(400));
    assert_eq!(set.budgets().instructions_max_bytes(), 1000);
    assert_eq!(set.budgets().always_loaded_skills_max(), Some(0));
}

#[test]
fn a_budget_that_is_not_a_whole_number_of_0_or_more_is_refused() {
    let errors = with(
        "budgets:\n  instructions_max_lines: 1.5\n  instructions_max_bytes: -1\n  always_loaded_skills_max: '3'",
    )
    .expect_err("refused");
    assert_eq!(
        messages(&errors),
        [
            "budgets.instructions_max_lines must be a whole number",
            "budgets.instructions_max_bytes must be a whole number from 0 to 18446744073709551615",
            "budgets.always_loaded_skills_max must be a whole number",
        ]
    );
}

#[test]
fn options_are_true_or_false_and_true_when_absent() {
    let defaults = read(STACK).expect("valid");
    assert!(defaults.options().commit_generated() && defaults.options().strict_slots());
    let off = with("options:\n  commit_generated: false\n  strict_slots: false").expect("valid");
    assert!(!off.options().commit_generated() && !off.options().strict_slots());
    assert_eq!(
        messages(&with("options:\n  strict_slots: yes-please").expect_err("text")),
        ["options.strict_slots must be true or false"]
    );
}

#[test]
fn a_section_that_is_not_a_mapping_is_refused() {
    for field in ["mcp", "workspace", "budgets", "options"] {
        assert_eq!(
            messages(&with(&format!("{field}: [a]")).expect_err("not a mapping")),
            [format!("{field} must be a mapping")]
        );
    }
}

#[test]
fn every_error_of_one_file_is_reported_in_document_order() {
    let errors = refused(
        "targets: [x]\nstack:\n  - source: nope\n    ref: [a]\nbudgets:\n  instructions_max_lines: no\nbogus: 1\n",
    );
    assert_eq!(
        messages(&errors),
        [
            "targets[0] must be a target: claude or codex",
            "stack[0].source must start with path:, github: or git:",
            "stack[0].ref must be text",
            "budgets.instructions_max_lines must be a whole number",
            "unknown field 'bogus'",
        ]
    );
}

proptest! {
    #[test]
    fn no_error_holds_a_refused_ref_or_source(secret in "[A-Za-z0-9]{6,12}") {
        let documents = [
            format!("stack:\n  - source: path:./a\n    ref: \"-{secret}\"\n"),
            format!("stack:\n  - source: path:./a\n    ref: \"a..{secret}\"\n"),
            format!("stack:\n  - source: \"nope-{secret}\"\n"),
            format!("stack:\n  - source: \"git:https://u:{secret}@h/x@v1\"\n"),
            format!("stack:\n  - source: path:./a\ntargets: [{secret}]\n"),
            format!("stack:\n  - source: path:./a\nworkspace:\n  hub: /{secret}\n"),
        ];
        for document in documents {
            let errors = read(&document).expect_err("refused");
            let shown = errors
                .manifest_diagnostics()
                .to_string();
            prop_assert!(!shown.contains(&secret), "{shown}");
        }
    }
}

// ---- null counts as absent (spec 5.2) ----

#[test]
fn an_optional_field_or_section_that_is_null_counts_as_absent() {
    let nulls = with("targets:\nmcp:\nworkspace: ~\nprotected_paths: null\nbudgets:\noptions: ~")
        .expect("null optional fields are absent");
    assert_eq!(nulls, read(STACK).expect("the base manifest"));
    let inner = with(
        "workspace:\n  hub:\n  add_dirs: ~\nmcp:\n  enable:\nbudgets:\n  instructions_max_bytes:\noptions:\n  strict_slots: ~",
    )
    .expect("null fields inside a section are absent");
    assert_eq!(inner, read(STACK).expect("the base manifest"));
    let pin = read("stack:\n  - source: path:./a\n    ref:\n").expect("a null ref is absent");
    assert!(pin.stack().iter().all(|entry| entry.pin().is_none()));
}

#[test]
fn a_required_field_that_is_null_is_reported_as_missing() {
    assert_eq!(messages(&refused("stack:\n")), ["stack is required"]);
    assert_eq!(messages(&refused("stack: ~\n")), ["stack is required"]);
    assert_eq!(
        messages(&refused("stack:\n  - source:\n")),
        ["stack[0].source is required"]
    );
}
