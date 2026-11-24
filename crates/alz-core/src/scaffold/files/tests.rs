//! The bytes of every scaffolded file against spec §10.3: each expected text is
//! written out in full.

use super::{MISE_TASKS, RepoOptions, RepoSurroundings, Scaffolded, layer_files, repo_files};
use crate::layer::{LayerName, Visibility};
use crate::scaffold::{Hub, stack_entry};

const EXPECTED_MANIFEST_WITH_STACK_AND_HUB: &str = "stack:
  - source: path:../p
    ref: v1
  - source: github:a/b@v2
  - source: path:./.layer   # the repo layer, always last
targets: [claude]
workspace:
  hub: ../hub             # granted to agents as an additional directory
# protected_paths: []       # extra globs the build never touches
mcp:
  enable: []
budgets:
  instructions_max_lines: 400
options:
  commit_generated: true
  strict_slots: true
";

const EXPECTED_MANIFEST_BARE: &str = "stack:
  # Most general first, for example:
  # - source: github:<you>/agent-layerz-personal@v1.0.0
  - source: path:./.layer   # the repo layer, always last
targets: [claude]
# workspace:
#   hub: ../hub             # granted to agents as an additional directory
# protected_paths: []       # extra globs the build never touches
mcp:
  enable: []
budgets:
  instructions_max_lines: 400
options:
  commit_generated: true
  strict_slots: true
";

const FRESH: RepoSurroundings = RepoSurroundings {
    repo_layer_exists: false,
    mise_config_exists: false,
};

fn name(text: &str) -> LayerName {
    LayerName::try_from(text).expect("a layer name")
}

fn listed(scaffolded: &Scaffolded) -> Vec<(&str, &str)> {
    scaffolded
        .files()
        .iter()
        .map(|file| (file.path().as_str(), file.text()))
        .collect()
}

#[test]
fn a_repository_with_a_stack_and_a_hub_has_the_expected_bytes() {
    let stack = [
        stack_entry("../p@v1").expect("an entry"),
        stack_entry("github:a/b@v2").expect("an entry"),
    ];
    let hub = Hub::new("../hub").expect("a hub");
    let scaffolded = repo_files(
        RepoOptions {
            name: &name("my-repo"),
            visibility: Visibility::Private,
            stack: &stack,
            hub: Some(&hub),
        },
        FRESH,
    );
    assert_eq!(
        listed(&scaffolded),
        [
            ("layer.yaml", EXPECTED_MANIFEST_WITH_STACK_AND_HUB),
            (
                ".layer/layer.yaml",
                "name: my-repo\nkind: repo\nvisibility: private\nversion: 0.1.0\n"
            ),
            (
                ".layer/instructions/50-repo.md",
                "- Describe this repo's commands, paths and contracts here.\n"
            ),
            ("mise.toml", MISE_TASKS),
        ]
    );
    assert_eq!(scaffolded.mise_advice(), None);
}

#[test]
fn a_repository_without_options_has_the_example_manifest() {
    let scaffolded = repo_files(
        RepoOptions {
            name: &name("r"),
            visibility: Visibility::Private,
            stack: &[],
            hub: None,
        },
        FRESH,
    );
    assert_eq!(
        scaffolded.files().first().map(super::ScaffoldFile::text),
        Some(EXPECTED_MANIFEST_BARE)
    );
}

#[test]
fn the_mise_tasks_are_the_expected_ones() {
    assert_eq!(
        MISE_TASKS,
        "[tasks.\"alz:build\"]\nrun = \"alz build\"\n\n[tasks.\"alz:check\"]\nrun = \"alz check\"\n\n[tasks.\"alz:diff\"]\nrun = \"alz diff\"\n\n[tasks.\"alz:doctor\"]\nrun = \"alz doctor\"\n"
    );
}

#[test]
fn an_existing_repo_layer_and_an_existing_mise_config_are_not_written_again() {
    let scaffolded = repo_files(
        RepoOptions {
            name: &name("r"),
            visibility: Visibility::Private,
            stack: &[],
            hub: None,
        },
        RepoSurroundings {
            repo_layer_exists: true,
            mise_config_exists: true,
        },
    );
    let paths: Vec<&str> = scaffolded
        .files()
        .iter()
        .map(|file| file.path().as_str())
        .collect();
    assert_eq!(paths, ["layer.yaml"]);
    assert_eq!(
        scaffolded.mise_advice(),
        Some(format!("add these tasks to your mise config:\n\n{MISE_TASKS}\n").as_str())
    );
}

#[test]
fn a_layer_has_the_two_expected_files() {
    let scaffolded = layer_files(&name("x.y"), Visibility::Confidential);
    assert_eq!(
        listed(&scaffolded),
        [
            (
                "layer.yaml",
                "name: x.y\nkind: layer\nvisibility: confidential\nversion: 0.1.0\n"
            ),
            (
                "instructions/00-core.md",
                "- Write the standards of this layer here.\n"
            ),
        ]
    );
}
