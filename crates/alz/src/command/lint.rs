//! `alz lint [--layer <path>]` (spec §10, §10.1): validates one layer, or the stack manifest and
//! lock of the repository in the working directory.
//!
//! The command wires three parts: `alz-io` reads `layer.yaml` and `layer.lock` without following
//! links and with a size cap and resolves the stack's layers, `alz_core::lint` decides what is
//! wrong, and [`crate::report`] prints it. It checks what is modelled so far (the schema of a
//! layer, the stack manifest and lock, and the resolution of the stack's layers) and the leak
//! guard of each public layer (§9.2); the rest of the checks arrive with R5.

use std::io::{self, Write};
use std::path::Path;

use alz_core::layer::{LayerContents, Purpose, ReadLayer, Visibility, scan_verdict};
use alz_core::lint::{self, LayerLint, LockFile};
use alz_core::path::{SafeRelPath, SafeRelPathError};
use alz_core::stack::resolution_failure::layer_unreadable;
use alz_core::yaml::MAX_BYTES;
use alz_core::{Diagnostic, Diagnostics, Stripped, leak_guard};
use alz_io::{EnvSnapshot, ReadError, ReadOnlyDir, StackResolver, WorkingDirectory};

use crate::cli::LintArgs;
use crate::report::emit_diagnostics;

/// The directory linted when `--layer` is not given.
const WORKING_DIRECTORY: &str = ".";

/// What a lint run found, and the summary line that goes with it, if it has one.
#[derive(Debug)]
struct Outcome {
    diagnostics: Diagnostics,
    /// The linted layers' names, for the summary line (§10.1). A stack whose manifest or lock is
    /// invalid has none: no layer was reached.
    summary: Option<Stripped>,
}

/// Where and with what environment the command runs, both taken once when the process started.
#[derive(Debug, Clone, Copy)]
pub struct Surroundings<'a> {
    pub env: &'a EnvSnapshot,
    pub directory: &'a WorkingDirectory,
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

/// A layer alone: its `layer.yaml`, then, when the layer is public, the leak guard over every file
/// of the directory (§9.2).
fn layer_outcome(
    directory: &Path,
    source: &Stripped,
    read: Result<Result<alz_core::yaml::Node, alz_core::yaml::YamlError>, Diagnostic>,
) -> Outcome {
    match read {
        Ok(document) => {
            let linted: LayerLint = lint::layer(source, document);
            let linted = match guard_of(directory, source, &linted) {
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

/// The leak guard's findings for a valid public layer in `directory`, or the one problem that kept
/// its files from being read; `None` for a layer that is invalid or not public.
fn guard_of(directory: &Path, source: &Stripped, linted: &LayerLint) -> Option<Diagnostics> {
    let meta = linted
        .meta()
        .filter(|meta| meta.visibility() == Visibility::Public)?;
    Some(read_contents(directory, source).map_or_else(
        |unreadable| Diagnostics::new().with(unreadable),
        |contents| leak_guard::check(meta, &contents),
    ))
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
                        .map(|layer| leak_guard::check(layer.meta(), layer.contents()))
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

fn read_layer_yaml(directory: &Path, source: &Stripped) -> Result<Vec<u8>, Diagnostic> {
    read_in(directory, "layer.yaml").map_err(|error| error.into_layer_yaml_diagnostic(source))
}

fn read_lock(directory: &Path) -> Result<Option<Vec<u8>>, Diagnostic> {
    match read_in(directory, "layer.lock") {
        Ok(bytes) => Ok(Some(bytes)),
        Err(ReadError::Missing) => Ok(None),
        Err(error) => Err(error.into_lock_diagnostic()),
    }
}

/// The regular file `name` of `directory`, at most 1 MiB, never reached through a link.
fn read_in(directory: &Path, name: &str) -> Result<Vec<u8>, ReadError> {
    let path = SafeRelPath::try_from(name).map_err(|SafeRelPathError| ReadError::Missing)?;
    ReadOnlyDir::open(directory)?.read_file(&path, MAX_BYTES)
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
