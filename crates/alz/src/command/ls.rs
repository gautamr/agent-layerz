//! `alz ls` (spec §10.3): what the stack builds, by section, with the layer that decides each item.
//!
//! The listing is read from the merged model, after the diagnostics of the shared steps. `--live`
//! is a later step (TASK-55.6.11): it takes the same [`Computed`](crate::command::pipeline::Computed),
//! runs after [`run`] has printed the listing, and compares it with what Claude loaded.

use std::io::{self, Write};

use alz_core::Diagnostics;
use alz_core::listing::Listing;

use crate::command::Surroundings;
use crate::command::pipeline;
use crate::report::emit_diagnostics;

/// Runs `alz ls`, writes the diagnostics to `stderr` and the listing to `stdout`, and returns the
/// exit code.
pub fn run(surroundings: Surroundings<'_>, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let repository = match pipeline::open_repository(surroundings) {
        Ok(repository) => repository,
        Err(diagnostic) => return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
    };
    let built = pipeline::compute(surroundings, &repository);
    let code = emit_diagnostics(&built.diagnostics, stderr);
    let Some(computed) = built.computed else {
        return code;
    };
    write_text(stdout, &Listing::of(&computed.merged).to_string()).map_or(1, |()| code)
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
