//! `alz lint [--layer <path>]` (spec §10, §10.1): validates one layer, or the stack manifest and
//! lock of the repository in the working directory.
//!
//! The command wires three parts: `alz-io` reads `layer.yaml` and `layer.lock` without following
//! links and with a size cap and resolves the stack's layers, `alz_core::lint` decides what is
//! wrong, and [`crate::report`] prints it. It checks the schema of a layer, the stack manifest and
//! lock, the resolution of the stack's layers, the leak guard of each public layer (§9.2) and
//! what the reader of each kind of item finds in each layer alone (§10).

use std::io::{self, Write};
use std::path::Path;

use alz_core::layer::{LayerContents, Purpose, ReadLayer, scan_verdict};
use alz_core::lint::{self, LayerLint, LockFile};
use alz_core::merge::check_alone;
use alz_core::stack::resolution_failure::layer_unreadable;
use alz_core::stack::{ResolvedLayer, Source};
use alz_core::{Diagnostic, Diagnostics, Stripped, leak_guard};
use alz_io::{ReadOnlyDir, StackResolver};

use crate::cli::LintArgs;
use crate::command::Surroundings;
use crate::command::repo_files::{WORKING_DIRECTORY, read_layer_yaml, read_lock};
use crate::report::emit_diagnostics;

/// What a lint run found, and the summary line that goes with it, if it has one.
#[derive(Debug)]
struct Outcome {
    diagnostics: Diagnostics,
    /// The linted layers' names, for the summary line (§10.1). A stack whose manifest or lock is
    /// invalid has none: no layer was reached.
    summary: Option<Stripped>,
}

/// Runs `alz lint`, writes the diagnostics to `stderr` and the summary to `stdout`, and returns
/// the exit code.
pub fn run(
    args: &LintArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let outcome = lint(args, surroundings);
    let code = emit_diagnostics(&outcome.diagnostics, stderr);
    match outcome.summary {
        Some(names) => write_summary(&names, &outcome.diagnostics, stdout).map_or(1, |()| code),
        None => code,
    }
}

fn lint(args: &LintArgs, surroundings: Surroundings<'_>) -> Outcome {
    match &args.layer {
        Some(directory) => lint_layer_in(directory),
        None => lint_working_directory(surroundings),
    }
}

/// `--layer <path>`: the directory is a layer, whatever its `layer.yaml` says.
fn lint_layer_in(directory: &Path) -> Outcome {
    let source = given(directory);
    layer_outcome(
        directory,
        &source,
        read_layer_yaml(directory, &source).map(|bytes| lint::parse_document(&bytes)),
    )
}

/// No `--layer`: a `layer.yaml` with a `stack` field is a repository's manifest, anything else
/// is a layer (§10).
fn lint_working_directory(surroundings: Surroundings<'_>) -> Outcome {
    let directory = Path::new(WORKING_DIRECTORY);
    let source = given(directory);
    match read_layer_yaml(directory, &source).map(|bytes| lint::parse_document(&bytes)) {
        Ok(Ok(document)) if lint::is_stack_manifest(&document) => {
            lint_stack(directory, &document, surroundings)
        }
        read => layer_outcome(directory, &source, read),
    }
}

/// A layer alone: its `layer.yaml`, then, when the layer is valid, the checks over every file of
/// the directory: the leak guard when the layer is public (§9.2) and the readers of every kind
/// (§10).
fn layer_outcome(
    directory: &Path,
    source: &Stripped,
    read: Result<Result<alz_core::yaml::Node, alz_core::yaml::YamlError>, Diagnostic>,
) -> Outcome {
    match read {
        Ok(document) => {
            let linted: LayerLint = lint::layer(source, document);
            let linted = match files_of(directory, source, &linted) {
                Some(found) => linted.with_diagnostics(found),
                None => linted,
            };
            Outcome {
                summary: Some(linted.name().clone()),
                diagnostics: linted.into_diagnostics(),
            }
        }
        Err(diagnostic) => Outcome {
            diagnostics: Diagnostics::new().with(diagnostic),
            summary: Some(source.clone()),
        },
    }
}

/// What the files of a valid layer in `directory` show, or the one problem that kept them from
/// being read; `None` for a layer that is invalid.
fn files_of(directory: &Path, source: &Stripped, linted: &LayerLint) -> Option<Diagnostics> {
    let meta = linted.meta()?;
    Some(read_contents(directory, source).map_or_else(
        |unreadable| Diagnostics::new().with(unreadable),
        |contents| {
            checked_alone(&ResolvedLayer::new(
                Source::working_directory(),
                meta.clone(),
                contents,
                None,
            ))
        },
    ))
}

/// The leak guard over a layer's files, then what its readers find in them, each layer alone: a
/// reference to a more general layer is not checked (§10).
fn checked_alone(layer: &ResolvedLayer) -> Diagnostics {
    leak_guard::check(layer.meta(), layer.contents()).merge(check_alone(layer))
}

/// Every file of the layer in `directory`, read as a build reads it (§6, §7.13): no link is
/// followed, no file is larger than 16 MiB.
fn read_contents(directory: &Path, source: &Stripped) -> Result<LayerContents, Diagnostic> {
    let scan = ReadOnlyDir::open(directory)
        .map(|opened| opened.scan(Purpose::Build))
        .map_err(|_unreadable| layer_unreadable(source.clone()))?;
    scan_verdict(Purpose::Build, scan)
        .map(|read: ReadLayer| read.into_parts().0)
        .map_err(|refusal| {
            refusal
                .file_name_diagnostic(source.clone())
                .unwrap_or_else(|| layer_unreadable(source.clone()))
        })
}

/// A repository's stack: the manifest and the lock are read first, and when both are valid each
/// entry is resolved (§6) and its `layer.yaml` read, then the stack is validated. A lock that
/// cannot be read is reported as a lock error; a lock that does not exist is no lock.
fn lint_stack(
    directory: &Path,
    manifest: &alz_core::yaml::Node,
    surroundings: Surroundings<'_>,
) -> Outcome {
    let bytes = read_lock(directory);
    let lock = match &bytes {
        Ok(None) => LockFile::Absent,
        Ok(Some(bytes)) => LockFile::Bytes(bytes),
        Err(unreadable) => LockFile::Unreadable(unreadable.clone()),
    };
    match lint::read_stack(manifest, lock) {
        Err(diagnostics) => Outcome {
            diagnostics,
            summary: None,
        },
        Ok(files) => {
            match StackResolver::in_working_directory(surroundings.env, surroundings.directory) {
                Err(diagnostic) => Outcome {
                    diagnostics: Diagnostics::new().with(diagnostic),
                    summary: None,
                },
                Ok(resolver) => {
                    let report = resolver.resolve(&files.manifest, files.lock.as_ref());
                    let guarded = report
                        .resolved_layers()
                        .iter()
                        .map(checked_alone)
                        .fold(Diagnostics::new(), Diagnostics::merge);
                    Outcome {
                        summary: Some(joined(report.names())),
                        diagnostics: report.diagnostics().clone().merge(guarded),
                    }
                }
            }
        }
    }
}

/// The names joined by `, ` (§10.1).
fn joined(names: &[Stripped]) -> Stripped {
    Stripped::from_untrusted(
        &names
            .iter()
            .map(Stripped::as_str)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// The directory as the person typed it, stripped (§10.1).
fn given(directory: &Path) -> Stripped {
    Stripped::from_untrusted(&directory.to_string_lossy())
}

/// `lint ok: <names>` or `lint failed: <n> errors in <names>` on standard output (§10.1).
fn write_summary(
    names: &Stripped,
    diagnostics: &Diagnostics,
    stdout: &mut impl Write,
) -> io::Result<()> {
    let line = match diagnostics.error_count() {
        0 => format!("lint ok: {names}\n"),
        1 => format!("lint failed: 1 error in {names}\n"),
        count => format!("lint failed: {count} errors in {names}\n"),
    };
    stdout.write_all(line.as_bytes())?;
    stdout.flush()
}
