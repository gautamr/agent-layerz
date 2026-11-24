//! `alz check [--skip-identity]` (spec §10.3): the same steps as `alz build` without a write. It
//! fails when a generated file, or the lock, differs from a fresh build.
//!
//! With a declared commit identity it also compares the identity git would use for a commit
//! (§9.4), as an error, unless `--skip-identity` is passed.

use std::io::{self, Write};

use alz_core::identity::Command as IdentityCommand;
use alz_core::render::write_plan::{Change, WritePlan};
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped, Where};

use crate::cli::CheckArgs;
use crate::command::Surroundings;
use crate::command::build::collision_diagnostics;
use crate::command::identity::{self, Step};
use crate::command::pipeline;
use crate::report::emit_diagnostics;

/// Runs `alz check`, writes the problems to `stderr` and the verdict to `stdout` when it passes,
/// and returns the exit code.
pub fn run(
    args: &CheckArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let repository = match pipeline::open_repository(surroundings) {
        Ok(repository) => repository,
        Err(diagnostic) => return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
    };
    let built = pipeline::compute(surroundings, &repository);
    let code = emit_diagnostics(&built.diagnostics, stderr);
    let Some(computed) = built.computed else {
        return code;
    };
    let plan = match pipeline::plan(&repository, &computed) {
        Ok(Some(plan)) => plan,
        Ok(None) => return code,
        Err(error) => {
            return emit_diagnostics(
                &Diagnostics::new().with(pipeline::write_failure(&error)),
                stderr,
            );
        }
    };
    let problems = problems_of(&plan);
    let comparison_failed = problems.iter().next().is_some();
    let printed = emit_diagnostics(&problems, stderr);
    let identity = identity_problems(args, surroundings, &repository, &computed);
    let identity_code = emit_diagnostics(&identity, stderr);
    if comparison_failed || identity.has_errors() {
        let last = if comparison_failed {
            "check failed: run `alz build` and commit the result\n"
        } else {
            "check failed: the git identity differs from the declared commit identity\n"
        };
        return write_text(stderr, last).map_or(1, |()| printed.max(identity_code).max(1));
    }
    let files = computed.rendered.tree().len();
    write_text(
        stdout,
        &format!("check passed: {files} generated files match the layers and the lock\n"),
    )
    .map_or(1, |()| code.max(identity_code))
}

/// The identity step as errors (§9.4), none with `--skip-identity` or without a declaration. With
/// either, no git command runs.
fn identity_problems(
    args: &CheckArgs,
    surroundings: Surroundings<'_>,
    repository: &alz_io::RepoRoot,
    computed: &pipeline::Computed,
) -> Diagnostics {
    if args.skip_identity {
        return Diagnostics::new();
    }
    match (
        identity::declared_of(computed),
        surroundings.directory.path(),
    ) {
        (Some(declared), Some(directory)) => {
            Step::new(declared, surroundings.env, repository, directory)
                .check(IdentityCommand::Check)
        }
        (Some(_) | None, Some(_) | None) => Diagnostics::new(),
    }
}

/// One error for each difference, each collision and an outdated lock, in that order.
fn problems_of(plan: &WritePlan) -> Diagnostics {
    plan.changes()
        .iter()
        .map(|change| {
            let what = match change {
                Change::Create { .. } => "missing",
                Change::Update { .. } => "differs from a fresh build",
                Change::Delete { .. } => "is stale",
            };
            check_error(change.path().as_str(), what)
        })
        .fold(Diagnostics::new(), Diagnostics::with)
        .merge(collision_diagnostics(plan.collisions()))
        .merge(
            if plan.lock_changed() {
                Some(check_error("layer.lock", "out of date"))
            } else {
                None
            }
            .into_iter()
            .fold(Diagnostics::new(), Diagnostics::with),
        )
}

fn check_error(path: &str, what: &'static str) -> Diagnostic {
    Diagnostic::error(
        Code::Check,
        Some(Where::repository_file(Stripped::from_untrusted(path))),
        Message::new(what),
    )
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
