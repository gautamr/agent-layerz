//! `CLAUDE.md` (spec §7.1, §8.1, target-facts §3): the import of `AGENTS.md`, and the
//! `## Claude-specific` block with the text of each layer's `targets/claude/CLAUDE.extra.md` in
//! stack order. The header names `AGENTS.md` and each extra as its sources; the core writes it.

use alz_adapter::{Content, HeaderSource, Markdown, Model};

use super::Made;

/// The path of the file.
const CLAUDE_MD: &str = "CLAUDE.md";
/// The generated file that `CLAUDE.md` imports.
const AGENTS_MD: &str = "AGENTS.md";
/// The heading of the Claude-only text.
const CLAUDE_SPECIFIC: &str = "## Claude-specific";
/// The separator of the parts of the body.
const PART_SEPARATOR: &str = "\n\n";

/// `CLAUDE.md`: always written, for it carries the import.
pub(super) fn render(model: &Model) -> Made {
    let specific = (!model.target_instructions.is_empty())
        .then(|| CLAUDE_SPECIFIC.to_owned())
        .into_iter()
        .chain(
            model
                .target_instructions
                .iter()
                .map(|extra| extra.body.clone()),
        );
    let body = std::iter::once(format!("@{AGENTS_MD}"))
        .chain(specific)
        .collect::<Vec<_>>()
        .join(PART_SEPARATOR);
    let sources = std::iter::once(HeaderSource::GeneratedFile(AGENTS_MD.to_owned()))
        .chain(
            model
                .target_instructions
                .iter()
                .map(|extra| HeaderSource::from(extra.source.clone())),
        )
        .collect();
    Made::file(
        CLAUDE_MD,
        Content::Markdown(Markdown::new(body).with_header_sources(sources)),
    )
}
