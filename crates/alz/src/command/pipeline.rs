//! The steps `alz build` and `alz check` share (spec §6, §10.3): read the manifest and the lock,
//! resolve and merge the stack, render the build in memory, and decide what the writer would do.
//!
//! The pipeline makes no write. `alz-io` reads the repository and resolves the stack, `alz-core`
//! decides, and the commands that use this module print and, for `build`, apply the plan.

use alz_core::lint::{self, LockFile};
use alz_core::lock::Lock;
use alz_core::merge::merge;
use alz_core::path::AbsolutePath;
use alz_core::render::gitignore::Generated;
use alz_core::render::rendered::{Rendered, render};
use alz_core::render::write_plan::{Disk, WritePlan, plan_writes};
use alz_core::stack::Manifest;
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped, Where, leak_guard};
use alz_io::{ReadError, RepoError, RepoRoot, StackResolver, WalkError};
use std::path::Path;

use crate::command::Surroundings;
use crate::command::repo_files::{WORKING_DIRECTORY, read_in, read_lock};

/// The file that hides the generated `AGENTS.md` from Codex (target-facts §8.4).
const OVERRIDE_FILE: &str = "AGENTS.override.md";

/// What a build made in memory: the files, the lock it would write and the lock that was there.
#[derive(Debug)]
pub struct Computed {
    pub manifest: Manifest,
    pub rendered: Rendered,
    pub lock: Lock,
    pub previous: Option<Lock>,
}

/// What the shared steps found: every diagnostic, and the build when it has no error.
#[derive(Debug)]
pub struct Pipeline {
    pub diagnostics: Diagnostics,
    pub computed: Option<Computed>,
}

impl Pipeline {
    fn failed(diagnostics: Diagnostics) -> Self {
        Self {
            diagnostics,
            computed: None,
        }
    }
}

/// Opens the repository in the working directory for reading and writing.
pub fn open_repository(surroundings: Surroundings<'_>) -> Result<RepoRoot, Diagnostic> {
    let directory: &AbsolutePath = surroundings
        .directory
        .path()
        .ok_or_else(|| failure("cannot find the working directory"))?;
    RepoRoot::at(surroundings.env, directory).map_err(|error| match error {
        WalkError::Refused(_) => failure(
            "the working directory touches an account folder or the home folder; alz never writes there",
        ),
        WalkError::Missing | WalkError::NotADirectory | WalkError::TooManyLinks | WalkError::Io(_) => {
            failure("cannot open the working directory")
        }
    })
}

fn failure(message: &'static str) -> Diagnostic {
    Diagnostic::error(Code::Write, None, Message::new(message))
}

/// Reads, resolves, merges and renders (§6).
pub fn compute(surroundings: Surroundings<'_>, repository: &RepoRoot) -> Pipeline {
    let directory = Path::new(WORKING_DIRECTORY);
    let manifest_file = Where::repository_file(Stripped::from_untrusted("layer.yaml"));
    let document = match read_in(directory, "layer.yaml") {
        Ok(bytes) => lint::parse_document(&bytes),
        Err(ReadError::Missing) => return Pipeline::failed(no_manifest(surroundings)),
        Err(error) => {
            return Pipeline::failed(Diagnostics::new().with(
                error.into_layer_yaml_diagnostic(&Stripped::from_untrusted(WORKING_DIRECTORY)),
            ));
        }
    };
    let document = match document {
        Ok(document) => document,
        Err(error) => {
            return Pipeline::failed(Diagnostics::new().with(error.into_diagnostic(manifest_file)));
        }
    };
    let lock_bytes = read_lock(directory);
    let lock = match &lock_bytes {
        Ok(None) => LockFile::Absent,
        Ok(Some(bytes)) => LockFile::Bytes(bytes),
        Err(unreadable) => LockFile::Unreadable(unreadable.clone()),
    };
    match lint::read_stack(&document, lock) {
        Err(diagnostics) => Pipeline::failed(diagnostics),
        Ok(files) => resolve_and_render(surroundings, repository, files.manifest, files.lock),
    }
}

fn no_manifest(surroundings: Surroundings<'_>) -> Diagnostics {
    let directory = surroundings
        .directory
        .path()
        .map_or_else(String::new, |path| path.as_str().to_owned());
    Diagnostics::new().with(Diagnostic::error(
        Code::NoManifest,
        Some(Where::given(Stripped::from_untrusted(&directory))),
        Message::new("no layer.yaml here; run `alz init` first"),
    ))
}

fn resolve_and_render(
    surroundings: Surroundings<'_>,
    repository: &RepoRoot,
    manifest: Manifest,
    previous: Option<Lock>,
) -> Pipeline {
    let resolver =
        match StackResolver::in_working_directory(surroundings.env, surroundings.directory) {
            Ok(resolver) => resolver,
            Err(diagnostic) => return Pipeline::failed(Diagnostics::new().with(diagnostic)),
        };
    let report = resolver.resolve(&manifest, previous.as_ref());
    let guarded = report
        .resolved_layers()
        .iter()
        .map(|layer| leak_guard::check(layer.meta(), layer.contents()))
        .fold(Diagnostics::new(), Diagnostics::merge);
    let found = report.diagnostics().clone().merge(guarded);
    let Some(stack) = report.stack() else {
        return Pipeline::failed(found);
    };
    let merging = merge(stack, &manifest, &resolver.placements(stack));
    let found = found.merge(merging.diagnostics().clone());
    let Some(merged) = merging.merged() else {
        return Pipeline::failed(found);
    };
    let rendered = render(merged, &manifest);
    let found = found
        .merge(rendered.diagnostics().clone())
        .merge(shadowing_files(repository));
    if found.has_errors() {
        return Pipeline::failed(found);
    }
    let lock = rendered.lock(stack);
    Pipeline {
        diagnostics: found,
        computed: Some(Computed {
            manifest,
            rendered,
            lock,
            previous,
        }),
    }
}

/// An `AGENTS.override.md` anywhere in the repository hides the generated `AGENTS.md` from Codex
/// (target-facts §8.4): the error `collision` at each.
fn shadowing_files(repository: &RepoRoot) -> Diagnostics {
    match repository.files_named(OVERRIDE_FILE) {
        Ok(paths) => paths
            .iter()
            .map(|path| {
                Diagnostic::error(
                    Code::Collision,
                    Some(Where::repository_file(Stripped::from_untrusted(
                        path.as_str(),
                    ))),
                    Message::new(
                        "AGENTS.override.md silently shadows the generated AGENTS.md; remove it",
                    ),
                )
            })
            .fold(Diagnostics::new(), Diagnostics::with),
        Err(error) => Diagnostics::new().with(write_failure(&error)),
    }
}

/// What the writer would do for `computed`, from what is on disk. `None` when the build has an
/// error, which writes nothing (§4.3 "Writing").
pub fn plan(repository: &RepoRoot, computed: &Computed) -> Result<Option<WritePlan>, RepoError> {
    plan_with_disk(repository, computed).map(|planned| planned.map(|(plan, _)| plan))
}

/// [`plan`] with what the writer observed, which `alz diff` reads the old bytes from.
pub fn plan_with_disk(
    repository: &RepoRoot,
    computed: &Computed,
) -> Result<Option<(WritePlan, Disk)>, RepoError> {
    computed
        .rendered
        .to_write()
        .map(|tree| {
            repository
                .observe(
                    tree,
                    computed.previous.as_ref(),
                    computed.rendered.managed_folders(),
                )
                .map(|disk| {
                    let plan = plan_writes(
                        tree,
                        &computed.lock,
                        computed.previous.as_ref(),
                        &disk,
                        computed.rendered.protected(),
                        generated(&computed.manifest),
                    );
                    (plan, disk)
                })
        })
        .transpose()
}

const fn generated(manifest: &Manifest) -> Generated {
    if manifest.options().commit_generated() {
        Generated::Committed
    } else {
        Generated::Ignored
    }
}

/// The error `write` for a failure of the repository root.
pub fn write_failure(error: &RepoError) -> Diagnostic {
    Diagnostic::error(
        Code::Write,
        None,
        Message::new(Stripped::from_untrusted(&chain(error))),
    )
}

/// An error and its sources, joined by `: `.
fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    std::iter::successors(Some(error), |error| error.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}
