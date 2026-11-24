//! `alz explain <type> <name>` (spec §10.3): the layers that contributed to an item, general to
//! specific, then the merged output file the build makes for it.
//!
//! The item's lines come from [`alz_core::explain`], which holds only stripped text. The generated
//! file is the build's own text, made from untrusted layer files, so it passes the last guard of
//! [`strip_untrusted_keeping_layout`] before it reaches the terminal.

use std::io::{self, Write};

use alz_core::Diagnostics;
use alz_core::explain::{Explanation, explain};
use alz_core::render::tree::FileTree;

use crate::cli::ExplainArgs;
use crate::command::Surroundings;
use crate::command::pipeline;
use crate::report::emit_diagnostics;
use crate::terminal_text::strip_untrusted_keeping_layout;

/// Runs `alz explain`, writes the diagnostics to `stderr` and the explanation to `stdout`, and
/// returns the exit code.
pub fn run(
    args: &ExplainArgs,
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
    match explain(&computed.merged, args.item_type.into(), &args.name) {
        Err(missing) => {
            emit_diagnostics(&Diagnostics::new().with(missing.into_diagnostic()), stderr)
        }
        Ok(explanation) => write_text(stdout, &text_of(&explanation, computed.rendered.tree()))
            .map_or(1, |()| code),
    }
}

/// The explanation, then each output file the build makes, as `merged result (<path>):` and the
/// file's text.
fn text_of(explanation: &Explanation, tree: &FileTree) -> String {
    let files = explanation
        .outputs()
        .iter()
        .filter_map(|path| tree.get(path).map(|file| (path, file)))
        .map(|(path, file)| {
            format!(
                "\nmerged result ({path}):\n{}",
                strip_untrusted_keeping_layout(&String::from_utf8_lossy(&file.bytes))
            )
        });
    std::iter::once(explanation.to_string())
        .chain(files)
        .collect()
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
