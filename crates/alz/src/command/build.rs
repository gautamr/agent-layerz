//! `alz build` (spec §10.3): resolve, merge and render the stack, then write the files by the
//! rules of §4.3 and the lock last.
//!
//! With `--adopt` the hand-written native files are adopted into the repo layer first (§4.3), and
//! the build then runs again from the manifest.

mod adopt;

use std::io::{self, Write};

use alz_core::identity::Command as IdentityCommand;
use alz_core::render::write_plan::{Change, Collision, WritePlan};
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped, Where};

use crate::cli::BuildArgs;
use crate::command::Surroundings;
use crate::command::identity::{self, Step};
use crate::command::pipeline::{self, Computed};
use crate::report::emit_diagnostics;

/// Runs `alz build`, writes the diagnostics to `stderr` and the summary to `stdout`, and returns
/// the exit code.
pub fn run(
    args: &BuildArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let repository = match pipeline::open_repository(surroundings) {
        Ok(repository) => repository,
        Err(diagnostic) => return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
    };
    let first = pipeline::compute(surroundings, &repository);
    let built = if args.adopt {
        match adopt::adopt_first(surroundings, &repository, first, stdout, stderr) {
            Ok(second) => second,
            Err(code) => return code,
        }
    } else {
        first
    };
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
    let counts = Counts::of(&plan);
    match plan.into_clear() {
        Err(collisions) => emit_diagnostics(&collision_diagnostics(&collisions), stderr),
        Ok(clear) => match repository.apply(&clear) {
            Err(error) => emit_diagnostics(
                &Diagnostics::new().with(pipeline::write_failure(&error)),
                stderr,
            ),
            Ok(_) => match write_summary(&computed, counts, stdout) {
                Ok(()) => code.max(identity_step(
                    args,
                    surroundings,
                    &repository,
                    &computed,
                    stdout,
                    stderr,
                )),
                Err(io::Error { .. }) => 1,
            },
        },
    }
}

/// The commit identity step (spec §9.4, §10.3): `--adopt` sets the declared identity as `alz init`
/// does, a plain build only warns about a mismatch. Without a declaration nothing runs. Returns
/// the exit code, 1 only when a check after a write failed.
fn identity_step(
    args: &BuildArgs,
    surroundings: Surroundings<'_>,
    repository: &alz_io::RepoRoot,
    computed: &Computed,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let (Some(declared), Some(directory)) = (
        identity::declared_of(computed),
        surroundings.directory.path(),
    ) else {
        return 0;
    };
    let step = Step::new(declared, surroundings.env, repository, directory);
    if args.adopt {
        let outcome = step.set_up(IdentityCommand::Build);
        let printed = write_text(stdout, &outcome.stdout);
        let code = emit_diagnostics(&outcome.diagnostics, stderr);
        printed.map_or(1, |()| code)
    } else {
        emit_diagnostics(&step.check(IdentityCommand::Build), stderr)
    }
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

/// The errors `collision`, one for each path the writer refuses to touch.
pub fn collision_diagnostics(collisions: &[Collision]) -> Diagnostics {
    collisions
        .iter()
        .map(|collision| {
            Diagnostic::error(
                Code::Collision,
                Some(Where::repository_file(Stripped::from_untrusted(
                    collision.path().as_str(),
                ))),
                Message::new(Stripped::from_untrusted(&collision.reason().to_string())),
            )
        })
        .fold(Diagnostics::new(), Diagnostics::with)
}

/// How many files a plan creates, updates and deletes.
#[derive(Debug, Clone, Copy)]
struct Counts {
    created: usize,
    updated: usize,
    deleted: usize,
}

impl Counts {
    fn of(plan: &WritePlan) -> Self {
        let count =
            |wanted: fn(&Change) -> bool| plan.changes().iter().filter(|c| wanted(c)).count();
        Self {
            created: count(|change| matches!(change, Change::Create { .. })),
            updated: count(|change| matches!(change, Change::Update { .. })),
            deleted: count(|change| matches!(change, Change::Delete { .. })),
        }
    }
}

/// `built <n> files for <targets>: <c> created, <u> updated, <d> deleted (output visibility: <v>)`.
fn write_summary(computed: &Computed, counts: Counts, stdout: &mut impl Write) -> io::Result<()> {
    let files = computed.rendered.tree().len();
    let targets = computed
        .manifest
        .targets()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(
        stdout,
        "built {files} file{} for {targets}: {} created, {} updated, {} deleted (output visibility: {})",
        if files == 1 { "" } else { "s" },
        counts.created,
        counts.updated,
        counts.deleted,
        computed.lock.visibility.as_str(),
    )?;
    stdout.flush()
}
