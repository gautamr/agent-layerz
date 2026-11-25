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
    if let Err(diagnostic) = require_claude(computed.manifest.targets().as_slice()) {
        return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr);
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

/// `--live` asks Claude, so the manifest has to build for it (§10.3).
fn require_claude(targets: &[TargetName]) -> Result<(), Diagnostic> {
    if targets.contains(&TargetName::Claude) {
        Ok(())
    } else {
        Err(Diagnostic::error(
            Code::Ls,
            None,
            Message::new("--live needs the claude target"),
        ))
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

#[cfg(test)]
mod tests {
    use alz_core::target::TargetName;

    use super::require_claude;

    /// Spec §10.3 `alz ls [--live]`: `--live` needs the `claude` target, else it is
    /// `error [ls] --live needs the claude target`. No manifest can name only another target yet
    /// (the Codex adapter is TASK-18), so the check is tested where it is made.
    #[test]
    fn live_needs_the_claude_target() {
        assert!(require_claude(&[TargetName::Claude]).is_ok());
        assert!(require_claude(&[TargetName::Codex, TargetName::Claude]).is_ok());
        let refused = require_claude(&[TargetName::Codex]).expect_err("no claude target");
        assert_eq!(
            refused.to_string(),
            "error [ls] --live needs the claude target"
        );
        assert!(require_claude(&[]).is_err());
    }
}
