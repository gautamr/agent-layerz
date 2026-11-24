use proptest::collection::vec;
use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};

use super::{AgentsMd, agents_md_path, claude_md, claude_md_path};
use crate::lint::parse_document;
use crate::merge::testing::{layer_pinned, merged_placed};
use crate::merge::{Merged, Merging};
use crate::stack::{Budgets, Manifest};

/// A file of the acceptance fixtures, read at compile time.
macro_rules! fixture {
    ($path:literal) => {
        include_str!(concat!("../../../../../test/fixtures/", $path))
    };
}

/// The reference's output for a fixture repository, kept in `tests/fixtures/instructions`.
macro_rules! golden {
    ($path:literal) => {
        include_str!(concat!("../../../tests/fixtures/instructions/", $path))
    };
}

/// A layer as `(layer.yaml, other files, pin)`.
type Spec<'a> = (&'a str, &'a [(&'a str, &'a str)], Option<&'a str>);

/// The merged model of a stack; the layers named in `inside` lie in the repository.
fn model(specs: &[Spec<'_>], inside: &[&str]) -> Merged {
    let found: Merging = merged_placed(
        specs
            .iter()
            .map(|(yaml, files, rev)| layer_pinned(yaml, files, *rev))
            .collect(),
        inside,
    );
    assert!(!found.diagnostics().has_errors(), "{}", found.diagnostics());
    found.merged().expect("no error was found").clone()
}

fn agents(merged: &Merged) -> String {
    AgentsMd::compose(merged)
        .expect("a header without a bad source")
        .into_text()
}

fn claude(merged: &Merged) -> String {
    claude_md(merged).expect("a header without a bad source")
}

fn case_02() -> Merged {
    model(
        &[
            (
                fixture!("02-instructions/personal/layer.yaml"),
                &[
                    (
                        "instructions/00-core.md",
                        fixture!("02-instructions/personal/instructions/00-core.md"),
                    ),
                    (
                        "instructions/10-style.md",
                        fixture!("02-instructions/personal/instructions/10-style.md"),
                    ),
                ],
                None,
            ),
            (
                fixture!("02-instructions/client/layer.yaml"),
                &[(
                    "instructions/10-domain.md",
                    fixture!("02-instructions/client/instructions/10-domain.md"),
                )],
                None,
            ),
            (
                fixture!("02-instructions/repo/.layer/layer.yaml"),
                &[
                    (
                        "instructions/10-style.md",
                        fixture!("02-instructions/repo/.layer/instructions/10-style.md"),
                    ),
                    (
                        "instructions/50-repo.md",
                        fixture!("02-instructions/repo/.layer/instructions/50-repo.md"),
                    ),
                ],
                None,
            ),
        ],
        &["shop-api"],
    )
}

fn case_01() -> Merged {
    model(
        &[
            (
                fixture!("01-determinism/personal/layer.yaml"),
                &[(
                    "instructions/00-core.md",
                    fixture!("01-determinism/personal/instructions/00-core.md"),
                )],
                None,
            ),
            (
                fixture!("01-determinism/client/layer.yaml"),
                &[(
                    "instructions/10-domain.md",
                    fixture!("01-determinism/client/instructions/10-domain.md"),
                )],
                None,
            ),
            (
                fixture!("01-determinism/repo/.layer/layer.yaml"),
                &[
                    (
                        "instructions/50-repo.md",
                        fixture!("01-determinism/repo/.layer/instructions/50-repo.md"),
                    ),
                    (
                        "targets/claude/CLAUDE.extra.md",
                        fixture!("01-determinism/repo/.layer/targets/claude/CLAUDE.extra.md"),
                    ),
                    (
                        "skills/stack-conventions/SKILL.md",
                        fixture!("01-determinism/repo/.layer/skills/stack-conventions/SKILL.md"),
                    ),
                ],
                None,
            ),
        ],
        &["shop-api"],
    )
}

/// Fixture 16 with the hub pinned at `layer-v0.1.0`: the repo's layer file is `repo-a` or `repo-b`.
fn case_16(repo_yaml: &'static str, repo_file: &'static str) -> Merged {
    model(
        &[
            (
                fixture!("16-hub/personal/layer.yaml"),
                &[(
                    "instructions/00-core.md",
                    fixture!("16-hub/personal/instructions/00-core.md"),
                )],
                None,
            ),
            (
                fixture!("16-hub/hub/layer/layer.yaml"),
                &[(
                    "instructions/10-domain.md",
                    fixture!("16-hub/hub/layer/instructions/10-domain.md"),
                )],
                Some("layer-v0.1.0"),
            ),
            (repo_yaml, &[("instructions/50-repo.md", repo_file)], None),
        ],
        &[],
    )
}

fn case_26() -> Merged {
    model(
        &[
            (
                fixture!("26-commit-identity/personal/layer.yaml"),
                &[(
                    "instructions/00-core.md",
                    fixture!("26-commit-identity/personal/instructions/00-core.md"),
                )],
                None,
            ),
            (
                fixture!("26-commit-identity/client/layer.yaml"),
                &[(
                    "instructions/10-client.md",
                    fixture!("26-commit-identity/client/instructions/10-client.md"),
                )],
                None,
            ),
            (
                fixture!("26-commit-identity/repo/.layer/layer.yaml"),
                &[(
                    "instructions/50-repo.md",
                    fixture!("26-commit-identity/repo/.layer/instructions/50-repo.md"),
                )],
                None,
            ),
        ],
        &["repo"],
    )
}

fn budgets(lines: Option<u64>, bytes: Option<u64>) -> Budgets {
    let yaml = format!(
        "stack:\n  - source: path:./.layer\nbudgets:\n{}{}",
        lines.map_or_else(String::new, |n| format!("  instructions_max_lines: {n}\n")),
        bytes.map_or_else(String::new, |n| format!("  instructions_max_bytes: {n}\n")),
    );
    *Manifest::from_yaml(&parse_document(yaml.as_bytes()).expect("valid YAML"))
        .expect("a manifest")
        .budgets()
}

// AC 1: the reference's bytes. Fixture 02 has its own expected files; the others were made with
// the reference's `alz build` (tests/fixtures/instructions).

#[test]
fn fixture_02_matches_the_expected_files_byte_for_byte() {
    let merged = case_02();
    assert_eq!(
        agents(&merged),
        fixture!("02-instructions/expected/AGENTS.md")
    );
    assert_eq!(
        claude(&merged),
        fixture!("02-instructions/expected/CLAUDE.md")
    );
}

#[test]
fn fixture_01_matches_the_reference_with_the_claude_extra() {
    let merged = case_01();
    assert_eq!(agents(&merged), golden!("01-determinism/AGENTS.md"));
    assert_eq!(claude(&merged), golden!("01-determinism/CLAUDE.md"));
}

#[test]
fn fixture_16_matches_the_reference_with_the_hub_label_of_its_ref() {
    let repo_a = case_16(
        fixture!("16-hub/repo-a/.layer/layer.yaml"),
        fixture!("16-hub/repo-a/.layer/instructions/50-repo.md"),
    );
    assert_eq!(agents(&repo_a), golden!("16-hub-repo-a/AGENTS.md"));
    assert_eq!(claude(&repo_a), golden!("16-hub-repo-a/CLAUDE.md"));
    let repo_b = case_16(
        fixture!("16-hub/repo-b/.layer/layer.yaml"),
        fixture!("16-hub/repo-b/.layer/instructions/50-repo.md"),
    );
    assert_eq!(agents(&repo_b), golden!("16-hub-repo-b/AGENTS.md"));
    assert_eq!(claude(&repo_b), golden!("16-hub-repo-b/CLAUDE.md"));
}

#[test]
fn fixture_26_matches_the_reference_with_the_commits_section_of_the_client() {
    let merged = case_26();
    assert_eq!(agents(&merged), golden!("26-commit-identity/AGENTS.md"));
    assert_eq!(claude(&merged), golden!("26-commit-identity/CLAUDE.md"));
}

#[test]
fn the_commits_section_names_the_layer_that_declares_and_its_layer_yaml_in_the_header() {
    let text = agents(&case_26());
    assert!(text.contains("Source: personal@1.0.0:instructions/00-core.md"));
    assert!(text.contains("+ client@0.1.0:layer.yaml. Edit the source"));
    assert!(text.contains("`Owner Example <owner@client.example>` (declared by layer `client`)"));
}

#[test]
fn a_stack_without_instructions_names_no_instructions() {
    let merged = model(
        &[("name: only\nkind: repo\nvisibility: private\n", &[], None)],
        &["only"],
    );
    assert_eq!(
        agents(&merged),
        "<!-- GENERATED by agent-layerz \u{2014} DO NOT EDIT. Source: no instructions. Edit the source and run `alz build`. -->\n"
    );
    assert_eq!(
        claude(&merged),
        fixture!("02-instructions/expected/CLAUDE.md")
    );
}

#[test]
fn the_extras_of_every_layer_follow_the_claude_specific_heading_in_stack_order() {
    let extra = |body: &'static str| -> &'static [(&'static str, &'static str)] {
        Box::leak(Box::new([("targets/claude/CLAUDE.extra.md", body)]))
    };
    let merged = model(
        &[
            (
                "name: a\nkind: personal\nvisibility: private\nversion: 2.0.0\n",
                extra("  First.  \n"),
                None,
            ),
            (
                "name: b\nkind: repo\nvisibility: private\n",
                extra("Second."),
                None,
            ),
        ],
        &["b"],
    );
    assert_eq!(
        claude(&merged),
        "<!-- GENERATED by agent-layerz \u{2014} DO NOT EDIT. Source: AGENTS.md + a@2.0.0:targets/claude/CLAUDE.extra.md + b:targets/claude/CLAUDE.extra.md. Edit the source and run `alz build`. -->\n\n@AGENTS.md\n\n## Claude-specific\n\nFirst.\n\nSecond.\n"
    );
}

#[test]
fn the_paths_of_the_two_files_are_the_repository_root() {
    assert_eq!(agents_md_path().as_str(), "AGENTS.md");
    assert_eq!(claude_md_path().as_str(), "CLAUDE.md");
}

// Untrusted text.

#[test]
fn a_version_that_would_end_the_header_comment_is_the_error_layer_naming_the_file() {
    let merged = model(
        &[(
            "name: p\nkind: personal\nvisibility: private\nversion: \"1.0 --> x\"\n",
            &[("instructions/00-core.md", "Rule.\n")],
            None,
        )],
        &[],
    );
    let found = AgentsMd::compose(&merged).expect_err("the source cannot stand in a comment");
    assert_eq!(
        found.to_string(),
        "error [layer] p:instructions/00-core.md: a source of the generated header holds a control or format character or '-->', which would end its comment; rename the file or change the version or ref\n"
    );
    assert!(claude_md(&merged).is_ok());
}

#[test]
fn a_kind_prints_in_the_heading_without_control_and_format_characters() {
    let merged = model(
        &[(
            "name: p\nkind: \"re\\u200Bpo\"\nvisibility: private\n",
            &[("instructions/00-core.md", "Rule.\n")],
            None,
        )],
        &[],
    );
    assert!(agents(&merged).contains("\n\n## Repo: p\n\nRule.\n"));
}

// AC 2: the budgets count the whole composed text, the Commits section included.

fn counts(text: &str) -> (u64, u64) {
    (
        u64::try_from(text.matches('\n').count()).expect("a count"),
        u64::try_from(text.len()).expect("a count"),
    )
}

#[test]
fn a_text_at_each_budget_is_within_it_and_one_over_is_the_error_budget() {
    let composed = AgentsMd::compose(&case_26()).expect("composes");
    let (lines, bytes) = counts(composed.text());
    assert!(
        composed
            .budget_problems(&budgets(Some(lines), Some(bytes)))
            .iter()
            .next()
            .is_none()
    );
    let over_lines = composed.budget_problems(&budgets(Some(lines - 1), Some(bytes)));
    assert_eq!(
        over_lines.to_string(),
        format!(
            "error [budget] AGENTS.md has {lines} lines; instructions_max_lines is {}. Move long standards into skills\n",
            lines - 1
        )
    );
    let over_bytes = composed.budget_problems(&budgets(Some(lines), Some(bytes - 1)));
    assert_eq!(
        over_bytes.to_string(),
        format!(
            "error [budget] AGENTS.md is {bytes} bytes; instructions_max_bytes is {}\n",
            bytes - 1
        )
    );
    assert_eq!(
        composed
            .budget_problems(&budgets(Some(lines - 1), Some(bytes - 1)))
            .error_count(),
        2
    );
}

#[test]
fn the_commits_section_counts_toward_both_budgets() {
    let composed = AgentsMd::compose(&case_26()).expect("composes");
    let without_section = composed
        .text()
        .split("\n\n## Commits")
        .next()
        .map(|before| format!("{before}\n"))
        .expect("a split has a first part");
    let (lines, bytes) = counts(&without_section);
    assert!(
        composed
            .budget_problems(&budgets(Some(lines), Some(bytes)))
            .has_errors(),
        "a limit that the text without its Commits section just meets is exceeded by the text"
    );
    assert_eq!(
        composed
            .budget_problems(&budgets(Some(lines), Some(bytes)))
            .error_count(),
        2
    );
}

#[test]
fn the_default_is_32768_bytes_and_no_line_limit() {
    let composed = AgentsMd::compose(&case_01()).expect("composes");
    assert!(
        composed
            .budget_problems(&Budgets::default())
            .iter()
            .next()
            .is_none()
    );
}

#[test]
fn lines_are_the_line_feeds_and_bytes_the_utf8_length() {
    let merged = model(
        &[(
            "name: p\nkind: repo\nvisibility: private\n",
            &[("instructions/00-core.md", "Caf\u{e9} \u{2014} ok\n")],
            None,
        )],
        &[],
    );
    let composed = AgentsMd::compose(&merged).expect("composes");
    let (lines, bytes) = counts(composed.text());
    assert_eq!(lines, 5);
    assert_eq!(
        usize::try_from(bytes).expect("a count"),
        composed.text().chars().map(char::len_utf8).sum::<usize>()
    );
    assert!(bytes > u64::try_from(composed.text().chars().count()).expect("a count"));
}

// AC 3: determinism.

fn layer_spec() -> impl Strategy<Value = (String, Vec<(String, String)>, Option<String>)> {
    (
        "[a-z]{1,8}",
        vec(("[0-9]{2}-[a-z]{1,6}", "[A-Za-z ,.]{1,24}"), 0..4),
        proptest::option::of("[A-Za-z ]{1,12}"),
    )
}

proptest! {
    #[test]
    fn rendering_is_deterministic_for_generated_stacks(
        layers in vec(layer_spec(), 1..4),
    ) {
        let build = || {
            let owned: Vec<(String, Vec<(String, String)>)> = layers
                .iter()
                .enumerate()
                .map(|(index, (kind, files, extra))| {
                    let yaml = format!("name: layer-{index}\nkind: {kind}x\nvisibility: private\n");
                    let files: Vec<(String, String)> = files
                        .iter()
                        .map(|(name, body)| (format!("instructions/{name}.md"), body.clone()))
                        .chain(extra.iter().map(|body| {
                            ("targets/claude/CLAUDE.extra.md".to_owned(), body.clone())
                        }))
                        .collect();
                    (yaml, files)
                })
                .collect();
            let borrowed: Vec<(&str, Vec<(&str, &str)>)> = owned
                .iter()
                .map(|(yaml, files)| {
                    (
                        yaml.as_str(),
                        files.iter().map(|(p, b)| (p.as_str(), b.as_str())).collect(),
                    )
                })
                .collect();
            let specs: Vec<Spec<'_>> = borrowed
                .iter()
                .map(|(yaml, files)| (*yaml, files.as_slice(), None))
                .collect();
            model(&specs, &[])
        };
        let (first, second) = (build(), build());
        prop_assert_eq!(agents(&first), agents(&second));
        prop_assert_eq!(claude(&first), claude(&second));
        prop_assert_eq!(agents(&first), agents(&first));
        prop_assert!(agents(&first).starts_with("<!-- GENERATED by agent-layerz"));
        prop_assert!(agents(&first).ends_with('\n') && !agents(&first).ends_with("\n\n"));
        prop_assert!(claude(&first).contains("\n\n@AGENTS.md"));
    }
}
