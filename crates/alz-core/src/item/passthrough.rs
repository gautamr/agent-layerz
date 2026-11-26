//! The passthrough files of a layer (spec §4.1, §7.11): `targets/<target>/<path>` for a known
//! target, copied into place as they are. `targets/claude/CLAUDE.extra.md` is not one (§7.1).
//!
//! Each file is checked for the variables no layer may set (§7.9) before it is an item, and a
//! file with a problem gives no item. A file that is used gives a `passthrough` warning, so the
//! layer's author turns the file into a first-class item over time. A file below a folder of
//! `targets/` that is not a known target is an error that names the file; a file directly in
//! `targets/` is no item and gives no diagnostic.

mod check;
mod coverage;
mod toml_text;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::{ItemPath, LayerName};
use crate::path::SafeRelPath;
use crate::stack::ResolvedLayer;
use crate::untrusted_text::Stripped;
use alz_adapter::TargetId;

use super::place;
use super::protected_variable::unchecked;

/// One file of `targets/<target>/` of a layer, with the bytes the layer walk kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassthroughFile {
    target: TargetId,
    path: SafeRelPath,
    source: SafeRelPath,
    bytes: Vec<u8>,
    executable: bool,
}

impl PassthroughFile {
    /// The target the file is for.
    #[must_use]
    pub const fn target(&self) -> &TargetId {
        &self.target
    }

    /// The path inside the target's folder: where the file is written, relative to the root the
    /// target's adapter writes under. Two layers' files merge when they have the same target and
    /// this path.
    #[must_use]
    pub const fn path(&self) -> &SafeRelPath {
        &self.path
    }

    /// The path in the layer, `targets/<target>/<path>`.
    #[must_use]
    pub const fn source(&self) -> &SafeRelPath {
        &self.source
    }

    /// The content, byte for byte.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether the file has the owner-execute bit in the layer, which it keeps where it is
    /// written (§8.1).
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        self.executable
    }
}

/// What one layer holds of §7.11: its passthrough files in path order (§6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PassthroughItems {
    files: Vec<PassthroughFile>,
}

impl PassthroughItems {
    /// The files, in the order of their paths in the layer.
    #[must_use]
    pub fn files(&self) -> &[PassthroughFile] {
        &self.files
    }

    /// The files, for a caller that takes them apart.
    #[must_use]
    pub fn into_files(self) -> Vec<PassthroughFile> {
        self.files
    }

    /// Reads the passthrough files of `layer`. Every problem is reported, and a file with a
    /// problem gives no item. Each file that is read gives a `passthrough` warning.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        let (items, found) = Self::scan(layer);
        let all = found
            .into_iter()
            .fold(Diagnostics::new(), |all, entry| match entry {
                Found::Warning(_, warning) => all.with(warning),
                Found::Problems(problems) => all.merge(problems),
            });
        (items, all)
    }

    /// Reads the passthrough files of `layer` like [`PassthroughItems::read`], but returns the
    /// warning of each file with its target instead of in the diagnostics, so a caller that
    /// knows the targets of the build keeps only the warnings of files that will be written
    /// (§5.2 **Target ids**).
    #[must_use]
    pub fn read_apart(layer: &ResolvedLayer) -> (Self, Vec<(TargetId, Diagnostic)>, Diagnostics) {
        let (items, found) = Self::scan(layer);
        let (warnings, problems) = found.into_iter().fold(
            (Vec::new(), Diagnostics::new()),
            |(warnings, problems), entry| match entry {
                Found::Warning(target, warning) => (
                    warnings.into_iter().chain([(target, warning)]).collect(),
                    problems,
                ),
                Found::Problems(more) => (warnings, problems.merge(more)),
            },
        );
        (items, warnings, problems)
    }

    /// The files and what was found while reading them, in path order.
    fn scan(layer: &ResolvedLayer) -> (Self, Vec<Found>) {
        let name = layer.meta().name();
        let (files, found) = layer.contents().iter().fold(
            (Vec::new(), Vec::new()),
            |(files, found): (Vec<PassthroughFile>, Vec<Found>), (path, bytes)| {
                let more = |entry: Found| found.iter().cloned().chain([entry]).collect();
                match ItemPath::of(path) {
                    ItemPath::Passthrough {
                        target,
                        path: inner,
                    } => match read_file(
                        name,
                        path,
                        target.clone(),
                        inner,
                        bytes,
                        layer.contents().is_executable(path),
                    ) {
                        Ok((file, warning)) => (
                            files.iter().cloned().chain([file]).collect(),
                            more(Found::Warning(target, warning)),
                        ),
                        Err(problems) => (files, more(Found::Problems(problems))),
                    },
                    // Below `targets/`, `Other` is a folder that is not a target id: the others are
                    // `Passthrough` and `ClaudeExtra`.
                    ItemPath::Other => (files, more(Found::Problems(unknown_target(name, path)))),
                    ItemPath::LayerYaml
                    | ItemPath::Instruction { .. }
                    | ItemPath::Agent { .. }
                    | ItemPath::AgentOverlay { .. }
                    | ItemPath::SkillFile { .. }
                    | ItemPath::Command { .. }
                    | ItemPath::Rule { .. }
                    | ItemPath::Hooks
                    | ItemPath::HookScript { .. }
                    | ItemPath::Permissions
                    | ItemPath::Mcp
                    | ItemPath::Models
                    | ItemPath::Settings
                    | ItemPath::ClaudeExtra => (files, found),
                }
            },
        );
        (Self { files }, found)
    }
}

/// What reading one file of `targets/` found.
#[derive(Debug, Clone)]
enum Found {
    /// The `passthrough` warning of a file that is read, with the file's target.
    Warning(TargetId, Diagnostic),
    /// The problems of a file that is refused, or of a folder that is not a target id.
    Problems(Diagnostics),
}

/// The file and its warning, or every problem found in it.
fn read_file(
    layer: &LayerName,
    source: &SafeRelPath,
    target: TargetId,
    inner: &str,
    bytes: &[u8],
    executable: bool,
) -> Result<(PassthroughFile, Diagnostic), Diagnostics> {
    let at = place(layer, source);
    let problems = non_ascii(source, &at).merge(check::problems(inner, bytes, &at));
    if problems.has_errors() {
        return Err(problems);
    }
    // A part of a safe path between two `/` is safe, so the refusal is not reachable; it keeps
    // the reader total.
    let path = SafeRelPath::try_from(inner).map_err(|error| {
        Diagnostics::new().with(Diagnostic::error(
            Code::Passthrough,
            Some(at.clone()),
            Message::new("the path inside the target's folder ").then(error.rule()),
        ))
    })?;
    let warning = Diagnostic::warning(
        Code::Passthrough,
        Some(at),
        Message::new("raw passthrough to ")
            .then(Stripped::from_untrusted(inner))
            .then("; consider turning it into a first-class item"),
    );
    Ok((
        PassthroughFile {
            target,
            path,
            source: source.clone(),
            bytes: bytes.to_vec(),
            executable,
        },
        warning,
    ))
}

/// A file below a folder of `targets/` that is not a known target: an error that names the file.
/// A file directly in `targets/` is no item, so it has no diagnostic.
fn unknown_target(layer: &LayerName, path: &SafeRelPath) -> Diagnostics {
    path.as_str()
        .strip_prefix("targets/")
        .and_then(|below| below.split_once('/'))
        .map_or_else(Diagnostics::new, |_| {
            let at = place(layer, path);
            Diagnostics::new()
                .with(Diagnostic::error(
                    Code::Passthrough,
                    Some(at.clone()),
                    Message::new(
                        "this file is below a folder of targets/ that is not a target id (\u{a7}5.2)",
                    ),
                ))
                .merge(non_ascii(path, &at))
        })
}

/// A path with a non-ASCII character is refused, whatever the target: a file system may fold the
/// character into a checked name (spec §7.9, `docs/target-facts.md` §12).
fn non_ascii(path: &SafeRelPath, at: &Where) -> Diagnostics {
    if path.as_str().is_ascii() {
        Diagnostics::new()
    } else {
        unchecked("this path", "has a non-ASCII character", at)
    }
}

#[cfg(test)]
mod tests;
