//! Spec §8.0 **Output checks.**, **Coverage report.** and **Security items a target cannot
//! enforce.**: the core checks what an adapter returns, through the registry, before anything is
//! written. A toy adapter returns a fixed output, and the build is the error `target`.

use alz_adapter::{
    Adapter, Bytes, Content, Context, Coverage, CoverageEntry, DefaultModeFloor, DenyRule,
    Descriptor, Diagnostic as Returned, Enforcement, FloorField, FloorFieldItem, HookEvent,
    LayerName, Level, LockedHook, MODEL_VERSION, Mode, Model, NeutralEvent, OnFail, Output,
    OutputFile, SecurityItemId, SecurityItems, SettingLayers, TargetId, Territory, ToolVersion,
    ToolVersionRange,
};

use super::Registry;
use crate::diagnostic::Diagnostics;
use crate::render::tree::{FileTree, OutputPath};

/// An adapter for `toy` whose territory is `out/` and the files the core owns, so that only the
/// core's own rule keeps it from writing those, and whose output is fixed.
struct Toy {
    descriptor: Descriptor,
    output: Output,
}

fn toy_id() -> TargetId {
    TargetId::try_from("toy").expect("a target id")
}

fn toy(output: Output) -> Registry {
    let mut descriptor = Descriptor::new(
        toy_id(),
        MODEL_VERSION,
        ToolVersionRange::new(ToolVersion::new(1, 0, 0), ToolVersion::new(2, 0, 0)),
        "docs/toy-facts.md",
    );
    descriptor.territory = Territory::new(
        ["AGENTS.md", "layer.yaml", "layer.lock", "toy.md"]
            .map(str::to_owned)
            .to_vec(),
        ["out", ".layer", ".git"].map(str::to_owned).to_vec(),
    );
    Registry::new()
        .with(Box::new(Toy { descriptor, output }))
        .expect("a toy adapter registers")
}

impl Adapter for Toy {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    fn render(&self, _model: &Model, _context: &Context) -> Output {
        self.output.clone()
    }
}

fn layer(name: &str) -> LayerName {
    LayerName::try_from(name).expect("a layer name")
}

fn bytes_at(path: &str) -> OutputFile {
    OutputFile::new(path, Content::Bytes(Bytes::new(vec![1])))
}

fn files(paths: &[&str]) -> Output {
    Output::new(
        paths.iter().copied().map(bytes_at).collect(),
        Vec::new(),
        Coverage::default(),
    )
}

fn render(registry: &Registry, model: &Model) -> (FileTree, Diagnostics) {
    registry
        .render(&toy_id(), model, &Context::default())
        .expect("the toy adapter is registered")
}

fn plain(registry: &Registry) -> (FileTree, Diagnostics) {
    render(registry, &Model::new(toy_id()))
}

fn has(tree: &FileTree, path: &str) -> bool {
    OutputPath::try_from(path).is_ok_and(|path| tree.get(&path).is_some())
}

/// Both a file inside the territory and the one under test: the output of a build that must
/// write nothing, because the second file is refused.
fn refused_path(path: &str) -> (FileTree, String) {
    let (tree, found) = plain(&toy(files(&["out/fine.bin", path])));
    assert!(found.has_errors(), "{path}: {found}");
    (tree, found.to_string())
}

fn assert_refused_with(path: &str, rule: &str) {
    let (tree, text) = refused_path(path);
    assert!(
        text.starts_with(&format!(
            "error [target] the 'toy' adapter returned output that is refused: the path {path} {rule}"
        )),
        "{text}"
    );
    assert!(!has(&tree, path), "{path} is not written");
}

#[test]
fn a_file_in_the_territory_is_written() {
    let (tree, found) = plain(&toy(files(&["out/a/b.bin", "toy.md"])));
    assert!(found.to_string().is_empty(), "{found}");
    assert!(has(&tree, "out/a/b.bin") && has(&tree, "toy.md"));
}

#[test]
fn a_path_outside_the_territory_is_the_error_target() {
    assert_refused_with("elsewhere/x.bin", "is outside the adapter's territory");
}

#[test]
fn a_folder_of_the_territory_is_not_a_file_of_it() {
    assert_refused_with("out", "is outside the adapter's territory");
}

#[test]
fn a_folder_that_only_starts_like_a_territory_folder_is_outside_it() {
    assert_refused_with("outside/x.bin", "is outside the adapter's territory");
}

#[test]
fn agents_md_is_written_by_the_core_only() {
    assert_refused_with("AGENTS.md", "is written by the core only");
}

#[test]
fn agents_md_in_another_case_is_written_by_the_core_only() {
    assert_refused_with("agents.MD", "is written by the core only");
}

#[test]
fn layer_yaml_is_written_by_the_core_only() {
    assert_refused_with("layer.yaml", "is written by the core only");
}

#[test]
fn layer_lock_is_written_by_the_core_only() {
    assert_refused_with("layer.lock", "is written by the core only");
}

#[test]
fn a_path_below_the_layer_folder_is_written_by_the_core_only() {
    assert_refused_with(".layer/x", "is written by the core only");
}

#[test]
fn a_path_below_the_git_folder_is_the_error_target() {
    let (tree, text) = refused_path(".git/x");
    assert!(
        text.starts_with("error [target] the 'toy' adapter returned output that is refused: the path .git/x is not allowed"),
        "{text}"
    );
    assert!(!has(&tree, ".git/x"));
}

#[test]
fn a_path_in_the_message_is_stripped() {
    let (_, text) = refused_path("elsewhere/\u{1b}[31mx\u{202e}.bin");
    assert!(text.contains("elsewhere/[31mx.bin"), "{text:?}");
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{202e}'),
        "{text:?}"
    );
}

#[test]
fn more_files_than_the_limit_is_the_error_target_and_no_file_is_written() {
    let many = (0..10_001)
        .map(|n| bytes_at(&format!("out/{n}.bin")))
        .collect();
    let (tree, found) = plain(&toy(Output::new(many, Vec::new(), Coverage::default())));
    assert!(tree.is_empty());
    assert_eq!(
        found.to_string(),
        "error [target] the 'toy' adapter returned output that is refused: it returned 10001 files, more than the limit of 10000\n"
    );
}

#[test]
fn exactly_the_file_limit_is_within_it() {
    let many = (0..10_000)
        .map(|n| bytes_at(&format!("out/{n}.bin")))
        .collect();
    let (tree, found) = plain(&toy(Output::new(many, Vec::new(), Coverage::default())));
    assert_eq!(tree.len(), 10_000);
    assert!(!found.has_errors(), "{found}");
}

#[test]
fn a_file_of_more_than_16_mib_is_the_error_target() {
    let big = |size: usize| {
        Output::new(
            vec![OutputFile::new(
                "out/big.bin",
                Content::Bytes(Bytes::new(vec![0; size])),
            )],
            Vec::new(),
            Coverage::default(),
        )
    };
    let (_, within) = plain(&toy(big(16 * 1024 * 1024)));
    assert!(!within.has_errors(), "{within}");
    let (tree, found) = plain(&toy(big(16 * 1024 * 1024 + 1)));
    assert!(tree.is_empty());
    assert_eq!(
        found.to_string(),
        "error [target] the 'toy' adapter returned output that is refused: the file out/big.bin is too large: it holds 16777217 bytes, more than the limit of 16777216 bytes for a file\n"
    );
}

#[test]
fn a_diagnostic_code_of_another_namespace_is_the_error_target() {
    let returned = |code: &str| {
        Output::new(
            Vec::new(),
            vec![Returned::new(Level::Warning, code, "text")],
            Coverage::default(),
        )
    };
    for code in [
        "claude-agent",
        "yaml",
        "target",
        "collision",
        "toy-unknown",
        "",
    ] {
        let (_, found) = plain(&toy(returned(code)));
        assert!(
            found.to_string().starts_with(&format!(
                "error [target] the 'toy' adapter returned the diagnostic code {code}, which is not one of its codes"
            )),
            "{code}: {found}"
        );
    }
}

#[test]
fn the_shared_adapter_codes_are_accepted_from_any_adapter() {
    for code in ["models", "tool-scope", "hook-on-fail"] {
        let output = Output::new(
            Vec::new(),
            vec![Returned::new(Level::Warning, code, "text")],
            Coverage::default(),
        );
        let (_, found) = plain(&toy(output));
        assert!(!found.has_errors(), "{code}: {found}");
        assert_eq!(found.warning_count(), 1);
    }
}

#[test]
fn a_message_with_control_characters_is_stripped() {
    let output = Output::new(
        Vec::new(),
        vec![Returned::new(
            Level::Warning,
            "models",
            "a\u{1b}[2Jb\r\nc\u{7f}d\u{2028}e",
        )],
        Coverage::default(),
    );
    let (_, found) = plain(&toy(output));
    assert_eq!(found.to_string(), "warning [models] a[2Jbcde\n");
}

// Coverage, one test per kind of security item (§8.0 **Model.**).

fn items() -> SecurityItems {
    let mut items = SecurityItems::default();
    items.deny_rules = vec![DenyRule::new(
        "read:.env*",
        SettingLayers::new(layer("personal")).and(layer("repo")),
    )];
    items.floor_fields = vec![FloorFieldItem::new(
        FloorField::BlockWritesOutside,
        SettingLayers::new(layer("client")),
    )];
    items.default_mode_floor = Some(DefaultModeFloor::new(
        Mode::Default,
        SettingLayers::new(layer("personal")),
    ));
    items.locked_hooks = vec![LockedHook::new(
        "block-secrets",
        HookEvent::Neutral(NeutralEvent::Stop),
        OnFail::Deny,
        layer("repo"),
    )];
    items
}

fn model() -> Model {
    let mut model = Model::new(toy_id());
    model.security = items();
    model
}

fn report(entries: Vec<(SecurityItemId, Enforcement)>) -> Output {
    Output::new(
        Vec::new(),
        Vec::new(),
        Coverage::new(
            entries
                .into_iter()
                .map(|(item, enforcement)| CoverageEntry::new(item, enforcement))
                .collect(),
        ),
    )
}

fn deny() -> SecurityItemId {
    SecurityItemId::DenyRule("read:.env*".to_owned())
}

fn floor() -> SecurityItemId {
    SecurityItemId::FloorField(FloorField::BlockWritesOutside)
}

fn mode() -> SecurityItemId {
    SecurityItemId::DefaultModeFloor
}

fn hook() -> SecurityItemId {
    SecurityItemId::LockedHook("block-secrets".to_owned())
}

fn all(enforcement: Enforcement) -> Vec<(SecurityItemId, Enforcement)> {
    [deny(), floor(), mode(), hook()]
        .into_iter()
        .map(|item| (item, enforcement))
        .collect()
}

fn without(skip: &SecurityItemId) -> Vec<(SecurityItemId, Enforcement)> {
    all(Enforcement::Expressed)
        .into_iter()
        .filter(|(item, _)| item != skip)
        .collect()
}

const REFUSED: &str =
    "error [target] the 'toy' adapter returned output that is refused: the coverage report";

fn one_error(entries: Vec<(SecurityItemId, Enforcement)>) -> String {
    let (_, found) = render(&toy(report(entries)), &model());
    assert_eq!(found.error_count(), 1, "{found}");
    found.to_string()
}

#[test]
fn a_report_that_expresses_every_item_passes() {
    let (_, found) = render(&toy(report(all(Enforcement::Expressed))), &model());
    assert!(found.to_string().is_empty(), "{found}");
}

#[test]
fn an_approximated_item_passes() {
    let (_, found) = render(&toy(report(all(Enforcement::Approximated))), &model());
    assert!(found.to_string().is_empty(), "{found}");
}

#[test]
fn a_deny_rule_missing_from_the_report_is_the_error_target() {
    assert_eq!(
        one_error(without(&deny())),
        format!("{REFUSED} leaves out the deny rule 'read:.env*'\n")
    );
}

#[test]
fn a_floor_field_missing_from_the_report_is_the_error_target() {
    assert_eq!(
        one_error(without(&floor())),
        format!("{REFUSED} leaves out the floor field block_writes_outside\n")
    );
}

#[test]
fn the_default_mode_floor_missing_from_the_report_is_the_error_target() {
    assert_eq!(
        one_error(without(&mode())),
        format!("{REFUSED} leaves out the default_mode floor\n")
    );
}

#[test]
fn a_locked_hook_missing_from_the_report_is_the_error_target() {
    assert_eq!(
        one_error(without(&hook())),
        format!("{REFUSED} leaves out the locked hook 'block-secrets'\n")
    );
}

#[test]
fn an_item_named_twice_is_the_error_target_for_each_kind() {
    for (item, text) in [
        (deny(), "the deny rule 'read:.env*'"),
        (floor(), "the floor field block_writes_outside"),
        (mode(), "the default_mode floor"),
        (hook(), "the locked hook 'block-secrets'"),
    ] {
        let mut entries = all(Enforcement::Expressed);
        entries.push((item, Enforcement::Expressed));
        assert_eq!(
            one_error(entries),
            format!("{REFUSED} names twice {text}\n")
        );
    }
}

#[test]
fn an_item_that_the_model_does_not_hold_is_the_error_target() {
    let mut entries = all(Enforcement::Expressed);
    entries.push((
        SecurityItemId::DenyRule("write:\u{1b}x".to_owned()),
        Enforcement::Expressed,
    ));
    assert_eq!(
        one_error(entries),
        format!("{REFUSED} names the deny rule 'write:x', which the model does not hold\n")
    );
}

#[test]
fn an_empty_report_for_a_model_without_items_passes() {
    let (_, found) = plain(&toy(files(&[])));
    assert!(found.to_string().is_empty(), "{found}");
}

fn not_expressed(item: &SecurityItemId) -> String {
    let entries = all(Enforcement::Expressed)
        .into_iter()
        .map(|(known, how)| {
            if known == *item {
                (known, Enforcement::NotExpressed)
            } else {
                (known, how)
            }
        })
        .collect();
    let (_, found) = render(&toy(report(entries)), &model());
    found.to_string()
}

#[test]
fn a_deny_rule_that_is_not_expressed_is_the_error_target_once_for_each_layer() {
    assert_eq!(
        not_expressed(&deny()),
        "error [target] personal:permissions.yaml: the 'toy' adapter cannot express the deny rule 'read:.env*' of layer personal; remove the target, or list the rule under unenforced.toy.deny in that layer's permissions.yaml\n\
         error [target] repo:permissions.yaml: the 'toy' adapter cannot express the deny rule 'read:.env*' of layer repo; remove the target, or list the rule under unenforced.toy.deny in that layer's permissions.yaml\n"
    );
}

#[test]
fn a_floor_field_that_is_not_expressed_is_the_error_target() {
    assert_eq!(
        not_expressed(&floor()),
        "error [target] client:permissions.yaml: the 'toy' adapter cannot express the floor field block_writes_outside of layer client; remove the target, or list the field under unenforced.toy.floor_fields in that layer's permissions.yaml\n"
    );
}

#[test]
fn the_default_mode_floor_that_is_not_expressed_is_the_error_target() {
    assert_eq!(
        not_expressed(&mode()),
        "error [target] personal:permissions.yaml: the 'toy' adapter cannot express the default_mode floor default of layer personal; remove the target, or set unenforced.toy.default_mode to true in that layer's permissions.yaml\n"
    );
}

#[test]
fn a_locked_hook_that_is_not_expressed_is_the_error_target() {
    assert_eq!(
        not_expressed(&hook()),
        "error [target] repo:permissions.yaml: the 'toy' adapter cannot express the locked hook 'block-secrets' of layer repo; remove the target, or list the hook under unenforced.toy.hooks in that layer's permissions.yaml\n"
    );
}

#[test]
fn the_claude_adapter_reports_every_kind_of_item_as_expressed() {
    let claude = crate::target::claude();
    let mut model = Model::new(claude.clone());
    model.security = items();
    let output = Registry::builtin()
        .adapter(claude)
        .expect("the claude adapter")
        .render(&model, &Context::default());
    let reported: Vec<_> = output
        .coverage
        .entries()
        .iter()
        .map(|entry| (entry.item.clone(), entry.enforcement))
        .collect();
    assert_eq!(reported, all(Enforcement::Expressed));
    let (_, found) = Registry::builtin()
        .render(claude, &model, &Context::default())
        .expect("rendered");
    assert!(!found.has_errors(), "{found}");
}
