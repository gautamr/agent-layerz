//! `alz check [--skip-identity]` (spec §10.3): the same steps as `alz build` without a write. It
//! fails when a generated file, or the lock, differs from a fresh build.
//!
//! The commit identity comparison (§9.4) is a later task (TASK-55.7): `--skip-identity` is
//! parsed and has nothing to skip yet.

use std::io::{self, Write};

use alz_core::render::write_plan::{Change, WritePlan};
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped, Where};

use crate::cli::CheckArgs;
use crate::command::Surroundings;
use crate::command::build::collision_diagnostics;
use crate::command::pipeline;
use crate::report::emit_diagnostics;

/// Runs `alz check`, writes the problems to `stderr` and the verdict to `stdout` when it passes,
/// and returns the exit code.
pub fn run(
    _args: &CheckArgs,
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
    if problems.iter().next().is_some() {
        let printed = emit_diagnostics(&problems, stderr);
        let failed = write_text(
            stderr,
            "check failed: run `alz build` and commit the result\n",
        );
        return failed.map_or(1, |()| printed.max(1));
    }
    let files = computed.rendered.tree().len();
    write_text(
        stdout,
        &format!("check passed: {files} generated files match the layers and the lock\n"),
    )
    .map_or(1, |()| code)
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
