//! The one short Claude session of `alz ls --live` (spec §10.3, target-facts §0.2): which program,
//! what is asked, and what a failure says. `alz-io` runs the program, `alz-core` reads the event.

use std::path::Path;

use alz_core::probe::Probe;
use alz_core::{Code, Diagnostic, Message, Stripped};
use alz_io::{Purpose, ToolRunner};

use crate::command::Surroundings;

/// The program when `ALZ_CLAUDE_BIN` is not set.
const DEFAULT_PROGRAM: &str = "claude";

/// The arguments of the session: one turn, so the `init` event is printed and nothing more is
/// asked of the model.
const ARGUMENTS: [&str; 7] = [
    "-p",
    "Reply with OK.",
    "--output-format",
    "stream-json",
    "--verbose",
    "--max-turns",
    "1",
];

/// The program as the user named it, stripped, for messages.
pub fn program_name(surroundings: Surroundings<'_>) -> Stripped {
    Stripped::from_untrusted(&surroundings.env.vars().get("ALZ_CLAUDE_BIN").map_or_else(
        || DEFAULT_PROGRAM.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    ))
}

fn failure(message: Message) -> Diagnostic {
    Diagnostic::error(Code::Ls, None, message)
}

/// Runs the session in `directory` and reads what Claude loaded, or says why it cannot.
pub fn probe(surroundings: Surroundings<'_>, directory: &Path) -> Result<Probe, Diagnostic> {
    let program = program_name(surroundings);
    let could_not = |reason: &str| {
        failure(
            Message::new("could not run ")
                .then(program.clone())
                .then(": ")
                .then(Stripped::from_untrusted(reason)),
        )
    };
    let runner = ToolRunner::claude(surroundings.env, Purpose::Probe)
        .map_err(|error| could_not(&error.to_string()))?;
    let arguments = ARGUMENTS.map(Into::into);
    let output = runner
        .run(&arguments, directory)
        .map_err(|error| could_not(&error.to_string()))?;
    Probe::parse(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
    .map_err(|missing| {
        failure(
            Message::new(program.clone())
                .then(" printed no init event: ")
                .then(missing.first_line().clone()),
        )
    })
}
