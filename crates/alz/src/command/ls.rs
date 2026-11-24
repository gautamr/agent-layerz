//! `alz ls` (spec §10.3): what the stack builds, by section, with the layer that decides each item.
//!
//! The listing is read from the merged model, after the diagnostics of the shared steps. With
//! `--live` the same [`Computed`] is compared with what Claude
//! says it loaded, after the listing has been printed.

use std::io::{self, Write};

use alz_core::listing::Listing;
use alz_core::live::{Built, Comparison};
use alz_core::target::TargetName;
use alz_core::{Code, Diagnostic, Diagnostics, Message};

use crate::cli::LsArgs;
use crate::command::pipeline::{self, Computed};
use crate::command::{Surroundings, live_probe};
use crate::report::emit_diagnostics;

/// Runs `alz ls`, writes the diagnostics to `stderr` and the listing to `stdout`, and returns the
/// exit code.
pub fn run(
    args: &LsArgs,
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
    if write_text(stdout, &Listing::of(&computed.merged).to_string()).is_err() {
        return 1;
    }
    if args.live {
        code.max(live(&computed, surroundings, stdout, stderr))
    } else {
        code
    }
}

/// The comparison with Claude, after the listing (§10.3).
fn live(
    computed: &Computed,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    if !computed.manifest.targets().contains(&TargetName::Claude) {
        return fail(stderr, Message::new("--live needs the claude target"));
    }
    let Some(directory) = surroundings.directory.path() else {
        return fail(stderr, Message::new("cannot find the working directory"));
    };
    let asking = format!(
        "\nasking {} what it loads here (one short model call)…\n",
        live_probe::program_name(surroundings)
    );
    if write_text(stdout, &asking).is_err() {
        return 1;
    }
    match live_probe::probe(surroundings, std::path::Path::new(directory.as_str())) {
        Err(diagnostic) => emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
        Ok(probe) => {
            let comparison = Comparison::of(&Built::of(&computed.merged), &probe);
            let code = emit_diagnostics(comparison.diagnostics(), stderr);
            write_text(stdout, &comparison.to_string()).map_or(1, |()| code)
        }
    }
}

fn fail(stderr: &mut impl Write, message: Message) -> u8 {
    emit_diagnostics(
        &Diagnostics::new().with(Diagnostic::error(Code::Ls, None, message)),
        stderr,
    )
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
