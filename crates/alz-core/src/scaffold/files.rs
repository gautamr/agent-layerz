//! The files `alz init` creates and their text (spec §10.3), byte for byte as the TypeScript `alz`
//! writes them.

use std::sync::LazyLock;

use crate::layer::{LayerName, Visibility};
use crate::path::SafeRelPath;
use crate::scaffold::hub::Hub;
use crate::stack::StackEntry;

/// The `alz:*` tasks of a new `mise.toml`.
pub const MISE_TASKS: &str = "[tasks.\"alz:build\"]
run = \"alz build\"

[tasks.\"alz:check\"]
run = \"alz check\"

[tasks.\"alz:diff\"]
run = \"alz diff\"

[tasks.\"alz:doctor\"]
run = \"alz doctor\"
";

/// The line of a new repo layer's instructions.
const REPO_INSTRUCTIONS: &str = "- Describe this repo's commands, paths and contracts here.\n";

/// The line of a new layer's instructions.
const LAYER_INSTRUCTIONS: &str = "- Write the standards of this layer here.\n";

/// What `alz init --kind repo` found in the folder before writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepoSurroundings {
    /// A `.layer/layer.yaml` is there: the repo layer is not written again.
    pub repo_layer_exists: bool,
    /// A `mise.toml` or `.mise.toml` is there: the tasks are printed and not written.
    pub mise_config_exists: bool,
}

/// What decides the text of a new repository's manifest and repo layer.
#[derive(Debug, Clone, Copy)]
pub struct RepoOptions<'a> {
    /// The repo layer's name.
    pub name: &'a LayerName,
    /// The repo layer's visibility.
    pub visibility: Visibility,
    /// The layers below the repo layer, most general first.
    pub stack: &'a [StackEntry],
    /// The hub, when `--hub` was given.
    pub hub: Option<&'a Hub>,
}

/// One file to create: a path below the repository root and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldFile {
    path: SafeRelPath,
    text: String,
}

impl ScaffoldFile {
    /// A file at a path written in this module, which is safe by construction.
    fn constant(path: &'static str, text: String) -> Self {
        Self {
            path: constant(path),
            text,
        }
    }

    /// The path below the repository root.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The text of the file.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// What a scaffold gives: the files to create, in the order they are listed, and what to print
/// when the mise tasks could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaffolded {
    files: Vec<ScaffoldFile>,
    mise_advice: Option<String>,
}

impl Scaffolded {
    /// The files to create, in order.
    #[must_use]
    pub fn files(&self) -> &[ScaffoldFile] {
        &self.files
    }

    /// The text that asks for the tasks to be added to the mise config that exists, if it does.
    #[must_use]
    pub fn mise_advice(&self) -> Option<&str> {
        self.mise_advice.as_deref()
    }
}

/// The paths `alz init` must look at before it plans anything.
fn constant(path: &'static str) -> SafeRelPath {
    SafeRelPath::try_from(path).expect("a constant safe path")
}

static MANIFEST: LazyLock<SafeRelPath> = LazyLock::new(|| constant("layer.yaml"));
static REPO_LAYER: LazyLock<SafeRelPath> = LazyLock::new(|| constant(".layer/layer.yaml"));
static MISE_CONFIGS: LazyLock<[SafeRelPath; 2]> =
    LazyLock::new(|| [constant("mise.toml"), constant(".mise.toml")]);

/// The path of `layer.yaml`, which `alz init` never writes over.
#[must_use]
pub fn manifest_path() -> &'static SafeRelPath {
    &MANIFEST
}

/// The path of the repo layer's `layer.yaml`.
#[must_use]
pub fn repo_layer_path() -> &'static SafeRelPath {
    &REPO_LAYER
}

/// The names of a mise config.
#[must_use]
pub fn mise_config_paths() -> &'static [SafeRelPath; 2] {
    &MISE_CONFIGS
}

/// `layer.yaml` of a repository, the repo layer when `.layer/layer.yaml` does not exist, and
/// `mise.toml` when there is no mise config.
#[must_use]
pub fn repo_files(options: RepoOptions<'_>, found: RepoSurroundings) -> Scaffolded {
    let manifest = ScaffoldFile::constant("layer.yaml", manifest(options.stack, options.hub));
    let repo_layer = (!found.repo_layer_exists)
        .then(|| {
            [
                ScaffoldFile::constant(
                    ".layer/layer.yaml",
                    layer_meta(options.name, "repo", options.visibility),
                ),
                ScaffoldFile::constant(
                    ".layer/instructions/50-repo.md",
                    REPO_INSTRUCTIONS.to_owned(),
                ),
            ]
        })
        .into_iter()
        .flatten();
    let mise = (!found.mise_config_exists)
        .then(|| ScaffoldFile::constant("mise.toml", MISE_TASKS.to_owned()));
    Scaffolded {
        files: std::iter::once(manifest)
            .chain(repo_layer)
            .chain(mise)
            .collect(),
        mise_advice: found
            .mise_config_exists
            .then(|| format!("add these tasks to your mise config:\n\n{MISE_TASKS}\n")),
    }
}

/// `layer.yaml` and `instructions/00-core.md` of a new layer.
#[must_use]
pub fn layer_files(name: &LayerName, visibility: Visibility) -> Scaffolded {
    Scaffolded {
        files: [
            ScaffoldFile::constant("layer.yaml", layer_meta(name, "layer", visibility)),
            ScaffoldFile::constant("instructions/00-core.md", LAYER_INSTRUCTIONS.to_owned()),
        ]
        .into(),
        mise_advice: None,
    }
}

fn layer_meta(name: &LayerName, kind: &str, visibility: Visibility) -> String {
    format!(
        "name: {name}\nkind: {kind}\nvisibility: {}\nversion: 0.1.0\n",
        visibility.as_str()
    )
}

/// The lines of one stack entry: its source, and its pin when it has one.
fn entry_lines(entry: &StackEntry) -> String {
    let pin = entry
        .pin()
        .map_or_else(String::new, |pin| format!("    ref: {pin}\n"));
    format!("  - source: {}\n{pin}", entry.source())
}

/// The repo manifest: the given layers, then the repo layer, which is always last.
fn manifest(stack: &[StackEntry], hub: Option<&Hub>) -> String {
    let entries: String = stack.iter().map(entry_lines).collect();
    let example = if stack.is_empty() {
        "  # Most general first, for example:\n  # - source: github:<you>/agent-layerz-personal@v1.0.0\n"
    } else {
        ""
    };
    let workspace = hub.map_or_else(
        || {
            "# workspace:\n#   hub: ../hub             # granted to agents as an additional directory\n"
                .to_owned()
        },
        |hub| {
            format!(
                "workspace:\n  hub: {}             # granted to agents as an additional directory\n",
                hub.as_str()
            )
        },
    );
    format!(
        "stack:\n{example}{entries}  - source: path:./.layer   # the repo layer, always last\ntargets: [claude]\n{workspace}# protected_paths: []       # extra globs the build never touches\nmcp:\n  enable: []\nbudgets:\n  instructions_max_lines: 400\noptions:\n  commit_generated: true\n  strict_slots: true\n"
    )
}

#[cfg(test)]
mod tests;
