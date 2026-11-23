//! `alz lint [--layer <path>]` (spec §10, §10.1): validates one layer, or the stack manifest and
//! lock of the repository in the working directory.
//!
//! The command wires three parts: `alz-io` reads `layer.yaml` and `layer.lock` without following
//! links and with a size cap, `alz_core::lint` decides what is wrong, and [`crate::report`]
//! prints it. R2 checks only what R2 models (the schema of a layer, the stack manifest and the
//! lock); the leak guard and the rest of the checks arrive with R4 and R5.

use std::io::{self, Write};
use std::path::Path;

use alz_core::lint::{self, LayerLint, LockFile};
use alz_core::path::{SafeRelPath, SafeRelPathError};
use alz_core::yaml::MAX_BYTES;
use alz_core::{Diagnostic, Diagnostics, Stripped};
use alz_io::{ReadError, ReadOnlyDir};

use crate::cli::LintArgs;
use crate::report::emit_diagnostics;

/// The directory linted when `--layer` is not given.
const WORKING_DIRECTORY: &str = ".";

/// What a lint run found, and the summary line that goes with it, if it has one.
#[derive(Debug)]
struct Outcome {
    diagnostics: Diagnostics,
    /// The linted layers' names, for the summary line (§10.1). A stack has none in R2.
    summary: Option<Stripped>,
}

/// Runs `alz lint`, writes the diagnostics to `stderr` and the summary to `stdout`, and returns
/// the exit code.
pub fn run(args: &LintArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let outcome = lint(args);
    let code = emit_diagnostics(&outcome.diagnostics, stderr);
    match outcome.summary {
        Some(names) => write_summary(&names, &outcome.diagnostics, stdout).map_or(1, |()| code),
        None => code,
    }
}

fn lint(args: &LintArgs) -> Outcome {
    match &args.layer {
        Some(directory) => lint_layer_in(directory),
        None => lint_working_directory(),
    }
}

/// `--layer <path>`: the directory is a layer, whatever its `layer.yaml` says.
fn lint_layer_in(directory: &Path) -> Outcome {
    let source = given(directory);
    layer_outcome(
        &source,
        read_layer_yaml(directory, &source).map(|bytes| lint::parse_document(&bytes)),
    )
}

/// No `--layer`: a `layer.yaml` with a `stack` field is a repository's manifest, anything else
/// is a layer (§10).
fn lint_working_directory() -> Outcome {
    let directory = Path::new(WORKING_DIRECTORY);
    let source = given(directory);
    match read_layer_yaml(directory, &source).map(|bytes| lint::parse_document(&bytes)) {
        Ok(Ok(document)) if lint::is_stack_manifest(&document) => Outcome {
            diagnostics: lint_stack(directory, &document),
            summary: None,
        },
        read => layer_outcome(&source, read),
    }
}

fn layer_outcome(
    source: &Stripped,
    read: Result<Result<alz_core::yaml::Node, alz_core::yaml::YamlError>, Diagnostic>,
) -> Outcome {
    match read {
        Ok(document) => {
            let linted: LayerLint = lint::layer(source, document);
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

/// The manifest's and the lock's diagnostics. A lock that cannot be read is reported as a lock
/// error; a lock that does not exist is no lock.
fn lint_stack(directory: &Path, manifest: &alz_core::yaml::Node) -> Diagnostics {
    match read_lock(directory) {
        Ok(None) => lint::stack_manifest(manifest, LockFile::Absent),
        Ok(Some(bytes)) => lint::stack_manifest(manifest, LockFile::Bytes(&bytes)),
        Err(unreadable) => lint::stack_manifest(manifest, LockFile::Unreadable(unreadable)),
    }
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
