//! Adoption planning (spec §4.3, `alz build --adopt`): from the native files on disk, passed in as
//! the bytes observed, and the stack as it merges before adoption, the new files of the repo layer
//! and the edit of `layer.yaml`, all in memory. Nothing here touches the file system; the caller
//! observes the files and, when the plan is accepted, writes it.
//!
//! A plan is all or nothing. The planned repo layer goes through the merge every layer goes through
//! (the `check` module) with the manifest as it would be edited, so a plan never holds text that reading the
//! layer would refuse, and every problem is reported without printing the text of a native file.

mod agent;
mod check;
mod claude_md;
mod draft;
mod hooks;
mod manifest_edit;
mod mcp_json;
mod names;
mod natives;
mod settings_json;
mod values;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use thiserror::Error;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::merge::{Merged, Placements};
use crate::path::SafeRelPath;
use crate::stack::{Manifest, ResolvedStack};
use crate::untrusted_text::Stripped;

pub use natives::{ADOPT_ALWAYS, Candidates, candidates};

use draft::{Context, Draft};

/// A native file as the caller observed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFile {
    bytes: Vec<u8>,
    executable: bool,
}

impl NativeFile {
    /// A regular file with these bytes, executable or not.
    #[must_use]
    pub const fn new(bytes: Vec<u8>, executable: bool) -> Self {
        Self { bytes, executable }
    }

    /// The bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether the file has the owner-execute bit.
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        self.executable
    }
}

/// What the planner reads, all of it observed or resolved by the caller.
#[derive(Debug, Clone, Copy)]
pub struct AdoptionInput<'a> {
    /// The stack as resolved, the repo layer last.
    pub stack: &'a ResolvedStack,
    /// The repository's manifest.
    pub manifest: &'a Manifest,
    /// The bytes of the repository's `layer.yaml`.
    pub manifest_text: &'a [u8],
    /// Where each layer lies in relation to the repository.
    pub placements: &'a Placements,
    /// The stack merged before adoption.
    pub merged: &'a Merged,
    /// The native files to adopt, by path from the repository root: each collision of the build,
    /// and each file that is always adopted (`.claude/settings.json`, `.mcp.json`) that exists
    /// and that alz did not generate (§4.3 "Which files are alz's own").
    pub natives: &'a BTreeMap<SafeRelPath, NativeFile>,
    /// The bytes of `AGENTS.md` as it is on disk, when there is one: the paragraphs `CLAUDE.md`
    /// shares with it are not adopted twice.
    pub agents_md: Option<&'a [u8]>,
}

/// A native file moved byte for byte into the repo layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    from: SafeRelPath,
    to: SafeRelPath,
}

impl Move {
    /// The native file, from the repository root.
    #[must_use]
    pub const fn from(&self) -> &SafeRelPath {
        &self.from
    }

    /// Where it goes, from the repo layer's folder.
    #[must_use]
    pub const fn to(&self) -> &SafeRelPath {
        &self.to
    }
}

/// What one native file became, for the report: the paths that now hold it, from the repository
/// root, and none when the file held nothing to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adopted {
    from: String,
    to: Vec<String>,
}

impl Adopted {
    /// The native file.
    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }

    /// The files that hold it now.
    #[must_use]
    pub fn to(&self) -> &[String] {
        &self.to
    }
}

/// The edit of the repository's `layer.yaml`.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestEdit {
    text: String,
}

impl ManifestEdit {
    /// The new text of `layer.yaml`: every byte outside the added lines as it was.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// What adoption will do, planned in full before anything is written.
#[derive(Debug, Clone, PartialEq)]
pub struct AdoptionPlan {
    layer_dir: SafeRelPath,
    layer_files: BTreeMap<SafeRelPath, Vec<u8>>,
    moves: Vec<Move>,
    removes: Vec<SafeRelPath>,
    manifest_edit: Option<ManifestEdit>,
    report: Vec<Adopted>,
    notes: Vec<Diagnostic>,
}

impl AdoptionPlan {
    /// The repo layer's folder from the repository root.
    #[must_use]
    pub const fn layer_dir(&self) -> &SafeRelPath {
        &self.layer_dir
    }

    /// The files to write below the repo layer's folder, with their final bytes: YAML merged into
    /// what the layer holds and written as §10.2 says.
    #[must_use]
    pub const fn layer_files(&self) -> &BTreeMap<SafeRelPath, Vec<u8>> {
        &self.layer_files
    }

    /// The native files that move into the layer, in the order of the plan.
    #[must_use]
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// The native files that are deleted once their layer files are written.
    #[must_use]
    pub fn removes(&self) -> &[SafeRelPath] {
        &self.removes
    }

    /// The edit of `layer.yaml`, when directories or servers were adopted.
    #[must_use]
    pub const fn manifest_edit(&self) -> Option<&ManifestEdit> {
        self.manifest_edit.as_ref()
    }

    /// What each native file became, in the order of the plan.
    #[must_use]
    pub fn report(&self) -> &[Adopted] {
        &self.report
    }

    /// How a native file was adopted, as notes of the code `adopt`.
    #[must_use]
    pub fn notes(&self) -> &[Diagnostic] {
        &self.notes
    }
}

/// Adoption is refused: nothing is to be changed, and each problem is listed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("adoption has problems; nothing was adopted")]
pub struct AdoptionRefused {
    diagnostics: Diagnostics,
}

impl AdoptionRefused {
    /// Every problem, as errors of the code `adopt`.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    fn of(problems: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics: problems
                .into_iter()
                .fold(Diagnostics::new(), Diagnostics::with),
        }
    }
}

/// The folder of the repo layer: the last stack entry, a `path:` source that lies inside the
/// repository, as a path from its root.
fn repo_layer_dir(manifest: &Manifest) -> Option<SafeRelPath> {
    manifest
        .stack()
        .iter()
        .last()
        .and_then(|entry| entry.source().repository_folder())
}

/// What a native path is to adoption.
enum Kind<'a> {
    /// `CLAUDE.md`.
    ClaudeMd,
    /// `.claude/agents/<name>.md`.
    Agent,
    /// A file below `.claude/skills/`, with its path from there.
    Skill(&'a str),
    /// A file directly in `.claude/rules/`.
    Rule(&'a str),
    /// `.claude/settings.json`.
    Settings,
    /// `.mcp.json`.
    McpJson,
    /// A file adoption cannot take.
    Other,
}

fn classify(path: &str) -> Kind<'_> {
    let under = |dir: &str| {
        path.strip_prefix(dir)
            .and_then(|rest| rest.strip_prefix('/'))
    };
    if path == "CLAUDE.md" {
        Kind::ClaudeMd
    } else if path == ".claude/settings.json" {
        Kind::Settings
    } else if path == ".mcp.json" {
        Kind::McpJson
    } else if under(".claude/agents").is_some_and(|rest| {
        !rest.contains('/')
            && rest
                .strip_suffix(".md")
                .is_some_and(|stem| !stem.is_empty())
    }) {
        Kind::Agent
    } else if let Some(rest) = under(".claude/skills") {
        Kind::Skill(rest)
    } else if let Some(rest) = under(".claude/rules").filter(|rest| !rest.contains('/')) {
        Kind::Rule(rest)
    } else {
        Kind::Other
    }
}

/// Plans the adoption of the native files in `input`.
///
/// # Errors
///
/// [`AdoptionRefused`] when the last stack entry is not a repo layer inside the repository, when a
/// native file cannot be adopted safely, or when the planned layer would be refused by the reader.
/// Each problem is listed and nothing is partly planned.
pub fn plan(input: &AdoptionInput<'_>) -> Result<AdoptionPlan, AdoptionRefused> {
    let (Some(layer_dir), Some(repo_layer)) =
        (repo_layer_dir(input.manifest), input.stack.layers().last())
    else {
        return Err(AdoptionRefused::of(vec![Diagnostic::error(
            Code::Adopt,
            None,
            Message::new(
                "--adopt needs a repo layer inside the repo as the last stack entry (e.g. path:./.layer)",
            ),
        )]));
    };
    let mut draft = Draft::new(Context {
        layer_dir: &layer_dir,
        layer_name: repo_layer.meta().name(),
        existing: repo_layer.contents(),
        merged: input.merged,
        agents_md: input.agents_md,
    });
    convert(input.natives, &mut draft);
    let edited = if draft.problems.is_empty() {
        edit_manifest(input, &mut draft)
    } else {
        None
    };
    if draft.problems.is_empty() {
        check::read_back(
            input.stack,
            edited
                .as_ref()
                .map_or(input.manifest, |edited| &edited.manifest),
            input.placements,
            &mut draft,
        );
    }
    if !draft.problems.is_empty() {
        return Err(AdoptionRefused::of(draft.problems));
    }
    Ok(AdoptionPlan {
        layer_files: draft.layer_files(),
        moves: draft
            .moves
            .iter()
            .map(|moved| Move {
                from: moved.from.clone(),
                to: moved.to.clone(),
            })
            .collect(),
        removes: draft
            .removes
            .iter()
            .filter_map(|path| SafeRelPath::try_from(path.as_str()).ok())
            .collect(),
        manifest_edit: edited.map(|edited| ManifestEdit { text: edited.text }),
        report: draft
            .report
            .iter()
            .map(|reported| Adopted {
                from: reported.from.clone(),
                to: reported.to.clone(),
            })
            .collect(),
        notes: draft.notes,
        layer_dir,
    })
}

/// Adds what each native file becomes to the draft: `AGENTS.md` first, the hook scripts next so
/// that the settings can point at them, then the rest in path order.
fn convert(natives: &BTreeMap<SafeRelPath, NativeFile>, draft: &mut Draft<'_>) {
    if let Some((path, file)) = natives
        .iter()
        .find(|(path, _)| path.as_str() == "AGENTS.md")
    {
        draft.move_file(path, file, "instructions/00-adopted.md");
    }
    let mut scripts = Vec::<(String, String)>::new();
    for (path, file) in natives {
        let script = path
            .as_str()
            .strip_prefix(names::HOOKS_DIR)
            .and_then(|rest| rest.strip_prefix('/'));
        if let Some(script) = script {
            draft.move_file(path, file, &format!("hooks/scripts/{script}"));
            scripts.push((path.as_str().to_owned(), format!("scripts/{script}")));
        }
    }
    for (path, file) in natives {
        let text = path.as_str();
        if text == "AGENTS.md" || scripts.iter().any(|(script, _)| script == text) {
            continue;
        }
        match classify(text) {
            Kind::ClaudeMd => claude_md::adopt(file.bytes(), draft),
            Kind::Agent => agent::adopt(path, file, draft),
            Kind::Skill(rest) => draft.move_file(path, file, &format!("skills/{rest}")),
            Kind::Rule(rest) => draft.move_file(path, file, &format!("rules/{rest}")),
            Kind::Settings => settings_json::adopt(file.bytes(), &scripts, draft),
            Kind::McpJson => mcp_json::adopt(file.bytes(), draft),
            Kind::Other => draft.problem(
                text,
                Message::new("cannot be adopted automatically; move it into a layer by hand"),
            ),
        }
    }
}

/// Edits `layer.yaml` for the directories and servers the draft adopted.
fn edit_manifest(
    input: &AdoptionInput<'_>,
    draft: &mut Draft<'_>,
) -> Option<manifest_edit::Edited> {
    let Ok(text) = std::str::from_utf8(input.manifest_text) else {
        draft.problem("layer.yaml", Message::new("is not valid UTF-8 text"));
        return None;
    };
    match manifest_edit::edit(text, input.manifest, &draft.add_dirs, &draft.mcp_enable) {
        Ok(edited) => edited,
        Err(why) => {
            let by_hand = format!(
                "workspace.add_dirs: {}; mcp.enable: {}",
                draft.add_dirs.join(", "),
                draft.mcp_enable.join(", ")
            );
            draft.problem(
                "layer.yaml",
                Message::new(match why {
                    manifest_edit::Refused::Form => {
                        "has a form that adoption cannot edit; add by hand "
                    }
                    manifest_edit::Refused::ReadsBack => {
                        "would not read back as the same manifest after the edit; add by hand "
                    }
                })
                .then(Stripped::from_untrusted(&by_hand)),
            );
            None
        }
    }
}
